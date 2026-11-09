use stary_gpu::{Pass, Placement, StackGpu};

use super::{Combined, Lay, Layers};
use crate::{error::PipelineError, planes::Planes, stacking::integrate::Member};

const SKY: usize = 0;
const GROUND: usize = 1;

/// The picture kept on the GPU, which also lays the frames on it.
pub struct GpuLayers {
    stack: Box<StackGpu>,
    ground: bool,
}

impl GpuLayers {
    pub fn new(stack: Box<StackGpu>, ground: bool) -> Self {
        Self { stack, ground }
    }
}

fn pass_of(lay: Lay) -> Pass {
    match lay {
        Lay::Statistics => Pass::Statistics,
        Lay::Clipped(clipping) => Pass::Clipped {
            low: clipping.low,
            high: clipping.high,
        },
    }
}

impl Layers for GpuLayers {
    fn lay(&mut self, member: &Member, planes: &Planes, lay: Lay) -> Result<(), PipelineError> {
        let [red, green, blue] = &planes.data;
        self.stack.upload([red, green, blue])?;
        let pass = pass_of(lay);
        self.stack.lay(
            SKY,
            pass,
            &Placement::moved(member.to_source, member.matching),
        )?;
        if self.ground && member.with_ground {
            self.stack.lay(GROUND, pass, &Placement::unmoved())?;
        }
        Ok(())
    }

    fn combined(mut self, clipped: bool) -> Result<Combined, PipelineError> {
        let sky = self.stack.finish_layer(SKY, clipped)?;
        let ground = if self.ground {
            Some(self.stack.finish_layer(GROUND, clipped)?)
        } else {
            None
        };
        Ok(Combined { sky, ground })
    }
}
