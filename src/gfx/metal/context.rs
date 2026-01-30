//! Metal 鍥惧舰鍚庣瀹炵幇

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

/// Metal 鍥惧舰鍚庣
pub struct MetalContext {
    window: Arc<Window>,
    pub device: Device,
    pub command_queue: CommandQueue,
    pub layer: MetalLayer,
}

impl GraphicsBackend for MetalContext {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        info!("姝ｅ湪鍒濆鍖?Metal 鍚庣...");

        let window_builder = WindowBuilder::new()
            .with_title(&config.window.title)
            .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
            .with_resizable(config.window.resizable);

        let window = Arc::new(window_builder.build(event_loop).expect("鏃犳硶鍒涘缓绐楀彛"));

        // 鑾峰彇绯荤粺榛樿 Metal 璁惧
        let device = Device::system_default().expect("鏃犳硶鎵惧埌 Metal 璁惧");
        info!("Metal 璁惧: {}", device.name());

        let command_queue = device.new_command_queue();

        // 鍒涘缓骞堕厤缃?CAMetalLayer
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);
        
        // Enable triple buffering for better performance
        // This prevents next_drawable() from blocking when drawing faster than display refresh
        layer.set_maximum_drawable_count(3);

        // 灏?Layer 缁戝畾鍒扮獥鍙?
        // 杩欓噷闇€瑕佷娇鐢?raw-window-handle 鏉ヨ幏鍙栧簳灞?NSView
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::AppKit(handle) = handle.as_raw() {
                unsafe {
                    use cocoa::appkit::NSView;
                    let view = handle.ns_view.as_ptr() as cocoa::base::id;
                    view.setWantsLayer(YES);
                    view.setLayer(layer.as_ref() as *const _ as _);
                }
            } else {
                error!("涓嶆敮鎸佺殑绐楀彛鍙ユ焺绫诲瀷: Metal 浠呮敮鎸?macOS AppKit");
            }
        }

        // 鏇存柊 layer 澶у皬
        let size = window.inner_size();
        layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
        
        info!("Metal 鍚庣鍒濆鍖栧畬鎴?);

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
