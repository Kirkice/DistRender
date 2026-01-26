//! 图形后端的统一抽象接口
//!
//! 本模块定义了所有图形后端（Vulkan、DirectX 12 等）必须实现的统一接口。
//! 这样可以在不同的图形 API 之间无缝切换，而不需要修改上层渲染逻辑。

use winit::window::Window;
use winit::event_loop::EventLoop;
use crate::core::Config;

/// 图形后端的统一接口
///
/// 所有具体的图形后端（如 Vulkan、DirectX 12）都必须实现此 trait，
/// 以提供设备初始化、资源管理等基础功能。
///
/// # 设计理念
///
/// - **抽象化**：隐藏不同图形 API 的实现细节
/// - **统一接口**：提供一致的调用方式
/// - **可扩展性**：方便添加新的图形后端
pub trait GraphicsBackend {
    /// 创建图形后端实例
    ///
    /// 根据提供的事件循环和配置参数初始化图形后端。
    ///
    /// # 参数
    ///
    /// * `event_loop` - winit 事件循环引用，用于创建窗口
    /// * `config` - 引擎配置，包含窗口大小、图形后端参数等
    ///
    /// # 返回值
    ///
    /// 初始化完成的图形后端实例
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self
    where
        Self: Sized;

    /// 获取窗口的引用
    ///
    /// 返回与此图形后端关联的窗口引用，用于获取窗口尺寸、处理窗口事件等。
    ///
    /// # 返回值
    ///
    /// 窗口的不可变引用
    fn window(&self) -> &Window;

    /// 获取后端的名称
    ///
    /// 返回当前使用的图形后端名称，用于日志输出和调试。
    ///
    /// # 返回值
    ///
    /// 后端名称的字符串切片（如 "Vulkan"、"DirectX 12"）
    fn backend_name(&self) -> &str;
}
