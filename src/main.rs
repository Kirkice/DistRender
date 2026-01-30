//! DistRender - 分布式渲染引擎
//!
//! 这是一个支持多图形 API 的渲染引擎，目前支持 Vulkan 和 DirectX 12。
//! 可以通过配置文件或命令行参数选择使用的图形后端。

use dist_render::core::{self, log, Config, SceneConfig};
use dist_render::core::config::GraphicsBackend;
use dist_render::core::input::InputSystem;
use dist_render::renderer::Renderer;
use dist_render::gui::ExternalGui;

use tracing::{debug, error, info};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;

use std::time::Instant;

fn main() {
    let mut config = Config::from_file_or_default("config.toml");
    let args: Vec<String> = std::env::args().collect();
    config.apply_args(args.iter());

    if let Err(e) = config.validate() {
        eprintln!("Invalid configuration: {}", e);
        std::process::exit(1);
    }

    let log_file = if config.logging.file_output {
        Some(config.logging.log_file.as_str())
    } else {
        None
    };
    log::init_logger(config.logging.level, config.logging.file_output, log_file);

    info!("DistRender starting...");
    info!(version = env!("CARGO_PKG_VERSION"), "Application initialized");

    let scene = SceneConfig::from_file_or_default("scene.toml");

    info!(
        backend = ?config.graphics.backend,
        width = config.window.width,
        height = config.window.height,
        "Graphics configuration"
    );

    core::init_renderer_backend(config.graphics.backend);

    info!(
        camera_pos = ?scene.camera.transform.position,
        camera_fov = scene.camera.fov,
        model_path = %scene.model.path,
        "Scene configuration"
    );

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let mut renderer = match Renderer::new(&event_loop, &config, &scene) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to initialize renderer: {}", e);
            eprintln!("Failed to initialize renderer: {}", e);
            std::process::exit(1);
        }
    };

    info!("Renderer initialized successfully");

    let mut input_system = InputSystem::new();

    let no_external_gui = args.iter().any(|a| a == "--no-external-gui");
    let force_external_gui = args.iter().any(|a| a == "--external-gui");

    let default_external_gui = matches!(config.graphics.backend, GraphicsBackend::Vulkan | GraphicsBackend::Dx12 | GraphicsBackend::Metal);
    let use_external_gui = !no_external_gui && (force_external_gui || default_external_gui);

    let external_gui = if use_external_gui && !config.graphics.backend.is_wgpu() {
        ExternalGui::try_start(&config, &scene)
    } else {
        None
    };

    if use_external_gui && external_gui.is_none() {
        warn_external_gui_disabled();
    }

    let mut last_frame = Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("Close requested, shutting down...");
                elwt.exit();
            }
            Event::WindowEvent {
                event: ref window_event,
                ..
            } => {
                // wgpu 后端需要先处理 GUI 事件
                let gui_consumed = if config.graphics.backend.is_wgpu() {
                    renderer.handle_gui_event(window_event)
                } else {
                    false
                };

                // 如果 GUI 没有消费事件，则处理其他事件
                if !gui_consumed {
                    match window_event {
                        WindowEvent::Resized(_) => {
                            renderer.resize();
                        }
                        WindowEvent::KeyboardInput {
                            event: key_event, ..
                        } => {
                            if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key {
                                input_system.on_keyboard_input(keycode, key_event.state);
                            }
                        }
                        WindowEvent::MouseInput { button, state, .. } => {
                            let window = renderer.window();
                            input_system.on_mouse_button(window, *button, *state);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            input_system.on_mouse_move((position.x, position.y));
                        }
                        WindowEvent::Focused(false) => {
                            let window = renderer.window();
                            input_system.unlock_cursor(window);
                            input_system.reset_mouse();
                        }
                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let delta_time = now.duration_since(last_frame).as_secs_f32();
                            last_frame = now;

                            renderer.update(&mut input_system, delta_time);

                            if let Some(gui) = &external_gui {
                                let packet = gui.read_packet();
                                renderer.apply_gui_packet(&packet);
                            }

                            if let Err(e) = renderer.draw() {
                                error!("Draw failed: {}", e);
                                eprintln!("Draw failed: {}", e);
                                elwt.exit();
                            }
                        }
                        _ => (),
                    }
                }
            }
            Event::AboutToWait => {
                renderer.window().request_redraw();
            }
            _ => (),
        }
    });
}

fn warn_external_gui_disabled() {
    tracing::warn!(
        "外部 GUI 未启动（找不到 dist_render_gui 或共享内存创建失败）。你可以：\n- 先运行 `cargo build` 生成 dist_render_gui\n- 或把 dist_render_gui 放到与主程序同目录\n- 或使用 --no-external-gui 禁用外部 GUI"
    );
}
