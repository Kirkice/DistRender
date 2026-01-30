//! DistRender - 多后端渲染引擎
//!
//! DistRender 是一个支持 Vulkan 和 DirectX 12 的跨平台渲染引擎。
//! 本库提供了统一的渲染接口和核心功能模块。
//!
//! # 模块结构
//!
//! - `math`: 数学库模块（向量、矩阵、四元数、几何处理）
//! - `core`: 核心功能模块（日志、配置、错误处理、事件系统、场景）
//! - `geometry`: 几何体加载模块（顶点、网格、OBJ/FBX加载器）
//! - `component`: 组件系统（Transform、Camera、Light）
//! - `renderer`: 渲染器模块（统一接口和资源管理）
//! - `gfx`: 图形后端抽象层（Vulkan、DX12、Metal、wgpu）
//! - `gui`: GUI 模块（外部 GUI 和性能监控）
//!
//! # 使用示例
//!
//! ```no_run
//! use dist_render::core::event::*;
//!
//! // 创建窗口调整大小事件
//! let mut event = WindowResizeEvent::new(1920, 1080);
//!
//! // 创建事件分发器
//! let mut dispatcher = EventDispatcher::new(&mut event);
//!
//! // 分发事件
//! dispatcher.dispatch(EventType::WindowResize, |e| {
//!     println!("窗口调整为: {}", e.detail());
//!     true
//! });
//! ```
pub mod math;pub mod core;
pub mod geometry;
pub mod component;
pub mod gui;
pub mod renderer;
pub mod gfx;