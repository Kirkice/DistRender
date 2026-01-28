//! DistRender - 分布式渲染引擎
//!
//! 这是一个支持多图形 API 的渲染引擎，目前支持 Vulkan 和 DirectX 12。
//! 可以通过配置文件或命令行参数选择使用的图形后端。
//!
//! # 使用方法
//!
//! ```bash
//! # 使用配置文件
//! cargo run
//!
//! # 使用 DirectX 12（命令行覆盖）
//! cargo run -- --dx12
//! ```
//!
//! # 架构概览
//!
//! ```text
//! ┌─────────────┐
//! │   main.rs   │  应用程序入口
//! └──────┬──────┘
//!        │
//! ┌──────▼──────┐
//! │    Core     │  核心功能模块
//! │  (数学/日志) │
//! └──────┬──────┘
//!        │
//! ┌──────▼──────┐
//! │  Renderer   │  统一渲染接口
//! └──────┬──────┘
//!        │
//!   ┌────┴────┐
//!   │         │
//! ┌─▼──┐   ┌──▼──┐
//! │Vulkan│   │DX12│  具体后端实现
//! └─────┘   └─────┘
//! ```
//!
//! # 模块说明
//!
//! - `core`：核心功能模块（数学库、日志、配置、错误处理）
//! - `renderer`：渲染器模块，提供统一的渲染接口
//! - `gfx`：图形后端模块，封装不同的图形 API

mod core;
mod geometry;
mod renderer;
mod gfx;

use core::{Config, SceneConfig, log};
use tracing::{info, error, debug};
use renderer::Renderer;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

/// 应用程序入口点
///
/// 初始化日志系统、加载配置、选择图形后端、并启动渲染循环。
///
/// # 初始化流程
///
/// 1. 加载引擎配置文件（config.toml）
/// 2. 加载场景配置文件（scene.toml）
/// 3. 应用命令行参数覆盖
/// 4. 初始化日志系统
/// 5. 创建事件循环和渲染器
/// 6. 启动主循环
///
/// # 命令行参数
///
/// - `--dx12`: 使用 DirectX 12 后端（仅 Windows）
/// - `--width <value>`: 设置窗口宽度
/// - `--height <value>`: 设置窗口高度
///
/// # 事件处理
///
/// - `WindowEvent::CloseRequested`：用户关闭窗口，退出程序
/// - `WindowEvent::Resized`：窗口大小改变，通知渲染器重新创建资源
/// - `RedrawEventsCleared`：准备绘制下一帧
fn main() {
    // 1. 加载配置（在初始化日志之前）
    let mut config = Config::from_file_or_default("config.toml");

    // 2. 应用命令行参数
    config.apply_args(std::env::args());

    // 3. 验证配置
    if let Err(e) = config.validate() {
        eprintln!("Invalid configuration: {}", e);
        std::process::exit(1);
    }

    // 4. 初始化日志系统（使用配置中的设置）
    let log_file = if config.logging.file_output {
        Some(config.logging.log_file.as_str())
    } else {
        None
    };
    log::init_logger(
        config.logging.level,
        config.logging.file_output,
        log_file
    );
    info!("DistRender starting...");
    info!(version = env!("CARGO_PKG_VERSION"), "Application initialized");

    // 5. 加载场景配置
    let scene = SceneConfig::from_file_or_default("scene.toml");

    // 6. 输出配置信息
    info!(
        backend = ?config.graphics.backend,
        width = config.window.width,
        height = config.window.height,
        "Graphics configuration"
    );

    info!(
        camera_pos = ?scene.camera.transform.position,
        camera_fov = scene.camera.fov,
        model_path = %scene.model.path,
        "Scene configuration"
    );

    // 7. 创建事件循环
    let event_loop = EventLoop::new();

    // 8. 创建渲染器（传递配置和场景）
    let mut renderer = match Renderer::new(&event_loop, &config, &scene) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to initialize renderer: {}", e);
            eprintln!("Failed to initialize renderer: {}", e);
            std::process::exit(1);
        }
    };

    info!("Scene configuration integrated with renderer successfully");

    info!("Renderer initialized successfully");
    info!("Entering main loop...");

    // 9. 启动事件循环
    event_loop.run(move |event, _, control_flow| {
        match event {
            // 窗口关闭事件
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("Close requested, shutting down...");
                *control_flow = ControlFlow::Exit;
            }
            // 窗口大小调整事件
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                debug!(
                    width = new_size.width,
                    height = new_size.height,
                    "Window resized"
                );
                renderer.resize();
            }
            // 准备绘制下一帧
            Event::RedrawEventsCleared => {
                if let Err(e) = renderer.draw() {
                    error!("Draw failed: {}", e);
                    eprintln!("Draw failed: {}", e);
                    *control_flow = ControlFlow::Exit;
                }
            }
            // 忽略其他事件
            _ => (),
        }
    });
}
