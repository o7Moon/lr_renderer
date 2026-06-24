#[derive(Default)]
pub(crate) struct BindGroupLayoutCache {
    pub(crate) map: std::collections::HashMap<std::any::TypeId, wgpu::BindGroupLayout>,
}
