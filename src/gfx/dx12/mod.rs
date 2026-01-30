//! DirectX 12 图形 API 实现模块
//!
//! 本模块包含了所有 DirectX 12 相关的代码，包括：
//! - Backend: DX12 设备、命令队列、交换链等基础设施
//! - Renderer: DX12 渲染器实现
//! - Descriptor: DX12 描述符管理

pub mod context;
pub mod renderer;
pub mod descriptor;

// 重新导出常用类型
pub use context::Dx12Context;
pub use renderer::Renderer;
