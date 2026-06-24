use crate::Renderer;
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
        let key = std::any::TypeId::of::<Self>();
        if !rend.bind_group_layouts.map.contains_key(&key) {
            rend.bind_group_layouts.map.insert(
                key,
                rend.ctx
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some(Self::name()),
                        entries: &Self::bind_group_entries(),
                    }),
            );
        }
    }
    fn get_layout(rend: &'a Renderer) -> &'a wgpu::BindGroupLayout {
        &rend.bind_group_layouts.map[&std::any::TypeId::of::<Self>()]
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
    pub fn buffers(&'a self) -> &'a [wgpu::Buffer] {
        &self.buffers
    }
    pub fn new<T: DataLayout<'a>>(initial_data: T, rend: &'a mut Renderer) -> Self {
        let mut buffers = Vec::new();
        for i in 0..T::bind_group_entries().len() {
            let buffer = rend
                .ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&(T::name().to_owned() + " buffer " + &i.to_string())),
                    contents: &initial_data.get_data_for_entry(i as u32),
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
