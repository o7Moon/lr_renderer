use wgpu::{RenderPass, util::DeviceExt};

use crate::{CameraMatrixUniform, Renderer};
use std::collections::HashMap;

pub struct LineRenderer;

pub(crate) const LINERENDERER_PIPELINE_KEY: &str = "LineRenderer";

impl LineRenderer {
    pub(crate) fn init(
        rend: &Renderer,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader = rend
            .ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Line Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line.wgsl").into()),
            });

        let layout = rend
            .ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Renderer Layout"),
                bind_group_layouts: &[Some(camera_layout), Some(LineLayerBuffer::get_layout(rend))],
                immediate_size: 0,
            });

        rend.ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Line Renderer"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }

    pub fn draw(
        rend: &Renderer,
        buf: &mut LineLayerBuffer,
        pass: &mut RenderPass,
        camera: &CameraMatrixUniform,
    ) {
        if buf.writing_view.is_some() {
            buf.upload_staging(rend);
        }
        pass.set_bind_group(0, Some(&camera.0.bind_group), &[]);
        pass.set_bind_group(1, Some(&buf.bind_group), &[]);
        pass.set_pipeline(&rend.pipelines[&LINERENDERER_PIPELINE_KEY.to_owned()]);
        //println!("lines: {}", buf.local_linebuf.len() as u32);
        pass.draw(0..6, 0..buf.local_linebuf.len() as u32);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct GpuLine {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
}

pub trait ToGpuLine {
    fn positions(&self) -> [f32; 4];
    fn width(&self) -> f32;

    fn to_gpu_line(&self) -> GpuLine {
        let pos = self.positions();
        let width = self.width();
        GpuLine {
            x1: pos[0],
            y1: pos[1],
            x2: pos[2],
            y2: pos[3],
            width,
        }
    }
}

const DEFAULT_LINE_COUNT: u64 = 1024; // smallish because we have one of these per layer

//const DEFAULT_LINE_COUNT: u64 = 300000; // less small because im prototyping and havent written the
// buffer resizing code yet lmao

type LineId = u32;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LayerColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<wgpu::Color> for LayerColor {
    fn from(value: wgpu::Color) -> Self {
        Self {
            r: value.r as f32,
            g: value.g as f32,
            b: value.b as f32,
            a: value.a as f32,
        }
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl LayerColor {
    pub fn from_start(str: &str, fallback: LayerColor) -> LayerColor {
        if str.starts_with("#") && str.len() >= 7 {
            let colorstring: &str = &str[1..7];
            let Ok(r) = u8::from_str_radix(&colorstring[0..2], 16) else {
                return fallback;
            };
            let Ok(g) = u8::from_str_radix(&colorstring[2..4], 16) else {
                return fallback;
            };
            let Ok(b) = u8::from_str_radix(&colorstring[4..6], 16) else {
                return fallback;
            };
            let a: u8 = 255;

            return LayerColor {
                r: srgb_to_linear(r as f32 / 255.0),
                g: srgb_to_linear(g as f32 / 255.0),
                b: srgb_to_linear(b as f32 / 255.0),
                a: a as f32 / 255.0,
            };
        }

        fallback
    }
}

pub struct LineLayerBuffer {
    local_linebuf: Vec<GpuLine>,
    local_index_to_line_id: Vec<u32>,
    local_line_id_to_index: HashMap<LineId, usize>,
    line_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) writing_view: Option<wgpu::QueueWriteBufferView>, // implicitly created whenever
                                                                 // making line changes, and submitted before drawing
}

const LINELAYERLAYOUT_KEY: &str = "LineLayer";

impl LineLayerBuffer {
    pub fn ensure_layout(rend: &mut Renderer) {
        rend.bind_group_layouts.insert(
            LINELAYERLAYOUT_KEY.to_owned(),
            rend.ctx
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Uniform<T> BindGroupLayout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            // Line Data
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            // Layer Color
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                }),
        );
    }

    pub fn get_layout<'a>(rend: &'a Renderer) -> &'a wgpu::BindGroupLayout {
        &rend.bind_group_layouts[&LINELAYERLAYOUT_KEY.to_owned()]
    }

    pub fn new(rend: &Renderer, color: LayerColor) -> Self {
        let lines: Vec<GpuLine> = Vec::with_capacity(DEFAULT_LINE_COUNT as usize);
        let ctx = &rend.ctx;
        let linebuf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Layer Line Buffer"),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            size: DEFAULT_LINE_COUNT * std::mem::size_of::<GpuLine>() as u64,
            mapped_at_creation: false,
        });
        let colorbuf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Line Layer Color Buffer"),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                contents: bytemuck::cast_slice(&[color]),
            });
        let bindgroup = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Line Layer BindGroup"),
            layout: Self::get_layout(rend),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &linebuf,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &colorbuf,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            local_linebuf: lines,
            bind_group: bindgroup,
            local_index_to_line_id: Vec::new(),
            local_line_id_to_index: HashMap::new(),
            line_buffer: linebuf,
            color_buffer: colorbuf,
            writing_view: None,
        }
    }

    fn begin_writing(&mut self, rend: &Renderer) {
        if self.writing_view.is_some() {
            return;
        }

        self.writing_view = rend.ctx.queue.write_buffer_with(
            &self.line_buffer,
            0,
            std::num::NonZero::new(self.line_buffer.size()).unwrap(),
        );
    }

    fn write_staging(&mut self, offset: u64, data: &[u8]) {
        let Some(staging) = self.writing_view.as_mut() else {
            return;
        };
        staging
            .slice(offset as usize..(offset as usize + data.len()))
            .copy_from_slice(data);
    }

    pub(crate) fn upload_staging(&mut self, rend: &Renderer) {
        if let Some(staging) = self.writing_view.take() {
            drop(staging);
            rend.ctx.queue.submit([]);
        }
    }

    pub fn put_line(&mut self, line: GpuLine, at_id: LineId, rend: &Renderer) {
        if self.local_line_id_to_index.contains_key(&at_id) {
            self.local_linebuf[self.local_line_id_to_index[&at_id]] = line;
        } else {
            self.local_linebuf.push(line);
            self.local_line_id_to_index
                .insert(at_id, self.local_linebuf.len() - 1);
            self.local_index_to_line_id.push(at_id);
        }
        self.begin_writing(rend);
        self.write_staging(
            (std::mem::size_of::<GpuLine>() * self.local_line_id_to_index[&at_id]) as u64,
            bytemuck::cast_slice(&[line]),
        );
    }

    pub fn remove_line(&mut self, _at_id: LineId) {
        todo!()
    }
}
