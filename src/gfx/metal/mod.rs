//! Metal 图形后端模块
//!
//! 本模块提供了基于 Apple Metal API 的图形后端实现。
//! 仅在 macOS/iOS 平台上可用。

#[cfg(target_os = "macos")]
pub mod context;
#[cfg(target_os = "macos")]
pub mod renderer;

#[cfg(target_os = "macos")]
pub use context::MetalContext;
#[cfg(target_os = "macos")]
pub use renderer::Renderer;
