use std::{
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use stary_pipeline::{Cancel, JobSpec, Progress, RunOptions, Stage, plan_run, run_job};
use tauri::{AppHandle, Emitter};

use crate::payload::job_events::{FINISHED_EVENT, FinishedEvent, PROGRESS_EVENT, ProgressEvent};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(60);

/// Only one job runs at a time; this holds the handle that can stop it.
#[derive(Default)]
pub struct JobRegistry {
    running: Mutex<Option<Cancel>>,
}

impl JobRegistry {
    pub fn begin(&self) -> Option<Cancel> {
        let mut running = self.running.lock().ok()?;
        if running.is_some() {
            return None;
        }
        let cancel = Cancel::default();
        *running = Some(cancel.clone());
        Some(cancel)
    }

    pub fn finish(&self) {
        if let Ok(mut running) = self.running.lock() {
            *running = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().is_ok_and(|running| running.is_some())
    }

    pub fn cancel(&self) {
        if let Some(cancel) = self.running.lock().ok().and_then(|running| running.clone()) {
            cancel.request();
        }
    }
}

struct Throttle {
    last: Instant,
    stage: Option<Stage>,
}

fn emit_progress(app: &AppHandle, throttle: &Mutex<Throttle>, progress: Progress) {
    let Ok(mut state) = throttle.lock() else {
        return;
    };
    let finished_step = progress.done >= progress.total;
    let stage_changed = state.stage != Some(progress.stage);
    if finished_step || stage_changed || state.last.elapsed() >= PROGRESS_INTERVAL {
        state.last = Instant::now();
        state.stage = Some(progress.stage);
        let _ = app.emit(PROGRESS_EVENT, ProgressEvent::from(progress));
    }
}

pub fn spawn_job(app: AppHandle, spec: JobSpec, cancel: Cancel) {
    thread::spawn(move || {
        let started = Instant::now();
        let plan = plan_run(
            spec.lights.first().map(|path| path.as_path()),
            spec.memory_percent,
        );
        let options = RunOptions {
            workers: plan.workers,
            ..RunOptions::default()
        };
        let throttle = Mutex::new(Throttle {
            last: Instant::now(),
            stage: None,
        });
        let report = |progress: Progress| emit_progress(&app, &throttle, progress);
        let result = run_job(&spec, &options, &cancel, &report);
        let event = FinishedEvent::from_result(result, started.elapsed().as_secs_f64());
        if let Some(registry) = tauri::Manager::try_state::<JobRegistry>(&app) {
            registry.finish();
        }
        let _ = app.emit(FINISHED_EVENT, event);
    });
}
