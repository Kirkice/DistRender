//! Vulkan 图形 API 实现模块
//!
//! 本模块包含了所有 Vulkan 相关的代码，包括：
//! - Backend: Vulkan 设备、队列、内存分配器等基础设施
//! - Renderer: Vulkan 渲染器实现
//! - Descriptor: Vulkan 描述符管理
//! - Shaders: Vulkan shader 加载

pub mod backend;
pub mod renderer;
pub mod descriptor;
pub mod shaders;

// 重新导出常用类型
pub use backend::VulkanBackend;
pub use renderer::Renderer;
pub use descriptor::VulkanDescriptorManager;
