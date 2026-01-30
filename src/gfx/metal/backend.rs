//! Metal 图形后端实现

use std::sync::Arc;
use tracing::{info, error};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::dpi::LogicalSize;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use metal::{Device, CommandQueue, MetalLayer, MTLPixelFormat};
use objc::runtime::{YES};
use core_graphics_types::geometry::CGSize;

use crate::gfx::backend::GraphicsBackend;
use crate::core::Config;

/// Metal 图形后端
pub struct MetalBackend {
    window: Arc<Window>,
    pub device: Device,
    pub command_queue: CommandQueue,
    pub layer: MetalLayer,
}

impl GraphicsBackend for MetalBackend {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        info!("正在初始化 Metal 后端...");

        let window_builder = WindowBuilder::new()
            .with_title(&config.window.title)
            .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
            .with_resizable(config.window.resizable);

        let window = Arc::new(window_builder.build(event_loop).expect("无法创建窗口"));

        // 获取系统默认 Metal 设备
        let device = Device::system_default().expect("无法找到 Metal 设备");
        info!("Metal 设备: {}", device.name());

        let command_queue = device.new_command_queue();

        // 创建并配置 CAMetalLayer
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);
        
        // Enable triple buffering for better performance
        // This prevents next_drawable() from blocking when drawing faster than display refresh
        layer.set_maximum_drawable_count(3);

        // 将 Layer 绑定到窗口
        // 这里需要使用 raw-window-handle 来获取底层 NSView
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::AppKit(handle) = handle.as_raw() {
                unsafe {
                    use cocoa::appkit::NSView;
                    let view = handle.ns_view.as_ptr() as cocoa::base::id;
                    view.setWantsLayer(YES);
                    view.setLayer(layer.as_ref() as *const _ as _);
                }
            } else {
                error!("不支持的窗口句柄类型: Metal 仅支持 macOS AppKit");
            }
        }

        // 更新 layer 大小
        let size = window.inner_size();
        layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
        
        info!("Metal 后端初始化完成");

        Self {
            window,
            device,
            command_queue,
            layer,
        }
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn backend_name(&self) -> &str {
        "Metal"
    }
}
