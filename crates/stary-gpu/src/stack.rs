use std::{collections::VecDeque, sync::Arc};

use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;
use stary_register::Transform;
use stary_stack::{Kernel, Matching};

use crate::{context::GpuContext, error::GpuError};

const COLORS: usize = 3;
const GROUP: u32 = 16;
const SAMPLE_BYTES: u64 = 16;
const FRAME_SAMPLE_BYTES: u64 = 2;
const PICTURE_SAMPLE_BYTES: u64 = 4;
const READBACK_BYTES: u64 = 128 << 20;
/// Frames that are worked on at the same time: the one being laid and the one after it.
const FRAMES_IN_FLIGHT: usize = 2;
const UNMOVED: u32 = 1;
const CLIPPED: u32 = 2;

/// The picture the frames are laid on, and the layers of it that are kept apart: the ground that
/// is stacked without moving the frames is a second layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackLayout {
    pub width: usize,
    pub height: usize,
    pub layers: usize,
    pub kernel: Kernel,
}

/// How a frame lies on the picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    to_source: Option<Transform>,
    matching: [Matching; COLORS],
}

impl Placement {
    /// The frame moved onto the picture: `to_source` maps coordinates of the picture onto the
    /// frame, and `matching` brings the sky level and noise of each color to those of the reference.
    pub const fn moved(to_source: Transform, matching: [Matching; COLORS]) -> Self {
        Self {
            to_source: Some(to_source),
            matching,
        }
    }

    /// The frame as it is, sample for sample.
    pub const fn unmoved() -> Self {
        Self {
            to_source: None,
            matching: [Matching::NONE; COLORS],
        }
    }
}

/// Which of the two lays of a frame to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pass {
    /// Counts the frames over each sample and finds their mean and spread.
    Statistics,
    /// Adds up the values that are within `low` and `high` spreads of the mean.
    Clipped { low: f32, high: f32 },
}

/// What the GPU holds for a picture: the state of every sample of every layer, one frame, and
/// what the combined picture needs to be handed back.
pub fn stack_bytes(width: usize, height: usize, layers: usize) -> u64 {
    let samples = (width * height * COLORS) as u64;
    let handed_back = samples * PICTURE_SAMPLE_BYTES;
    layers as u64 * samples * SAMPLE_BYTES
        + samples * FRAME_SAMPLE_BYTES
        + handed_back
        + handed_back.min(READBACK_BYTES)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    rows: [[f32; 4]; 3],
    gain: [f32; 4],
    offset: [f32; 4],
    clipping: [f32; 4],
    width: u32,
    height: u32,
    plane_words: u32,
    flags: u32,
}

struct Layer {
    accumulators: [wgpu::Buffer; COLORS],
    lay_group: wgpu::BindGroup,
}

struct Readback {
    picture: wgpu::Buffer,
    staging: wgpu::Buffer,
}

/// Work still on the GPU: the last submission of each frame that was laid.
#[derive(Default)]
struct InFlight {
    frames: VecDeque<Option<wgpu::SubmissionIndex>>,
}

/// Frames laid on a picture on the GPU, one at a time, in the two lays of a sigma-clipped mean
/// (or the one lay of a plain mean). Moving, matching and adding a frame is one piece of work for
/// the GPU per frame and layer.
pub struct StackGpu {
    context: Arc<GpuContext>,
    layout: StackLayout,
    plane_words: usize,
    statistics: wgpu::ComputePipeline,
    clipped: wgpu::ComputePipeline,
    finish: wgpu::ComputePipeline,
    finish_layout: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
    frame: wgpu::Buffer,
    layers: Vec<Layer>,
    readback: Option<Readback>,
    in_flight: InFlight,
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bind(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn storage_buffer(device: &wgpu::Device, bytes: u64, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

const fn kernel_constants(kernel: Kernel) -> [(&'static str, f64); 2] {
    match kernel {
        Kernel::Bilinear => [("KERNEL", 0.0), ("TAPS", 2.0)],
        Kernel::Bicubic => [("KERNEL", 1.0), ("TAPS", 4.0)],
        Kernel::Lanczos3 => [("KERNEL", 2.0), ("TAPS", 6.0)],
    }
}

fn pipeline(
    context: &GpuContext,
    module: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    entry_point: &str,
    kernel: Kernel,
) -> wgpu::ComputePipeline {
    let pipeline_layout = context
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(entry_point),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
    context
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry_point),
            layout: Some(&pipeline_layout),
            module,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &kernel_constants(kernel),
                ..Default::default()
            },
            cache: None,
        })
}

