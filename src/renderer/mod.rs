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
//! - 子模块：`vulkan`、`dx12` 分别实现具体的渲染逻辑
//!
//! # 使用示例
//!
//! ```no_run
//! use winit::event_loop::EventLoop;
//! use crate::renderer::Renderer;
//!
//! let event_loop = EventLoop::new();
//! let use_dx12 = false;  // 使用 Vulkan
//! let mut renderer = Renderer::new(&event_loop, use_dx12);
//!
//! // 渲染循环
//! renderer.draw();
//! ```

use tracing::{trace, debug, info, warn, error};
use winit::event_loop::EventLoop;
use crate::renderer::vulkan::Renderer as VulkanRenderer;
use crate::renderer::dx12::Renderer as Dx12Renderer;
use crate::core::Config;

// 子模块声明
pub mod vertex;
pub mod shaders;
pub mod vulkan;
pub mod dx12;

/// 图形后端枚举
///
/// 封装不同的图形 API 实现，支持运行时选择使用哪个后端。
/// 通过枚举模式实现零成本抽象，避免动态分发的性能开销。
enum Backend {
    /// Vulkan 渲染器实例
    Vulkan(VulkanRenderer),
    /// DirectX 12 渲染器实例
    Dx12(Dx12Renderer),
}

/// 统一的渲染器接口
///
/// 提供与图形 API 无关的渲染接口，内部根据用户选择调用相应的后端实现。
/// 这是应用程序与渲染系统交互的主要入口点。
///
/// # 设计原则
///
/// - **抽象性**：隐藏不同图形 API 的实现细节
/// - **一致性**：提供统一的接口，无论使用哪个后端
/// - **性能**：使用枚举而非 trait object，避免动态分发开销
///
/// # 字段说明
///
/// - `backend`：实际的图形后端实现（Vulkan 或 DirectX 12）
pub struct Renderer {
    /// 内部图形后端
    backend: Backend,
}

impl Renderer {
    /// 创建新的渲染器
    ///
    /// 根据配置文件选择合适的图形后端进行初始化。
    ///
    /// # 参数
    ///
    /// * `event_loop` - Winit 事件循环的引用，用于创建窗口和表面
    /// * `config` - 引擎配置，包含图形后端选择、窗口参数等
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `Renderer` 实例
    ///
    /// # Panics
    ///
    /// 如果后端初始化失败（例如驱动不支持、设备不可用等），会 panic
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use winit::event_loop::EventLoop;
    /// use crate::renderer::Renderer;
    /// use crate::core::Config;
    ///
    /// let event_loop = EventLoop::new();
    /// let config = Config::from_file_or_default("config.toml");
    ///
    /// let renderer = Renderer::new(&event_loop, &config);
    /// ```
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        let backend = if config.graphics.backend.is_dx12() {
            info!("Initializing DX12 Backend");
            let renderer = Dx12Renderer::new(event_loop, config);
            Backend::Dx12(renderer)
        } else {
            info!("Initializing Vulkan Backend");
            let renderer = VulkanRenderer::new(event_loop, config);
            Backend::Vulkan(renderer)
        };

        Self { backend }
    }

    /// 处理窗口大小调整
    ///
    /// 当窗口大小改变时调用，通知后端重新创建交换链和相关资源。
    ///
    /// # 说明
    ///
    /// 窗口调整是一个常见操作，需要重新创建与窗口尺寸相关的资源，
    /// 如交换链、帧缓冲、视口等。
    pub fn resize(&mut self) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.resize(),
            Backend::Dx12(r) => r.resize(),
        }
    }

    /// 绘制一帧
    ///
    /// 执行渲染命令，将图形输出到屏幕。
    /// 这是渲染循环的核心方法，应该在每帧被调用一次。
    ///
    /// # 说明
    ///
    /// 此方法会：
    /// 1. 获取下一个可用的交换链图像
    /// 2. 记录渲染命令
    /// 3. 提交命令到 GPU
    /// 4. 呈现渲染结果
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use winit::event_loop::EventLoop;
    /// # use crate::renderer::Renderer;
    /// # let event_loop = EventLoop::new();
    /// # let mut renderer = Renderer::new(&event_loop, false);
    /// // 渲染循环
    /// loop {
    ///     renderer.draw();
    /// }
    /// ```
    pub fn draw(&mut self) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.draw(),
            Backend::Dx12(r) => r.draw(),
        }
    }
}
