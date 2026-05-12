use crate::{Context, Renderer};
use wgpu::util::DeviceExt;

/// specifies how to create the bindgrouplayout for some data to get used by the gpu
pub trait DataLayout<'a>: bytemuck::Pod + bytemuck::NoUninit {
    fn name() -> &'static str;
    fn visibility() -> wgpu::ShaderStages;
    fn bind_group_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: Self::visibility(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }]
    }
    fn ensure_layout(rend: &mut Renderer) {
        let key: String = Self::name().to_owned() + " BindGroup";
        if !rend.bind_group_layouts.contains_key(&key) {
            rend.bind_group_layouts.insert(
                key.clone(),
                rend.ctx
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some(&key),
                        entries: &Self::bind_group_entries(),
                    }),
            );
        }
    }
    fn get_layout(rend: &'a Renderer) -> &'a wgpu::BindGroupLayout {
        let key: String = Self::name().to_owned() + " BindGroup";
        &rend.bind_group_layouts[&key]
    }
    fn get_data_for_entry(self, idx: u32) -> Vec<u8> {
        let _ = idx;
        bytemuck::cast_slice(&[self]).into()
    }
}

pub struct Uniform {
    buffers: Vec<wgpu::Buffer>,
    pub bind_group: wgpu::BindGroup,
}

impl<'a> Uniform {
    pub fn update<T: DataLayout<'a>>(&self, idx: u32, value: T, ctx: &Context) {
        ctx.queue.write_buffer(
            &self.buffers[idx as usize],
            0,
            &value.get_data_for_entry(idx),
        );
    }
    pub fn new<T: DataLayout<'a>>(initial_data: T, rend: &'a mut Renderer) -> Self {
        let mut buffers = Vec::new();
        for i in 0..T::bind_group_entries().len() {
            let buffer = rend
                .ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&(T::name().to_owned() + " buffer " + &i.to_string())),
                    contents: bytemuck::cast_slice(&[initial_data]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
            buffers.push(buffer);
        }

        T::ensure_layout(rend);
        let bind_group_layout = T::get_layout(rend);
        let bind_group = rend
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&(T::name().to_owned() + " BindGroup")),
                layout: bind_group_layout,
                entries: &buffers
                    .iter()
                    .enumerate()
                    .map(|x| wgpu::BindGroupEntry {
                        binding: x.0 as u32,
                        resource: x.1.as_entire_binding(),
                    })
                    .collect::<Vec<wgpu::BindGroupEntry>>(),
            });
        Self {
            buffers,
            bind_group,
        }
    }
}
