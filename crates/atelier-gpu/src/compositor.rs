//! GPU tile compositor (spec 0004). Executes the op list from
//! `atelier_raster::ops` per 256² tile with compute shaders; must match the
//! CPU reference within 1 LSB after quantization (D-9). This slice exists for
//! parity validation; wiring it to the canvas is a later perf slice.

use atelier_core::{Document, TILE_SIZE};
use atelier_raster::ops::{build_op_list, CompositeOp};
use atelier_raster::quantize_rgba8;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const TILE: usize = TILE_SIZE;
const PX_PER_TILE: usize = TILE * TILE;
const F32BUF_BYTES: u64 = (PX_PER_TILE * 16) as u64; // vec4f per pixel

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    mode: u32,
    opacity: f32,
    tile_origin: [i32; 2],
}

pub struct GpuCompositor {
    pipeline_tile: wgpu::ComputePipeline,
    pipeline_buffer: wgpu::ComputePipeline,
    bgl_tile: wgpu::BindGroupLayout,
    bgl_buffer: wgpu::BindGroupLayout,
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

impl GpuCompositor {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/composite.wgsl"));

        let bgl_tile = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite tile bgl"),
            entries: &[storage_entry(0, false), storage_entry(1, true), uniform_entry(2)],
        });
        let bgl_buffer = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite buffer bgl"),
            entries: &[storage_entry(0, false), uniform_entry(2), storage_entry(3, true)],
        });

        let make = |label: &str, bgl: &wgpu::BindGroupLayout, entry: &str| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[bgl],
                push_constant_ranges: &[],
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        Self {
            pipeline_tile: make("composite cs_tile", &bgl_tile, "cs_tile"),
            pipeline_buffer: make("composite cs_buffer", &bgl_buffer, "cs_buffer"),
            bgl_tile,
            bgl_buffer,
        }
    }

    /// Flatten `doc` to straight-alpha RGBA8 — same contract and quantization
    /// as `atelier_raster::composite_rgba8`.
    pub fn composite_rgba8(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        doc: &Document,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let ops = build_op_list(doc);
        let mut out = vec![0u8; (width * height * 4) as usize];

        let tiles_x = (width as usize).div_ceil(TILE);
        let tiles_y = (height as usize).div_ceil(TILE);

        let new_f32buf = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: F32BUF_BYTES,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let params_buf = |mode: u32, opacity: f32, origin: [i32; 2]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("composite params"),
                contents: bytemuck::bytes_of(&Params { mode, opacity, tile_origin: origin }),
                usage: wgpu::BufferUsages::UNIFORM,
            })
        };

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite staging"),
            size: F32BUF_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let origin = [(tx * TILE) as i32, (ty * TILE) as i32];
                let mut encoder = device.create_command_encoder(&Default::default());

                // Stack of isolated buffers; index 0 is the backdrop.
                let mut stack = vec![new_f32buf("stack0")];
                // Zero-initialize reused semantics: fresh buffers are zeroed by wgpu.

                for op in &ops {
                    match op {
                        CompositeOp::Layer { tiles, mode, opacity } => {
                            let Some(tile) = tiles.tile_at((tx as i32, ty as i32)) else {
                                continue; // absent tile = fully transparent source
                            };
                            let src = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("src tile"),
                                contents: tile.bytes(),
                                usage: wgpu::BufferUsages::STORAGE,
                            });
                            let params = params_buf(mode_index(*mode), *opacity, origin);
                            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("cs_tile bg"),
                                layout: &self.bgl_tile,
                                entries: &[
                                    bind(0, stack.last().expect("stack non-empty")),
                                    bind(1, &src),
                                    bind(2, &params),
                                ],
                            });
                            dispatch(&mut encoder, &self.pipeline_tile, &bg);
                        }
                        CompositeOp::Push => stack.push(new_f32buf("isolated")),
                        CompositeOp::Pop { mode, opacity } => {
                            let src = stack.pop().expect("balanced push/pop");
                            let params = params_buf(mode_index(*mode), *opacity, origin);
                            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("cs_buffer bg"),
                                layout: &self.bgl_buffer,
                                entries: &[
                                    bind(0, stack.last().expect("backdrop below")),
                                    bind(2, &params),
                                    bind(3, &src),
                                ],
                            });
                            dispatch(&mut encoder, &self.pipeline_buffer, &bg);
                        }
                    }
                }

                encoder.copy_buffer_to_buffer(&stack[0], 0, &staging, 0, F32BUF_BYTES);
                queue.submit([encoder.finish()]);

                // Read back and quantize exactly like the CPU path.
                let slice = staging.slice(..);
                let (tx_done, rx_done) = std::sync::mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |r| {
                    tx_done.send(r).expect("receiver alive");
                });
                device.poll(wgpu::Maintain::Wait);
                rx_done.recv().expect("map callback ran").expect("map ok");
                {
                    let data = slice.get_mapped_range();
                    let px: &[f32] = bytemuck::cast_slice(&data);
                    let x_end = (width as usize).min((tx + 1) * TILE) - tx * TILE;
                    let y_end = (height as usize).min((ty + 1) * TILE) - ty * TILE;
                    for y in 0..y_end {
                        for x in 0..x_end {
                            let i = (y * TILE + x) * 4;
                            let dx = tx * TILE + x;
                            let dy = ty * TILE + y;
                            let o = (dy * width as usize + dx) * 4;
                            for c in 0..4 {
                                out[o + c] = quantize_rgba8(px[i + c]);
                            }
                        }
                    }
                }
                staging.unmap();
            }
        }
        out
    }
}

fn bind(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: buffer.as_entire_binding() }
}

fn dispatch(encoder: &mut wgpu::CommandEncoder, pipeline: &wgpu::ComputePipeline, bg: &wgpu::BindGroup) {
    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bg, &[]);
    pass.dispatch_workgroups((TILE / 16) as u32, (TILE / 16) as u32, 1);
}

fn mode_index(mode: atelier_core::BlendMode) -> u32 {
    atelier_core::BlendMode::ALL
        .iter()
        .position(|&m| m == mode)
        .expect("mode in ALL") as u32
}
