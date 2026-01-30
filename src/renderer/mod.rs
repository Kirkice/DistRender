//! 渲染器模块
//!
//! 本模块提供了统一的渲染接口，封装了不同图形 API 的具体实现。
//! 应用程序通过这个模块与底层图形 API（Vulkan、DirectX 12）交互，
//! 而不需要关心具体使用的是哪个图形 API。
//!
//! # 架构设计
//!
//! - `Renderer`：统一的渲染器接口，对外提供一致的 API
//! - `Backend`：内部枚举，封装不同的图形后端实现
//! - 底层实现在 `gfx` 模块中，按 API 分类组织

use tracing::info;
use winit::event_loop::EventLoop;

use crate::core::error::Result;
use crate::core::Config;
#[cfg(target_os = "windows")]
use crate::gfx::dx12::Renderer as Dx12Renderer;
use crate::gfx::vulkan::Renderer as VulkanRenderer;
use crate::gfx::wgpu::Renderer as WgpuRenderer;
#[cfg(target_os = "macos")]
use crate::gfx::metal::Renderer as MetalRenderer;
use crate::gui::ipc::GuiStatePacket;

// 通用渲染器组件（与具体 API 无关）
pub mod vertex;
pub mod resource;
pub mod sync;
pub mod command;
pub mod descriptor;

/// 图形后端枚举
///
/// 封装不同的图形 API 实现，支持运行时选择使用哪个后端。
/// 通过枚举模式实现零成本抽象，避免动态分发的性能开销。
enum Backend {
    Vulkan(VulkanRenderer),
    #[cfg(target_os = "windows")]
    Dx12(Dx12Renderer),
    Wgpu(WgpuRenderer),
    #[cfg(target_os = "macos")]
    Metal(MetalRenderer),
}

pub struct Renderer {
    backend: Backend,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &crate::core::SceneConfig) -> Result<Self> {
        use crate::core::config::GraphicsBackend as GfxBackend;
        
        let backend = match config.graphics.backend {
            GfxBackend::Wgpu => {
                info!("Initializing wgpu Backend");
                let renderer = WgpuRenderer::new(event_loop, config, scene)?;
                Backend::Wgpu(renderer)
            }
            #[cfg(target_os = "windows")]
            GfxBackend::Dx12 => {
                info!("Initializing DX12 Backend");
                let renderer = Dx12Renderer::new(event_loop, config, scene)?;
                Backend::Dx12(renderer)
            }
            #[cfg(not(target_os = "windows"))]
            GfxBackend::Dx12 => {
                 return Err(crate::core::error::DistRenderError::Initialization("DX12 backend is only available on Windows".to_string()));
            }
            #[cfg(target_os = "macos")]
            GfxBackend::Metal => {
                info!("Initializing Metal Backend");
                let renderer = MetalRenderer::new(event_loop, config, scene)?;
                Backend::Metal(renderer)
            }
            // 如果在非 macOS 系统上选择 Metal，回退到 Vulkan 或报错。这里为了简单，如果有 Metal 变体但平台不支持，编译器可能报错如果 variant 被 cfg 不包含。
            // 但我在 config.rs 里的 GraphicsBackend::Metal 没有加 cfg。
            #[cfg(not(target_os = "macos"))]
            GfxBackend::Metal => {
                return Err(crate::core::error::DistRenderError::Config("Metal backend is only available on macOS".to_string()));
            }
            GfxBackend::Vulkan => {
                info!("Initializing Vulkan Backend");
                let renderer = VulkanRenderer::new(event_loop, config, scene)?;
                Backend::Vulkan(renderer)
            }
        };

        Ok(Self { backend })
    }

    pub fn resize(&mut self) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.resize(),
            #[cfg(target_os = "windows")]
            Backend::Dx12(r) => r.resize(),
            Backend::Wgpu(r) => r.resize(),
            #[cfg(target_os = "macos")]
            Backend::Metal(r) => r.resize(),
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        match &mut self.backend {
            Backend::Vulkan(r) => r.draw(),
            #[cfg(target_os = "windows")]
            Backend::Dx12(r) => r.draw(),
            Backend::Wgpu(r) => r.draw(),
            #[cfg(target_os = "macos")]
            Backend::Metal(r) => r.draw(),
        }
    }

    pub fn update(&mut self, input_system: &mut crate::core::input::InputSystem, delta_time: f32) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.update(input_system, delta_time),
            #[cfg(target_os = "windows")]
            Backend::Dx12(r) => r.update(input_system, delta_time),
            Backend::Wgpu(r) => r.update(input_system, delta_time),
            #[cfg(target_os = "macos")]
            Backend::Metal(r) => r.update(input_system, delta_time),
        }
    }

    pub fn window(&self) -> &winit::window::Window {
        match &self.backend {
            Backend::Vulkan(r) => r.window(),
            #[cfg(target_os = "windows")]
            Backend::Dx12(r) => r.window(),
            Backend::Wgpu(r) => r.window(),
            #[cfg(target_os = "macos")]
            Backend::Metal(r) => r.window(),
        }
    }

    pub fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.apply_gui_packet(packet),
            #[cfg(target_os = "windows")]
            Backend::Dx12(r) => r.apply_gui_packet(packet),
            Backend::Wgpu(r) => r.apply_gui_packet(packet),
            #[cfg(target_os = "macos")]
            Backend::Metal(r) => r.apply_gui_packet(packet),
        }
    }

    pub fn handle_gui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        match &mut self.backend {
            Backend::Wgpu(r) => r.handle_gui_event(event),
            _ => false, // Vulkan 和 DX12 使用外部 GUI，不需要处理事件
        }
    }
}
