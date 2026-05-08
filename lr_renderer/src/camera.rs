use crate::{Context, Renderer, Uniform};

#[derive(PartialEq, Default, Copy, Clone)]
pub struct Camera {
    pub center_x: f64,
    pub center_y: f64,
    // worldspace height of the camera viewport
    pub zoom_height: f64,
}

impl Camera {
    pub fn new(x: f64, y: f64, height: f64) -> Self {
        Self {
            center_x: x,
            center_y: y,
            zoom_height: height,
        }
    }
    pub fn with_zoom_height(self, zoom_height: f64) -> Self {
        Self {
            center_x: self.center_x,
            center_y: self.center_y,
            zoom_height,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraMatrix([f32; 12]);

struct WorldSpace;
struct RenderSpace;

impl CameraMatrix {
    fn from(camera: &Camera, aspect_ratio: f32) -> Self {
        let transform = euclid::Transform2D::<f32, WorldSpace, RenderSpace>::scale(1.0, -1.0)
            .then_translate(euclid::Vector2D::<f32, RenderSpace> {
                x: -camera.center_x as f32,
                y: -camera.center_y as f32,
                ..Default::default()
            })
            .then_scale(aspect_ratio, 1.0)
            .then_scale(
                2.0 / camera.zoom_height as f32,
                2.0 / camera.zoom_height as f32,
            );
        let mut buffer: [f32; 12] = [0.0; 12];
        buffer[8..].copy_from_slice(&[0.0, 0.0, 1.0, 0.0]);
        buffer[0..2].copy_from_slice(&transform.to_array()[..2]);
        buffer[4..6].copy_from_slice(&transform.to_array()[2..4]);
        buffer[8..10].copy_from_slice(&transform.to_array()[4..6]);
        Self(buffer)
    }
}

//                                                    cached last values, dont reupload to gpu if same
pub struct CameraMatrixUniform(pub Uniform<CameraMatrix>, Option<(Camera, f32)>);

impl CameraMatrixUniform {
    pub fn new(camera: Camera, aspect_ratio: f32, rend: &mut Renderer) -> Self {
        Self(
            Uniform::new(
                CameraMatrix::from(&camera, aspect_ratio),
                rend,
                wgpu::ShaderStages::VERTEX,
            ),
            Some((camera, aspect_ratio)),
        )
    }

    pub fn update(&mut self, camera: Camera, aspect_ratio: f32, ctx: &Context) {
        if let Some((cache_camera, cache_aspect_ratio)) = self.1 {
            if cache_camera != camera || cache_aspect_ratio != aspect_ratio {
                self.internal_update(camera, aspect_ratio, ctx);
                self.1 = Some((camera, aspect_ratio));
            }
        } else {
            self.internal_update(camera, aspect_ratio, ctx);
            self.1 = Some((camera, aspect_ratio));
        }
    }

    fn internal_update(&self, camera: Camera, aspect_ratio: f32, ctx: &Context) {
        self.0
            .update(CameraMatrix::from(&camera, aspect_ratio), ctx);
    }
}
