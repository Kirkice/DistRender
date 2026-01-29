//! wgpu 图形后端实现
//!
//! 本模块实现了基于 wgpu 的图形后端，wgpu 是一个跨平台的图形 API，
//! 可以在 Vulkan、Metal、DirectX 12、OpenGL 等多种后端上运行。
//!
//! # 模块结构
//!
//! - `backend` - WgpuBackend 结构（设备初始化和管理）
//! - `renderer` - Renderer 结构（渲染逻辑实现）

mod backend;
mod renderer;

pub use backend::WgpuBackend;
pub use renderer::Renderer;
