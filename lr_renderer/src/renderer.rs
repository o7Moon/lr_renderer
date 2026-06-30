use std::sync::Mutex;
use wgpu::wgt::WgpuHasDisplayHandle;

use crate::Context;

pub struct Renderer<'a> {
    pub ctx: Context<'a>,
    pub(crate) sconf: Option<wgpu::SurfaceConfiguration>,
    pub(crate) pipelines: Mutex<crate::pipelines::RenderPipelineCache>,
    pub(crate) bind_group_layouts: Mutex<crate::layout::BindGroupLayoutCache>,
}

#[derive(Debug)]
pub enum RenderError {
    NoSurface,
}

impl<'a> Renderer<'a> {
    pub async fn new(
        window_handle: Option<impl Into<wgpu::SurfaceTarget<'a>>>,
        display_handle: Option<Box<dyn WgpuHasDisplayHandle>>,
        width_height: Option<(u32, u32)>,
    ) -> Result<Self, crate::context::ContextNewError> {
        let ctx = Context::new(window_handle, display_handle).await?;
        let sconf = if let Some(surface) = &ctx.surface {
            let caps = surface.get_capabilities(&ctx.adapter);
            let format = caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(caps.formats[0]);
            let size = width_height.unwrap_or((640, 360));
            let conf = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.0,
                height: size.1,
                present_mode: caps.present_modes[0],
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&ctx.device, &conf);
            Some(conf)
        } else {
            None
        };
        let rend = Self {
            ctx,
            sconf,
            pipelines: Default::default(),
            bind_group_layouts: Default::default(),
        };

        Ok(rend)
    }

    // assume request_redraw happened before this is called
    pub fn render(
        &self,
        color: wgpu::Color,
        draw: Option<impl FnOnce(&Renderer, &mut wgpu::RenderPass)>,
    ) -> Result<(), RenderError> {
        let surface = match &self.ctx.surface {
            Some(s) => s,
            None => return Err(RenderError::NoSurface),
        };

        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => return Err(RenderError::NoSurface),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });
        {
            let mut _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            if let Some(draw) = draw {
                draw(self, &mut _pass);
            }
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn resize(&mut self, width_height: (u32, u32)) {
        if let Some(sconf) = self.sconf.as_mut() {
            sconf.width = width_height.0;
            sconf.height = width_height.1;
            self.ctx
                .surface
                .as_ref()
                .unwrap()
                .configure(&self.ctx.device, sconf);
        }
    }
}
