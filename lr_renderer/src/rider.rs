use crate::{DataLayout, pipelines::Pipeline};

pub struct Sprite<'a> {
    texture: &'a wgpu::BindGroup,
    info: SpriteShaderData,
    _string: bool, // if true, then draw a line all the way across the bone instead of in the
                   // direction of it. just uses left as a "radius" and ignores the rest
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct SpriteShaderData {
    // distances from the origin/pivot on axes relative to the bone vector
    left: f32,
    right: f32,
    along: f32,
    back: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct ShaderData {
    originx: f32,
    originy: f32,
    endpointx: f32,
    endpointy: f32,
    sprite: SpriteShaderData,
}

pub struct SpriteBuffer {
    buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SpriteBuffer {
    pub fn new(rend: &crate::Renderer) -> Self {
        let buf = rend.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size_of::<ShaderData>() as u64,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::UNIFORM,
        });
        Self {
            buf: buf.clone(),
            bind_group: rend
                .ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &Self::layout(rend),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf.as_entire_binding(),
                    }],
                }),
        }
    }

    fn layout(rend: &crate::Renderer) -> wgpu::BindGroupLayout {
        rend.bind_group_layouts
            .lock()
            .unwrap()
            .map
            .entry(std::any::TypeId::of::<Self>())
            .or_insert_with(|| {
                rend.ctx
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Sprite Buffer"),
                        entries: &[wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                    })
            })
            .clone()
    }
}

pub struct SpriteRenderer;

#[derive(Clone, Hash, PartialEq, Eq, Default)]
pub struct SpriteRendererVariant {
    is_string: bool,
}

impl crate::pipelines::Pipeline for SpriteRenderer {
    type Variant = SpriteRendererVariant;

    fn compile(renderer: &crate::Renderer, _v: &Self::Variant) -> wgpu::RenderPipeline {
        let shader = renderer
            .ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Sprite Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
            });
        let layout = renderer
            .ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sprite Renderer Layout"),
                bind_group_layouts: &[
                    Some(&crate::camera::CameraMatrix::get_layout(renderer)),
                    Some(&SpriteBuffer::layout(renderer)),
                    Some(&crate::texture::Texture::layout_single(renderer)),
                ],
                immediate_size: 0,
            });
        renderer
            .ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sprite Renderer"),
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
                        format: if let Some(sconf) = &renderer.sconf {
                            sconf.format
                        } else {
                            wgpu::TextureFormat::Bgra8UnormSrgb
                        },
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}

impl SpriteRenderer {
    pub fn draw(
        rend: &crate::Renderer,
        buf: &SpriteBuffer,
        sprite: Sprite,
        positions: [f32; 4],
        pass: &mut wgpu::RenderPass,
        camera: &crate::CameraMatrixUniform,
    ) {
        rend.ctx.queue.write_buffer(
            &buf.buf,
            0,
            bytemuck::cast_slice(&[ShaderData {
                originx: positions[0],
                originy: positions[1],
                endpointx: positions[2],
                endpointy: positions[3],
                sprite: sprite.info,
            }]),
        );

        pass.set_bind_group(0, Some(&camera.0.bind_group), &[]);
        pass.set_bind_group(1, Some(&buf.bind_group), &[]);
        pass.set_bind_group(2, Some(sprite.texture), &[]);
        pass.set_pipeline(&SpriteRenderer::get(rend, Default::default()));
        pass.draw(0..6, 0..1);
    }
}
