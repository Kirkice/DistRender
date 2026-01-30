//! 渲染命令和同步模块
//!
//! 包含与渲染命令执行和同步相关的所有类型和功能：
//! - 命令缓冲管理
//! - 同步原语（Fence、Semaphore等）

pub mod command;
pub mod sync;

// 重新导出常用类型
pub use sync::{FenceManager, FenceValue};
