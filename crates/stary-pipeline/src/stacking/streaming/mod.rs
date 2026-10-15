mod cpu;
mod gpu;

use stary_core::{COLORS, LinearRgbF32};
use stary_mask::Mask;
use stary_stack::Kernel;

use self::{cpu::CpuLayers, gpu::GpuLayers};
use super::{
    device::Backend,
    integrate::Member,
    prepare::{PrepareContext, demosaiced_planes},
};
use crate::{
    control::{Cancel, Progress, Stage},
    decode_stream::for_each_processed,
    error::PipelineError,
    masking::{Rows, blend_rows, mix_f32},
    planes::Planes,
};

/// How a stack is combined without keeping its frames: every frame is made, moved and laid on the
/// picture one at a time. With rejection the frames are laid twice, first to find the mean and the
/// spread of every sample, then to add up what is within `low` and `high` spreads of the mean.
#[derive(Debug, Clone, Copy)]
pub struct Clipping {
    pub low: f32,
    pub high: f32,
}

pub struct StreamSetup<'a> {
    pub width: usize,
    pub height: usize,
    pub kernel: Kernel,
    /// `None` is a plain mean.
    pub clipping: Option<Clipping>,
    pub workers: usize,
    pub context: &'a PrepareContext<'a>,
    /// When set, the frames are also combined without being moved, and that picture is put under
    /// the result where this mask says ground.
    pub ground: Option<&'a Mask>,
}

#[derive(Clone, Copy)]
enum Lay {
    Statistics,
    Clipped(Clipping),
}

/// The combined value of every sample, interleaved as colors of pixels and scaled to 1.0.
struct Combined {
    sky: Vec<f32>,
    ground: Option<Vec<f32>>,
}

/// Where the picture is kept and what lays the frames on it.
trait Layers {
    fn lay(&mut self, member: &Member, planes: &Planes, lay: Lay) -> Result<(), PipelineError>;
    fn combined(self, clipped: bool) -> Result<Combined, PipelineError>;
}

/// Goes through the members: those whose planes are kept first, then the others, which workers make
/// again.
fn for_each_member(
    members: &[Member],
    setup: &StreamSetup,
    cancel: &Cancel,
    mut each: impl FnMut(&Member, &Planes) -> Result<(), PipelineError>,
) -> Result<(), PipelineError> {
    for member in members {
        if let Some(planes) = &member.planes {
            each(member, planes)?;
        }
    }
    let missing: Vec<usize> = (0..members.len())
        .filter(|&position| members[position].planes.is_none())
        .collect();
    for_each_processed(
        missing.len(),
        setup.workers,
        |slot| demosaiced_planes(setup.context, members[missing[slot]].index, false),
        cancel,
        |slot, planes| each(&members[missing[slot]], &planes),
    )
}

fn lay_all(
    mut layers: impl Layers,
    members: &[Member],
    setup: &StreamSetup,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Combined, PipelineError> {
    let passes = if setup.clipping.is_some() { 2 } else { 1 };
    let total = members.len() * passes;
    let mut done = 0;
    let mut pass = |lay: Lay| {
        for_each_member(members, setup, cancel, |member, planes| {
            layers.lay(member, planes, lay)?;
            done += 1;
            progress(Progress {
                stage: Stage::Stack,
                done,
                total,
            });
            Ok(())
        })?;
        if cancel.is_requested() {
            return Err(PipelineError::Cancelled);
        }
        Ok(())
    };
    pass(Lay::Statistics)?;
    if let Some(clipping) = setup.clipping {
        pass(Lay::Clipped(clipping))?;
    }
    layers.combined(setup.clipping.is_some())
}

/// Combines the aligned frames one at a time: memory does not grow with the number of frames.
pub fn integrate_streaming(
    members: &[Member],
    setup: &StreamSetup,
    backend: Backend,
    cancel: &Cancel,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<LinearRgbF32, PipelineError> {
    let (width, height) = (setup.width, setup.height);
    let combined = match backend {
        Backend::Cpu => lay_all(CpuLayers::new(setup), members, setup, cancel, progress)?,
        Backend::Gpu { stack, .. } => lay_all(
            GpuLayers::new(stack, setup.ground.is_some()),
            members,
            setup,
            cancel,
            progress,
        )?,
    };
    let mut samples = combined.sky;
    if let (Some(under), Some(mask)) = (combined.ground, setup.ground) {
        let placed = Rows {
            width,
            height,
            channels: COLORS,
            first: 0,
        };
        blend_rows(&mut samples, &under, placed, mask, mix_f32);
    }
    Ok(LinearRgbF32 {
        width,
        height,
        samples,
    })
}
