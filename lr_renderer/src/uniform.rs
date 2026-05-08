use crate::{Renderer, context::Context};
use std::marker::PhantomData;
use wgpu::util::DeviceExt;

pub struct Uniform<T: bytemuck::Pod + bytemuck::Zeroable> {
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    _phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> Uniform<T> {
    pub fn ensure_layout(rend: &mut Renderer, visibility: wgpu::ShaderStages) {
        let mut key: String = "U<T> with ".to_owned();
        key += &visibility.bits().to_string();
        if !rend.bind_group_layouts.contains_key(&key) {
            rend.bind_group_layouts.insert(
                key.clone(),
                rend.ctx
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Uniform<T> BindGroupLayout"),
                        entries: &[wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                    }),
            );
        }
    }

    pub fn get_layout<'a>(
        rend: &'a Renderer,
        visibility: wgpu::ShaderStages,
    ) -> &'a wgpu::BindGroupLayout {
        let mut key: String = "U<T> with ".to_owned();
        key += &visibility.bits().to_string();
        &rend.bind_group_layouts[&key]
    }

    pub fn update(&self, value: T, ctx: &Context) {
        ctx.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[value]));
    }
    pub fn new(initial_data: T, rend: &mut Renderer, visibility: wgpu::ShaderStages) -> Self {
        let buffer = rend
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform<T> buffer"),
                contents: bytemuck::cast_slice(&[initial_data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        Self::ensure_layout(rend, visibility);
        let bind_group_layout = Self::get_layout(rend, visibility);
        let bind_group = rend
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Uniform<T> BindGroup"),
                layout: bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
        Self {
            buffer,
            bind_group,
            _phantom: PhantomData,
        }
    }
}
