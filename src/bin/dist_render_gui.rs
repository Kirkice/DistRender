use std::time::Instant;

use egui_wgpu::Renderer as EguiWgpuRenderer;
use egui_winit::State as EguiWinitState;
use shared_memory::{Shmem, ShmemConf};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use dist_render::core::{Config, SceneConfig};
use dist_render::gui::ipc::{DEFAULT_SHM_NAME, GuiStatePacket, SharedGuiState};
use dist_render::gui::panels;
use dist_render::gui::GuiState;

fn main() {
    let mut config = Config::from_file_or_default("config.toml");
    config.apply_args(std::env::args());
    let scene = SceneConfig::from_file_or_default("scene.toml");

    let packet0 = GuiStatePacket {
        clear_color: scene.clear_color,
        light_intensity: scene.light.intensity,
        light_direction: scene.light.transform.rotation,
        model_position: scene.model.transform.position,
        model_rotation: scene.model.transform.rotation,
        model_scale: scene.model.transform.scale,
        camera_fov: scene.camera.fov,
        camera_near: scene.camera.near_clip,
        camera_far: scene.camera.far_clip,
    };

    let shmem = create_or_open_shmem(DEFAULT_SHM_NAME, packet0);
    let shared = unsafe { &*(shmem.as_ptr() as *const SharedGuiState) };

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("DistRender GUI")
        .with_inner_size(winit::dpi::LogicalSize::new(360.0, 720.0))
        .build(&event_loop)
        .expect("Failed to create GUI window");

    // wgpu init (blocking)
    let mut gfx = pollster::block_on(WgpuGui::new(&window));

    let egui_ctx = egui::Context::default();
    let viewport_id = egui_ctx.viewport_id();
    let mut egui_state = EguiWinitState::new(egui_ctx.clone(), viewport_id, &window, None, None);

    let mut egui_renderer = EguiWgpuRenderer::new(&gfx.device, gfx.config.format, None, 1);

    let mut gui_state = GuiState::new(&config, &scene);

    let mut last_frame = Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                if matches!(event, WindowEvent::CloseRequested) {
                    elwt.exit();
                    return;
                }

                if matches!(event, WindowEvent::Resized(_)) {
                    gfx.resize(&window);
                }

                let _ = egui_state.on_window_event(&window, &event);

                if matches!(event, WindowEvent::RedrawRequested) {
                    let now = Instant::now();
                    let _dt = now.duration_since(last_frame).as_secs_f32();
                    last_frame = now;

                    let raw_input = egui_state.take_egui_input(&window);
                    egui_ctx.begin_frame(raw_input);

                    egui::SidePanel::left("control_panel")
                        .default_width(330.0)
                        .show(&egui_ctx, |ui| {
                            ui.heading("DistRender 控制面板");
                            ui.separator();

                            panels::performance::render(ui, &gui_state);
                            ui.separator();

                            panels::rendering::render(ui, &mut gui_state);
                            ui.separator();

                            panels::scene::render(ui, &mut gui_state);
                            ui.separator();

                            panels::backend::render(ui, &mut gui_state);
                        });

                    let full_output = egui_ctx.end_frame();
                    egui_state.handle_platform_output(&window, full_output.platform_output);

                    // write shared memory
                    let packet = GuiStatePacket {
                        clear_color: gui_state.clear_color,
                        light_intensity: gui_state.light_intensity,
                        light_direction: gui_state.light_direction,
                        model_position: gui_state.model_position,
                        model_rotation: gui_state.model_rotation,
                        model_scale: gui_state.model_scale,
                        camera_fov: gui_state.camera_fov,
                        camera_near: gui_state.camera_near,
                        camera_far: gui_state.camera_far,
                    };
                    shared.write_latest(packet);

                    // render egui with wgpu
                    if let Err(e) = gfx.render_egui(
                        &window,
                        &egui_ctx,
                        &mut egui_state,
                        &mut egui_renderer,
                        full_output,
                    ) {
                        eprintln!("GUI render error: {e}");
                    }

                    return;
                }

                window.request_redraw();
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn create_or_open_shmem(name: &str, init_packet: GuiStatePacket) -> Shmem {
    let size = SharedGuiState::MAGIC_SIZE;

    if let Ok(shmem) = ShmemConf::new().os_id(name).size(size).create() {
        unsafe {
            let ptr = shmem.as_ptr() as *mut SharedGuiState;
            ptr.write(SharedGuiState::new_init(init_packet));
        }
        return shmem;
    }

    ShmemConf::new()
        .os_id(name)
        .open()
        .expect("Failed to open shared memory")
}

struct WgpuGui {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuGui {
    async fn new(window: &winit::window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let window = std::sync::Arc::new(window);
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create wgpu surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request wgpu adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GUI Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to request wgpu device");

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb))
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
        }
    }

    fn resize(&mut self, window: &winit::window::Window) {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render_egui(
        &mut self,
        window: &winit::window::Window,
        egui_ctx: &egui::Context,
        _egui_state: &mut EguiWinitState,
        egui_renderer: &mut EguiWgpuRenderer,
        full_output: egui::FullOutput,
    ) -> Result<(), String> {
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to acquire frame: {e}"))?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GUI Encoder"),
            });

        let pixels_per_point = window.scale_factor() as f32;
        let paint_jobs = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GUI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            egui_renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
