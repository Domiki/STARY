use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError, sync_channel},
    },
    thread,
    time::Duration,
};

use stary_core::Frame;

use crate::{control::Cancel, error::PipelineError};

pub type DecodeFn = fn(&Path) -> Result<Frame, PipelineError>;

pub fn decode_file(path: &Path) -> Result<Frame, PipelineError> {
    Ok(stary_io::decode_frame(path)?)
}

type Handed<T> = (usize, Result<T, PipelineError>);
type Hand<T> = SyncSender<Handed<T>>;

const POLL: Duration = Duration::from_millis(20);

/// Waits for the next frame; gives up when the workers are gone or the run was cancelled.
fn wait_for_item<T>(receiver: &Receiver<Handed<T>>, cancel: &Cancel) -> Option<Handed<T>> {
    loop {
        match receiver.recv_timeout(POLL) {
            Ok(item) => return Some(item),
            Err(RecvTimeoutError::Disconnected) => return None,
            Err(RecvTimeoutError::Timeout) if cancel.is_requested() => return None,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

/// Runs `work(index)` for every index on worker threads and hands each result to `consume` on the
/// calling thread.
///
/// Workers are added only while `consume` is waiting for results, up to `max_workers`, so memory
/// is not spent on workers the consumer cannot keep up with. At most `max_workers` results wait
/// in the hand-over queue.
pub fn for_each_processed<T, W, F>(
    count: usize,
    max_workers: usize,
    work: W,
    cancel: &Cancel,
    mut consume: F,
) -> Result<(), PipelineError>
where
    T: Send,
    W: Fn(usize) -> Result<T, PipelineError> + Sync,
    F: FnMut(usize, T) -> Result<(), PipelineError>,
{
    let max_workers = max_workers.max(1);
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (sender, receiver) = sync_channel::<Handed<T>>(max_workers);
    thread::scope(|scope| {
        let mut hand: Option<Hand<T>> = Some(sender);
        let mut spawned = 0usize;
        let work = &work;
        let spawn_worker = |hand: &Hand<T>| {
            let (hand, next, stop) = (hand.clone(), &next, &stop);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= count || cancel.is_requested() || stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if hand.send((index, work(index))).is_err() {
                        break;
                    }
                }
            });
        };
        if let Some(first) = &hand {
            spawn_worker(first);
            spawned += 1;
        }
        let mut outcome = Ok(());
        loop {
            let received = match receiver.try_recv() {
                Ok(item) => Some(item),
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {
                    let work_left = next.load(Ordering::Relaxed) < count;
                    if let Some(active) =
                        hand.as_ref().filter(|_| work_left && spawned < max_workers)
                    {
                        spawn_worker(active);
                        spawned += 1;
                    }
                    if !work_left || spawned >= max_workers {
                        hand = None;
                    }
                    wait_for_item(&receiver, cancel)
                }
            };
            let Some((index, produced)) = received else {
                if cancel.is_requested() {
                    outcome = Err(PipelineError::Cancelled);
                }
                break;
            };
            if cancel.is_requested() {
                outcome = Err(PipelineError::Cancelled);
                break;
            }
            outcome = produced.and_then(|item| consume(index, item));
            if outcome.is_err() {
                stop.store(true, Ordering::Relaxed);
                break;
            }
        }
        drop(receiver);
        outcome
    })
}

/// Decodes files on worker threads and hands each frame to `consume` on the calling thread.
pub fn for_each_decoded<F>(
    paths: &[PathBuf],
    max_workers: usize,
    decode: DecodeFn,
    cancel: &Cancel,
    consume: F,
) -> Result<(), PipelineError>
where
    F: FnMut(usize, Frame) -> Result<(), PipelineError>,
{
    for_each_processed(
        paths.len(),
        max_workers,
        |index| decode(&paths[index]),
        cancel,
        consume,
    )
}
