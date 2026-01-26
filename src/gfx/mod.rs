//! 图形后端模块
//!
//! 本模块封装了不同图形 API 的底层实现，包括：
//! - Vulkan：跨平台的现代图形 API
//! - DirectX 12：Windows 平台的高性能图形 API
//!
//! 所有后端都实现了统一的 `GraphicsBackend` trait，
//! 确保可以在不同的图形 API 之间无缝切换。

pub mod backend;
pub mod vulkan;
pub mod dx12;

pub use backend::GraphicsBackend;
pub use vulkan::VulkanBackend;
pub use dx12::Dx12Backend;
