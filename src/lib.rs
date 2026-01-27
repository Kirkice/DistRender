//! DistRender - 多后端渲染引擎
//!
//! DistRender 是一个支持 Vulkan 和 DirectX 12 的跨平台渲染引擎。
//! 本库提供了统一的渲染接口和核心功能模块。
//!
//! # 模块结构
//!
//! - `core`: 核心功能模块（数学、日志、配置、错误处理、事件系统）
//! - `geometry`: 几何体加载模块（顶点、网格、OBJ/FBX加载器）
//! - `renderer`: 渲染器模块（Vulkan 和 DX12 实现）
//! - `gfx`: 图形后端抽象层
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

pub mod core;
pub mod geometry;
