use std::collections::HashMap;

use json::{array, object};
use lr_renderer::{
    Camera, CameraMatrixUniform, LayerColor, LineLayerBuffer, LineRenderer, Renderer, ToGpuLine,
    Uniform, wgpu,
};
use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, Size},
    event::{Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

#[derive(Default)]
struct App<'a> {
    window: Option<std::sync::Arc<Window>>,
    renderer: Option<Renderer<'a>>,
    layers: Vec<LineLayerBuffer>,
    layer_id_to_index: HashMap<u32, u32>,
    //lines: Option<LineLayerBuffer>,
    camera: Option<CameraMatrixUniform>,
    camera_values: Camera,
    mousedown: bool,
    last_pos: PhysicalPosition<f64>,
    aspect_ratio: f32,
}

struct SimLine {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl ToGpuLine for SimLine {
    fn positions(&self) -> [f32; 4] {
        [
            self.x1 as f32,
            self.y1 as f32,
            self.x2 as f32,
            self.y2 as f32,
        ]
    }
    fn width(&self) -> f32 {
        1.0
    }
}

pub struct JsonLine<'a>(&'a json::JsonValue);

impl<'a> ToGpuLine for JsonLine<'a> {
    fn positions(&self) -> [f32; 4] {
        [
            self.0["x1"].as_f32().unwrap(),
            self.0["y1"].as_f32().unwrap(),
            self.0["x2"].as_f32().unwrap(),
            self.0["y2"].as_f32().unwrap(),
        ]
    }

    fn width(&self) -> f32 {
        self.0["width"].as_f32().unwrap_or(1.0)
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(std::sync::Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(Size::Logical(LogicalSize::new(640.0, 360.0)))
                        .with_title("lr_renderer test app"),
                )
                .unwrap(),
        ));
        self.renderer = Renderer::new(
            self.window.clone(),
            Some(Box::new(event_loop.owned_display_handle())),
            Some((640, 360)),
        )
        .block_on()
        .ok();
        if let Some(renderer) = &mut self.renderer {
            /*lines.put_line(
                SimLine {
                    x1: -10.,
                    y1: 10.,
                    x2: 10.,
                    y2: -10.,
                }
                .to_gpu_line(),
                1,
                renderer,
            );*/

            println!("uhhh");
            let track = json::parse(include_str!("street address.track.json")).unwrap();
            for layer in track["layers"].members() {
                if !layer["visible"].as_bool().unwrap() {
                    continue;
                }
                let lines = LineLayerBuffer::new(
                    renderer,
                    LayerColor::from_start(
                        layer["name"].as_str().unwrap(),
                        LayerColor {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    ),
                );
                let index = self.layers.len();
                self.layers.push(lines);
                self.layer_id_to_index
                    .insert(layer["id"].as_u32().unwrap(), index as u32);
            }
            for line in track["lines"].members() {
                if !self
                    .layer_id_to_index
                    .contains_key(&line["layer"].as_u32().unwrap_or(0))
                {
                    continue;
                }
                let layer_idx: u32 = self
                    .layer_id_to_index
                    .get(&line["layer"].as_u32().unwrap_or(0))
                    .unwrap()
                    .to_owned();
                let lines = &mut self.layers[layer_idx as usize];
                let id: u32 = line["id"].as_u32().unwrap();
                lines.put_line(JsonLine(line).to_gpu_line(), id, renderer);
            }
            /*for (n, l) in track["lines"].members().enumerate() {
                lines.put_line(
                    SimLine {
                        x1: l["x1"].as_f64().unwrap(),
                        x2: l["x2"].as_f64().unwrap(),
                        y1: l["y1"].as_f64().unwrap(),
                        y2: l["y2"].as_f64().unwrap(),
                    }
                    .to_gpu_line(),
                    n as u32,
                    renderer,
                );
                println!("{}", n);
            }
            self.layer_id_to_index.insert(k, v)
            self.layers.push(lines);
            //self.lines = Some(lines);
            println!("done");*/

            self.camera_values = Camera::new(0., 15., 50.);
            let cam_buf: CameraMatrixUniform =
                CameraMatrixUniform::new(self.camera_values, 9. / 16., renderer);
            self.camera = Some(cam_buf);
            self.mousedown = false;
            self.aspect_ratio = 9. / 16.;
        }
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    let drawlines = |renderer: &mut Renderer, pass: &mut wgpu::RenderPass| {
                        for lines in &mut self.layers {
                            LineRenderer::draw(
                                renderer,
                                lines,
                                pass,
                                self.camera.as_ref().unwrap(),
                            );
                        }
                    };
                    renderer
                        .render(
                            lr_renderer::wgpu::Color {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 1.0,
                            },
                            Some(drawlines),
                        )
                        .unwrap();
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(h, v) => {
                        self.camera_values.zoom_height -= v as f64 * 17.;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.camera_values.zoom_height -= pos.y;
                    }
                }
                if let Some(camera) = &mut self.camera {
                    camera.update(
                        self.camera_values,
                        self.aspect_ratio,
                        &self.renderer.as_ref().unwrap().ctx,
                    );
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if button != MouseButton::Left {
                    return;
                }

                self.mousedown = state.is_pressed();
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                if self.mousedown {
                    let mut delta: (f64, f64) =
                        (position.x - self.last_pos.x, position.y - self.last_pos.y);
                    delta.0 /= 360.;
                    delta.0 *= self.camera_values.zoom_height;
                    delta.1 /= 360.;
                    delta.1 *= self.camera_values.zoom_height;
                    self.camera_values.center_x -= delta.0;
                    self.camera_values.center_y += delta.1;
                    /*println!(
                        "{}, {}",
                        self.camera_values.center_x, self.camera_values.center_y
                    );*/
                    if let Some(camera) = &mut self.camera {
                        camera.update(
                            self.camera_values,
                            self.aspect_ratio,
                            &self.renderer.as_ref().unwrap().ctx,
                        );
                    }
                }

                self.last_pos = position;
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize((size.width, size.height));
                    let values = self.camera_values;
                    let camera = self.camera.as_mut().unwrap();
                    self.aspect_ratio = size.height as f32 / size.width as f32;
                    camera.update(values, self.aspect_ratio, &renderer.ctx);
                }
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::default();
    _ = event_loop.run_app(&mut app);
}
