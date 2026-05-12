use wgpu::{Adapter, Device, Queue, Surface, wgt::WgpuHasDisplayHandle};

pub struct Context<'a> {
    pub adapter: Adapter,
    pub surface: Option<Surface<'a>>,
    pub device: Device,
    pub queue: Queue,
}

pub enum ContextNewError {
    NoAdapter(wgpu::RequestAdapterError),
    NoDevice(wgpu::RequestDeviceError),
}

impl From<wgpu::RequestAdapterError> for ContextNewError {
    fn from(value: wgpu::RequestAdapterError) -> Self {
        Self::NoAdapter(value)
    }
}

impl From<wgpu::RequestDeviceError> for ContextNewError {
    fn from(value: wgpu::RequestDeviceError) -> Self {
        Self::NoDevice(value)
    }
}

impl<'a> Context<'a> {
    pub async fn new(
        window_handle: Option<impl Into<wgpu::SurfaceTarget<'a>>>,
        display_handle: Option<Box<dyn WgpuHasDisplayHandle>>,
    ) -> Result<Self, ContextNewError> {
        let instance = wgpu::Instance::new(if let Some(display) = display_handle {
            wgpu::InstanceDescriptor::new_with_display_handle(display)
        } else {
            wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = if let Some(handle) = window_handle {
            let surface_result = instance.create_surface(handle);
            surface_result.ok()
        } else {
            None
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                label: None,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        Ok(Self {
            adapter,
            surface,
            device,
            queue,
        })
    }
}
