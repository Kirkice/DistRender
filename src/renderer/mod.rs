//! 渲染器模块
//!
//! 本模块提供了统一的渲染接口，封装了不同图形 API 的具体实现。
//! 应用程序通过这个模块与底层图形 API（Vulkan、DirectX 12、Metal、wgpu）交互，
//! 而不需要关心具体使用的是哪个图形 API。
//!
//! # 架构设计
//!
//! - `Renderer`：统一的渲染器接口，对外提供一致的 API
//! - `RenderBackend` trait：定义了所有后端必须实现的接口
//! - 底层实现在 `gfx` 模块中，按 API 分类组织
//!
//! # 重构说明
//!
//! 此模块已从枚举分发模式重构为 trait object 模式：
//! - **优势**：添加新方法只需修改 trait 和实现，无需修改此文件
//! - **性能**：虚函数调用开销可忽略（通常 < 1ns）
//! - **可维护性**：更符合开闭原则，代码更简洁

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
pub mod backend_trait;

// 重新导出 trait
pub use backend_trait::RenderBackend;

/// 渲染器
///
/// 对外提供统一的渲染接口，内部使用 trait object 动态分发到具体的图形后端。
///
/// # 示例
///
/// ```no_run
/// # use dist_render::renderer::Renderer;
/// # use dist_render::core::Config;
/// # use winit::event_loop::EventLoop;
/// # let event_loop = EventLoop::new().unwrap();
/// # let config = Config::default();
/// # let scene = dist_render::core::SceneConfig::default();
/// let mut renderer = Renderer::new(&event_loop, &config, &scene)?;
/// # Ok::<(), dist_render::core::error::DistRenderError>(())
/// ```
pub struct Renderer {
    backend: Box<dyn RenderBackend>,
}

impl Renderer {
    /// 创建新的渲染器实例
    ///
    /// 根据配置中指定的图形后端类型，创建对应的渲染器实现。
    ///
    /// # 参数
    ///
    /// * `event_loop` - winit 事件循环引用
    /// * `config` - 引擎配置
    /// * `scene` - 场景配置
    ///
    /// # 返回值
    ///
    /// 成功时返回渲染器实例，失败时返回错误
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &crate::core::SceneConfig) -> Result<Self> {
        use crate::core::config::GraphicsBackend as GfxBackend;
        
        let backend: Box<dyn RenderBackend> = match config.graphics.backend {
            GfxBackend::Wgpu => {
                info!("Initializing wgpu Backend");
                Box::new(WgpuRenderer::new(event_loop, config, scene)?)
            }
            #[cfg(target_os = "windows")]
            GfxBackend::Dx12 => {
                info!("Initializing DX12 Backend");
                Box::new(Dx12Renderer::new(event_loop, config, scene)?)
            }
            #[cfg(not(target_os = "windows"))]
            GfxBackend::Dx12 => {
                return Err(crate::core::error::DistRenderError::Initialization(
                    "DX12 backend is only available on Windows".to_string()
                ));
            }
            #[cfg(target_os = "macos")]
            GfxBackend::Metal => {
                info!("Initializing Metal Backend");
                Box::new(MetalRenderer::new(event_loop, config, scene)?)
            }
            #[cfg(not(target_os = "macos"))]
            GfxBackend::Metal => {
                return Err(crate::core::error::DistRenderError::Config(
                    crate::core::error::ConfigError::InvalidValue {
                        field: "backend".to_string(),
                        reason: "Metal backend is only available on macOS".to_string(),
                    }
                ));
            }
            GfxBackend::Vulkan => {
                info!("Initializing Vulkan Backend");
                Box::new(VulkanRenderer::new(event_loop, config, scene)?)
            }
        };

        Ok(Self { backend })
    }

    /// 窗口尺寸变化时调用
    ///
    /// 委托给底层图形后端处理交换链重建等操作。
    pub fn resize(&mut self) {
        self.backend.resize()
    }

    /// 渲染一帧
    ///
    /// 执行实际的渲染工作并将结果呈现到屏幕。
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误
    pub fn draw(&mut self) -> Result<()> {
        self.backend.draw()
    }

    /// 更新渲染器状态
    ///
    /// 在每帧渲染前调用，用于处理输入、更新相机等。
    ///
    /// # 参数
    ///
    /// * `input_system` - 输入系统的可变引用
    /// * `delta_time` - 距离上一帧的时间间隔（秒）
    pub fn update(&mut self, input_system: &mut crate::core::input::InputSystem, delta_time: f32) {
        self.backend.update(input_system, delta_time)
    }

    /// 获取窗口引用
    ///
    /// # 返回值
    ///
    /// 窗口的不可变引用
    pub fn window(&self) -> &winit::window::Window {
        self.backend.window()
    }

    /// 应用 GUI 参数包
    ///
    /// 当使用外部 GUI 进程时，通过共享内存传递的参数。
    ///
    /// # 参数
    ///
    /// * `packet` - GUI 状态参数包
    pub fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        self.backend.apply_gui_packet(packet)
    }

    /// 处理 GUI 事件
    ///
    /// 对于内置 GUI 的后端（如 wgpu + egui），需要将窗口事件传递给 GUI。
    ///
    /// # 参数
    ///
    /// * `event` - 窗口事件
    ///
    /// # 返回值
    ///
    /// - `true`: 事件被 GUI 消费，不应继续传播
    /// - `false`: 事件未被 GUI 消费，应继续处理
    pub fn handle_gui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.backend.handle_gui_event(event)
    }
}