fn send_plane(
    queue: &wgpu::Queue,
    frame: &wgpu::Buffer,
    index: usize,
    plane_words: usize,
    plane: &[u16],
) -> Result<(), GpuError> {
    let bytes: &[u8] = bytemuck::cast_slice(plane);
    let size = wgpu::BufferSize::new((plane_words * 4) as u64).expect("a plane has samples");
    let mut view = queue
        .write_buffer_with(frame, (index * plane_words * 4) as u64, size)
        .ok_or_else(|| GpuError::Failed("the frame could not be sent".into()))?;
    view.slice(..bytes.len()).copy_from_slice(bytes);
    view.slice(bytes.len()..).fill(0);
    Ok(())
}

impl StackGpu {
    pub fn new(context: Arc<GpuContext>, layout: StackLayout) -> Result<Self, GpuError> {
        let pixels = layout.width * layout.height;
        let plane_words = pixels.div_ceil(2);
        let accumulator_bytes = pixels as u64 * SAMPLE_BYTES;
        let frame_bytes = (COLORS * plane_words * 4) as u64;
        let allowed = context.max_binding_bytes();
        let needed = accumulator_bytes
            .max(frame_bytes)
            .max(pixels as u64 * COLORS as u64 * PICTURE_SAMPLE_BYTES);
        if needed > allowed {
            return Err(GpuError::TooLarge { needed, allowed });
        }
        context.allocate(|| Self::build(context.clone(), layout, plane_words))
    }

