//! 核心功能模块
//!
//! 本模块提供了渲染引擎的基础功能，包括数学库、日志系统、配置管理和错误处理。
//! 这些模块独立于具体的图形 API，可以在任何渲染后端中使用。
//!
//! # 模块组织
//!
//! - `math`：数学库，提供向量、矩阵、四元数等常用数学类型
//! - `log`：日志系统，提供结构化的日志记录功能
//! - `config`：配置管理，支持从配置文件加载引擎设置
//! - `error`：错误处理，定义统一的错误类型
//! - `event`：事件系统，提供统一的事件处理机制
//!
//! # 设计理念
//!
//! Core 模块参考了 DistEngine (C++) 的设计：
//! - **模块化**：清晰的职责划分
//! - **可复用**：与具体渲染 API 解耦
//! - **高性能**：使用 Rust 的零成本抽象
//! - **易用性**：提供友好的 API

pub mod math;
pub mod log;
pub mod config;
pub mod error;
pub mod event;

// 重新导出常用类型，方便使用
pub use math::{Vector2, Vector3, Vector4, Matrix4, Quaternion, Color};
pub use error::{Result, DistRenderError};
pub use config::Config;
pub use event::{
    Event, EventType, EventDispatcher, EventHandler,
    WindowResizeEvent, WindowCloseEvent,
    MouseButtonEvent, MouseMoveEvent, MouseScrollEvent, MouseButton,
    KeyboardEvent, KeyCode,
    TickEvent, DrawEvent,
};
