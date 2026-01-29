use std::time::Instant;

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

    let shm_name = DEFAULT_SHM_NAME;

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

    let shmem = create_or_open_shmem(shm_name, packet0);
    let shared = unsafe { &*(shmem.as_ptr() as *const SharedGuiState) };

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("DistRender GUI")
        .with_inner_size(winit::dpi::LogicalSize::new(360.0, 720.0))
        .build(&event_loop)
        .expect("Failed to create GUI window");

    let egui_ctx = egui::Context::default();
    let viewport_id = egui_ctx.viewport_id();
    let mut egui_state = EguiWinitState::new(egui_ctx.clone(), viewport_id, &window, None, None);

    let mut gui_state = GuiState::new(&config, &scene);

    let mut last_frame = Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                        return;
                    }
                    WindowEvent::RedrawRequested => {
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

                        return;
                    }
                    _ => {}
                }

                let _ = egui_state.on_window_event(&window, &event);
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
