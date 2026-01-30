//! 统一的渲染后端接口
//!
//! 本模块定义了所有图形后端（Vulkan, DX12, Metal, wgpu）都必须实现的统一接口。
//! 这样可以在不同的图形 API 之间无缝切换，而不需要修改上层渲染逻辑。
//!
//! # 设计理念
//!
//! - **抽象化**：隐藏不同图形 API 的实现细节
//! - **统一接口**：提供一致的调用方式
//! - **可扩展性**：方便添加新的图形后端
//! - **零成本抽象**：使用 trait object 的开销可以忽略不计

use crate::core::error::Result;
use crate::core::input::InputSystem;
use crate::gui::ipc::GuiStatePacket;
use winit::event::WindowEvent;
use winit::window::Window;

/// 统一的渲染后端接口
///
/// 所有具体的图形后端（如 Vulkan、DirectX 12、Metal、wgpu）都必须实现此 trait。
///
/// # 方法说明
///
/// - `window()`: 获取窗口引用，用于窗口相关操作
/// - `resize()`: 处理窗口尺寸变化事件
/// - `draw()`: 渲染一帧画面
/// - `update()`: 更新渲染器状态（处理输入、更新相机等）
/// - `apply_gui_packet()`: 应用 GUI 参数包
/// - `handle_gui_event()`: 处理 GUI 事件（默认不处理）
///
/// # 示例
///
/// ```ignore
/// // 创建后端实例（通过 trait object）
/// let backend: Box<dyn RenderBackend> = Box::new(VulkanRenderer::new(...)?);
///
/// // 使用统一接口
/// backend.update(&mut input_system, delta_time);
/// backend.draw()?;
/// ```
pub trait RenderBackend {
    /// 获取窗口的引用
    ///
    /// 返回与此图形后端关联的窗口引用，用于获取窗口尺寸、处理窗口事件等。
    ///
    /// # 返回值
    ///
    /// 窗口的不可变引用
    fn window(&self) -> &Window;

    /// 窗口尺寸变化时调用
    ///
    /// 当窗口被用户调整大小时，需要重新创建交换链和相关资源。
    /// 不同的图形 API 有不同的实现方式。
    fn resize(&mut self);

    /// 渲染一帧画面
    ///
    /// 执行实际的渲染工作，包括：
    /// - 获取下一帧图像
    /// - 记录渲染命令
    /// - 提交命令队列
    /// - 呈现到屏幕
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 渲染成功
    /// - `Err(...)`: 渲染失败（如设备丢失、交换链过期等）
    fn draw(&mut self) -> Result<()>;

    /// 更新渲染器状态
    ///
    /// 在每帧渲染前调用，用于：
    /// - 处理用户输入
    /// - 更新相机位置和方向
    /// - 更新动画状态
    /// - 更新 uniform buffer 等
    ///
    /// # 参数
    ///
    /// * `input_system` - 输入系统的可变引用
    /// * `delta_time` - 距离上一帧的时间间隔（秒）
    fn update(&mut self, input_system: &mut InputSystem, delta_time: f32);

    /// 应用 GUI 参数包
    ///
    /// 当使用外部 GUI 进程时，通过共享内存传递参数。
    /// 此方法将 GUI 参数应用到渲染器状态。
    ///
    /// # 参数
    ///
    /// * `packet` - GUI 状态参数包
    fn apply_gui_packet(&mut self, packet: &GuiStatePacket);

    /// 处理 GUI 事件
    ///
    /// 对于内置 GUI 的后端（如 wgpu + egui），需要将窗口事件传递给 GUI 系统。
    ///
    /// # 参数
    ///
    /// * `event` - 窗口事件
    ///
    /// # 返回值
    ///
    /// - `true`: 事件被 GUI 消费，不应继续传播
    /// - `false`: 事件未被 GUI 消费，应继续处理
    ///
    /// # 默认实现
    ///
    /// 默认返回 `false`，表示不处理 GUI 事件。
    /// 只有 wgpu 后端需要重写此方法。
    fn handle_gui_event(&mut self, _event: &WindowEvent) -> bool {
        false // 默认不处理
    }
}