    fn build(context: Arc<GpuContext>, layout: StackLayout, plane_words: usize) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stack"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stack.wgsl").into()),
        });
        let lay_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lay"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, false),
                storage_entry(2, false),
                storage_entry(3, false),
                uniform_entry(4),
            ],
        });
        let finish_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("finish"),
            entries: &[
                storage_entry(1, false),
                storage_entry(2, false),
                storage_entry(3, false),
                uniform_entry(4),
                storage_entry(5, false),
            ],
        });
        let statistics = pipeline(&context, &module, &lay_layout, "statistics", layout.kernel);
        let clipped = pipeline(&context, &module, &lay_layout, "clipped", layout.kernel);
        let finish = pipeline(&context, &module, &finish_layout, "finish", layout.kernel);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame = storage_buffer(device, (COLORS * plane_words * 4) as u64, "frame");
        let accumulator_bytes = (layout.width * layout.height) as u64 * SAMPLE_BYTES;
        let layers = (0..layout.layers)
            .map(|_| {
                let accumulators = std::array::from_fn(|_| {
                    storage_buffer(device, accumulator_bytes, "accumulator")
                });
                let lay_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("lay"),
                    layout: &lay_layout,
                    entries: &[
                        bind(0, &frame),
                        bind(1, &accumulators[0]),
                        bind(2, &accumulators[1]),
                        bind(3, &accumulators[2]),
                        bind(4, &params),
                    ],
                });
                Layer {
                    accumulators,
                    lay_group,
                }
            })
            .collect();
        Self {
            context: Arc::clone(&context),
            layout,
            plane_words,
            statistics,
            clipped,
            finish,
            finish_layout,
            params,
            frame,
            layers,
            readback: None,
            in_flight: InFlight::default(),
        }
    }

    fn wait_for(&self, index: wgpu::SubmissionIndex) -> Result<(), GpuError> {
        self.context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: None,
            })
            .map(|_| ())
            .map_err(|error| GpuError::Failed(error.to_string()))
    }

    /// Sends a frame (three planes of `width * height` samples) to the GPU, where the next lays
    /// use it. At most two frames are in the works: this waits for the one before the last.
    pub fn upload(&mut self, planes: [&[u16]; COLORS]) -> Result<(), GpuError> {
        self.context.check()?;
        let pixels = self.layout.width * self.layout.height;
        if planes.iter().any(|plane| plane.len() != pixels) {
            return Err(GpuError::Failed(
                "a frame does not have the size of the picture".into(),
            ));
        }
        while self.in_flight.frames.len() >= FRAMES_IN_FLIGHT {
            if let Some(Some(index)) = self.in_flight.frames.pop_front() {
                self.wait_for(index)?;
            }
        }
        self.in_flight.frames.push_back(None);
        let (queue, frame, plane_words) = (&self.context.queue, &self.frame, self.plane_words);
        planes
            .par_iter()
            .enumerate()
            .map(|(index, plane)| send_plane(queue, frame, index, plane_words, plane))
            .collect()
    }

    fn params_for(&self, flags: u32, clipping: [f32; 2], placement: &Placement) -> Params {
        let rows = placement
            .to_source
            .unwrap_or(Transform::IDENTITY)
            .m
            .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32, 0.0]);
        let matching = placement.matching;
        Params {
            rows,
            gain: [matching[0].gain, matching[1].gain, matching[2].gain, 0.0],
            offset: [
                matching[0].offset,
                matching[1].offset,
                matching[2].offset,
                0.0,
            ],
            clipping: [clipping[0], clipping[1], 0.0, 0.0],
            width: self.layout.width as u32,
            height: self.layout.height as u32,
            plane_words: self.plane_words as u32,
            flags: flags
                | if placement.to_source.is_none() {
                    UNMOVED
                } else {
                    0
                },
        }
    }

    fn dispatch(
        &self,
        pipeline: &wgpu::ComputePipeline,
        group: &wgpu::BindGroup,
        params: &Params,
    ) -> wgpu::SubmissionIndex {
        self.context
            .queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(params));
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(
                (self.layout.width as u32).div_ceil(GROUP),
                (self.layout.height as u32).div_ceil(GROUP),
                1,
            );
        }
        self.context.queue.submit([encoder.finish()])
    }

    /// Lays the frame that was last uploaded on a layer of the picture.
    pub fn lay(&mut self, layer: usize, pass: Pass, placement: &Placement) -> Result<(), GpuError> {
        self.context.check()?;
        let (pipeline, clipping) = match pass {
            Pass::Statistics => (&self.statistics, [0.0, 0.0]),
            Pass::Clipped { low, high } => (&self.clipped, [low, high]),
        };
        let group = &self
            .layers
            .get(layer)
            .ok_or_else(|| GpuError::Failed("there is no such layer".into()))?
            .lay_group;
        let params = self.params_for(0, clipping, placement);
        let index = self.dispatch(pipeline, group, &params);
        if let Some(last) = self.in_flight.frames.back_mut() {
            *last = Some(index);
        }
        Ok(())
    }

    fn readback(&mut self) -> Result<(wgpu::Buffer, wgpu::Buffer), GpuError> {
        if self.readback.is_none() {
            let bytes =
                (self.layout.width * self.layout.height * COLORS) as u64 * PICTURE_SAMPLE_BYTES;
            let device = &self.context.device;
            self.readback = Some(self.context.allocate(|| Readback {
                picture: storage_buffer(device, bytes, "picture"),
                staging: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("readback"),
                    size: bytes.min(READBACK_BYTES),
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            })?);
        }
        self.readback
            .as_ref()
            .map(|readback| (readback.picture.clone(), readback.staging.clone()))
            .ok_or_else(|| GpuError::Failed("the readback is missing".into()))
    }

    /// The combined value of every sample of a layer, the colors of a pixel next to each other and
    /// scaled so that 1.0 is full scale. `clipped` says whether the second lay was done.
    pub fn finish_layer(&mut self, layer: usize, clipped: bool) -> Result<Vec<f32>, GpuError> {
        self.context.check()?;
        let (picture, staging) = self.readback()?;
        let accumulators = &self
            .layers
            .get(layer)
            .ok_or_else(|| GpuError::Failed("there is no such layer".into()))?
            .accumulators;
        let group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("finish"),
                layout: &self.finish_layout,
                entries: &[
                    bind(1, &accumulators[0]),
                    bind(2, &accumulators[1]),
                    bind(3, &accumulators[2]),
                    bind(4, &self.params),
                    bind(5, &picture),
                ],
            });
        let params = self.params_for(
            if clipped { CLIPPED } else { 0 },
            [0.0, 0.0],
            &Placement::unmoved(),
        );
        let index = self.dispatch(&self.finish, &group, &params);
        self.wait_for(index)?;
        self.in_flight.frames.clear();
        self.collect(&picture, &staging)
    }

    fn collect(
        &self,
        picture: &wgpu::Buffer,
        staging: &wgpu::Buffer,
    ) -> Result<Vec<f32>, GpuError> {
        let total = (self.layout.width * self.layout.height * COLORS) as u64 * PICTURE_SAMPLE_BYTES;
        let mut samples = Vec::with_capacity((total / PICTURE_SAMPLE_BYTES) as usize);
        let mut offset = 0;
        while offset < total {
            let bytes = (total - offset).min(READBACK_BYTES);
            let mut encoder = self
                .context
                .device
                .create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(picture, offset, staging, 0, bytes);
            self.context.queue.submit([encoder.finish()]);
            staging
                .slice(..bytes)
                .map_async(wgpu::MapMode::Read, |_| {});
            self.context
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|error| GpuError::Failed(error.to_string()))?;
            let mapped = staging
                .slice(..bytes)
                .get_mapped_range()
                .map_err(|error| GpuError::Failed(error.to_string()))?;
            samples.extend_from_slice(bytemuck::cast_slice::<u8, f32>(&mapped));
            drop(mapped);
            staging.unmap();
            offset += bytes;
        }
        self.context.check()?;
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_of_26_megapixels_with_two_layers_needs_about_three_gigabytes() {
        let bytes = stack_bytes(6246, 4170, 2);
        assert!((3_000_000_000..3_400_000_000).contains(&bytes), "{bytes}");
    }

    #[test]
    fn a_second_layer_costs_what_the_state_of_the_samples_takes() {
        let one = stack_bytes(1000, 1000, 1);
        let two = stack_bytes(1000, 1000, 2);
        assert_eq!(two - one, 1000 * 1000 * 3 * SAMPLE_BYTES);
    }
}
