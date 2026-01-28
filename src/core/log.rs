//! 日志系统模块
//!
//! 基于 `tracing` 提供结构化的日志记录功能。
//! 类似于 C++ DistEngine 中的 spdlog 封装，但更强大。
//!
//! # 特性
//!
//! - 结构化日志：支持键值对
//! - 高性能：零成本抽象，编译时优化
//! - 灵活输出：支持控制台和文件输出
//! - 日志级别：trace, debug, info, warn, error
//!
//! # 使用示例
//!
//! ```no_run
//! use crate::core::log;
//!
//! // 初始化日志系统
//! log::init_logger(log::LogLevel::Info, false);
//!
//! // 使用宏记录日志
//! log::info!("Application started");
//! log::warn!("Low memory warning");
//! log::error!("Failed to load texture");
//!
//! // 结构化日志
//! log::info!(width = 800, height = 600, "Window created");
//! ```

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use std::path::Path;

use super::config::LogLevel;

/// 初始化日志系统
///
/// 必须在程序开始时调用一次。
///
/// # 参数
///
/// * `level` - 日志级别
/// * `file_output` - 是否输出到文件
/// * `log_file_path` - 日志文件路径（可选，默认为 "distrender.log"）
///
/// # 示例
///
/// ```no_run
/// use crate::core::log::{self, LogLevel};
///
/// // 仅控制台输出
/// log::init_logger(LogLevel::Info, false, None);
///
/// // 同时输出到文件
/// log::init_logger(LogLevel::Debug, true, Some("logs/app.log"));
/// ```
pub fn init_logger(level: LogLevel, file_output: bool, log_file_path: Option<&str>) {
    let filter = match level {
        LogLevel::Trace => EnvFilter::new("trace"),
        LogLevel::Debug => EnvFilter::new("debug"),
        LogLevel::Info => EnvFilter::new("info"),
        LogLevel::Warn => EnvFilter::new("warn"),
        LogLevel::Error => EnvFilter::new("error"),
    };

    if file_output {
        // 解析日志文件路径
        let log_path = log_file_path.unwrap_or("distrender.log");
        let path = Path::new(log_path);
        let directory = path.parent().unwrap_or(Path::new("."));
        let filename = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("distrender.log");

        // 创建滚动文件 appender（每天滚动）
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            directory,
            filename
        );

        // 创建格式化层
        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_ansi(true);

        let file_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_ansi(false)  // 文件不需要 ANSI 颜色
            .with_writer(file_appender);

        // 组合控制台和文件输出
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        // 仅控制台输出
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_span_events(FmtSpan::CLOSE)
            .with_ansi(true);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

/// 初始化简单的日志系统（仅控制台输出）
///
/// 使用默认的 Info 级别。
#[allow(dead_code)]
pub fn init_simple() {
    init_logger(LogLevel::Info, false, None);
}

// 重新导出 tracing 的宏，提供类似 spdlog 的接口

// 定义类似 DistEngine 的宏
/// 引擎核心日志 - Info 级别
#[macro_export]
macro_rules! engine_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "distrender::engine", $($arg)*)
    };
}

/// 引擎核心日志 - Warn 级别
#[macro_export]
macro_rules! engine_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "distrender::engine", $($arg)*)
    };
}

/// 引擎核心日志 - Error 级别
#[macro_export]
macro_rules! engine_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "distrender::engine", $($arg)*)
    };
}

/// 应用层日志 - Info 级别
#[macro_export]
macro_rules! app_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "distrender::app", $($arg)*)
    };
}

/// 应用层日志 - Warn 级别
#[macro_export]
macro_rules! app_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "distrender::app", $($arg)*)
    };
}

/// 应用层日志 - Error 级别
#[macro_export]
macro_rules! app_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "distrender::app", $($arg)*)
    };
}

/// 日志级别转换
impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

/// 日志辅助函数：检查日志级别是否启用
#[allow(dead_code)]
pub fn is_enabled(level: Level) -> bool {
    tracing::level_filters::LevelFilter::current() >= level
}

/// 性能追踪宏
///
/// 记录函数执行时间。
///
/// # 示例
///
/// ```no_run
/// use crate::core::log::span_trace;
///
/// fn expensive_operation() {
///     let _span = span_trace!("expensive_operation");
///     // 函数结束时会自动记录执行时间
/// }
/// ```
#[macro_export]
macro_rules! span_trace {
    ($name:expr) => {
        tracing::span!(tracing::Level::TRACE, $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(Level::from(LogLevel::Info), Level::INFO);
        assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
    }
}
