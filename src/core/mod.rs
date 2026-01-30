//! 核心功能模块
//!
//! 本模块提供了渲染引擎的基础功能，包括日志系统、配置管理和错误处理。
//! 这些模块独立于具体的图形 API，可以在任何渲染后端中使用。
//!
//! # 模块组织
//!
//! - `log`：日志系统，提供结构化的日志记录功能
//! - `config`：配置管理，支持从配置文件加载引擎设置
//! - `error`：错误处理，定义统一的错误类型
//! - `event`：事件系统，提供统一的事件处理机制
//! - `scene`：场景配置，管理相机和模型的变换数据
//! - `input`：输入系统，处理键盘和鼠标输入
//! - `runtime`：运行时管理，负责后端初始化
//!
//! # 设计理念
//!
//! Core 模块参考了 DistEngine (C++) 的设计：
//! - **模块化**：清晰的职责划分
//! - **可复用**：与具体渲染 API 解耦
//! - **高性能**：使用 Rust 的零成本抽象
//! - **易用性**：提供友好的 API

pub mod log;
pub mod config;
pub mod error;
pub mod event;
pub mod scene;
pub mod input;

pub mod runtime;

// 重新导出常用类型，方便使用
pub use config::Config;
pub use scene::SceneConfig;
pub use runtime::{RendererBackendKind, init_renderer_backend, renderer_backend};