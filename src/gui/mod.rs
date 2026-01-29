//! GUI 系统模块
//!
//! 基于 egui + wgpu 实现的统一 GUI 系统，支持所有图形后端。

mod manager;
mod state;
mod metrics;
pub mod panels;

pub mod ipc;
mod external;

pub use external::ExternalGui;
pub use manager::GuiManager;
pub use state::GuiState;
