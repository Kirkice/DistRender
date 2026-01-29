//! 配置管理模块
//!
//! 提供引擎配置的加载、解析和管理功能。
//! 支持从 TOML 配置文件加载，也支持命令行参数覆盖。
//!
//! # 配置文件格式 (config.toml)
//!
//! ```toml
//! [window]
//! width = 800
//! height = 600
//! title = "DistRender"
//! resizable = true
//!
//! [graphics]
//! backend = "vulkan"  # 或 "dx12"
//! vsync = true
//! msaa_samples = 4
//!
//! [logging]
//! level = "info"      # trace, debug, info, warn, error
//! file_output = true
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::error::{ConfigError, Result};

/// 引擎配置
///
/// 包含了引擎运行所需的所有配置项。
/// 可以从配置文件加载，也可以通过代码构建。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 窗口配置
    pub window: WindowConfig,

    /// 图形配置
    pub graphics: GraphicsConfig,

    /// 日志配置
    pub logging: LoggingConfig,
}

/// 窗口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// 窗口宽度
    #[serde(default = "default_width")]
    pub width: u32,

    /// 窗口高度
    #[serde(default = "default_height")]
    pub height: u32,

    /// 窗口标题
    #[serde(default = "default_title")]
    pub title: String,

    /// 是否可调整大小
    #[serde(default = "default_resizable")]
    pub resizable: bool,
}

/// 图形配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsConfig {
    /// 图形后端选择
    #[serde(default = "default_backend")]
    pub backend: GraphicsBackend,

    /// 垂直同步
    #[serde(default = "default_vsync")]
    pub vsync: bool,

    /// MSAA 采样数
    #[serde(default = "default_msaa")]
    pub msaa_samples: u32,
}

/// 图形后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphicsBackend {
    /// Vulkan 后端
    Vulkan,
    /// DirectX 12 后端
    Dx12,
    /// wgpu 后端（支持 Vulkan、Metal、DX12、OpenGL）
    Wgpu,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub level: LogLevel,

    /// 是否输出到文件
    #[serde(default = "default_file_output")]
    pub file_output: bool,

    /// 日志文件路径
    #[serde(default = "default_log_file")]
    pub log_file: String,
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// 默认值函数
fn default_width() -> u32 { 800 }
fn default_height() -> u32 { 600 }
fn default_title() -> String { "DistRender".to_string() }
fn default_resizable() -> bool { true }
fn default_backend() -> GraphicsBackend { GraphicsBackend::Vulkan }
fn default_vsync() -> bool { true }
fn default_msaa() -> u32 { 1 }
fn default_log_level() -> LogLevel { LogLevel::Info }
fn default_file_output() -> bool { false }
fn default_log_file() -> String { "distrender.log".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            graphics: GraphicsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            title: default_title(),
            resizable: default_resizable(),
        }
    }
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            vsync: default_vsync(),
            msaa_samples: default_msaa(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_output: default_file_output(),
            log_file: default_log_file(),
        }
    }
}

impl Config {
    /// 从配置文件加载
    ///
    /// # 参数
    ///
    /// * `path` - 配置文件路径
    ///
    /// # 返回值
    ///
    /// 成功返回 `Config` 实例，失败返回错误
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use crate::core::Config;
    ///
    /// let config = Config::from_file("config.toml")?;
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let contents = std::fs::read_to_string(path)
            .map_err(|_| ConfigError::FileNotFound(path_str.clone()))?;

        toml::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(e.to_string()).into())
    }

    /// 从配置文件加载，如果文件不存在则使用默认配置
    ///
    /// # 参数
    ///
    /// * `path` - 配置文件路径
    ///
    /// # 返回值
    ///
    /// 返回 `Config` 实例
    pub fn from_file_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::from_file(path).unwrap_or_default()
    }

    /// 保存配置到文件
    ///
    /// # 参数
    ///
    /// * `path` - 配置文件路径
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    #[allow(dead_code)]
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        std::fs::write(path, contents)?;
        Ok(())
    }

    /// 从命令行参数覆盖配置
    ///
    /// # 参数
    ///
    /// * `args` - 命令行参数迭代器
    ///
    /// # 说明
    ///
    /// 支持的参数：
    /// - `--dx12`: 使用 DirectX 12 后端
    /// - `--width <value>`: 设置窗口宽度
    /// - `--height <value>`: 设置窗口高度
    pub fn apply_args<I>(&mut self, args: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let args: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

        // 检查是否使用 DX12
        if args.iter().any(|a| a == "--dx12") {
            self.graphics.backend = GraphicsBackend::Dx12;
        }

        // 检查是否使用 wgpu
        if args.iter().any(|a| a == "--wgpu") {
            self.graphics.backend = GraphicsBackend::Wgpu;
        }

        // 检查窗口尺寸
        if let Some(idx) = args.iter().position(|a| a == "--width") {
            if let Some(width_str) = args.get(idx + 1) {
                if let Ok(width) = width_str.parse() {
                    self.window.width = width;
                }
            }
        }

        if let Some(idx) = args.iter().position(|a| a == "--height") {
            if let Some(height_str) = args.get(idx + 1) {
                if let Ok(height) = height_str.parse() {
                    self.window.height = height;
                }
            }
        }
    }

    /// 验证配置的有效性
    ///
    /// # 返回值
    ///
    /// 配置有效返回 `Ok(())`，否则返回错误
    pub fn validate(&self) -> Result<()> {
        // 验证窗口尺寸
        if self.window.width == 0 || self.window.height == 0 {
            return Err(ConfigError::InvalidValue {
                field: "window.width/height".to_string(),
                reason: "Window dimensions must be greater than 0".to_string(),
            }.into());
        }

        // 验证 MSAA 采样数
        if !matches!(self.graphics.msaa_samples, 1 | 2 | 4 | 8 | 16) {
            return Err(ConfigError::InvalidValue {
                field: "graphics.msaa_samples".to_string(),
                reason: "MSAA samples must be 1, 2, 4, 8, or 16".to_string(),
            }.into());
        }

        Ok(())
    }
}

impl GraphicsBackend {
    /// 检查是否为 DX12 后端
    #[allow(dead_code)]
    pub fn is_dx12(&self) -> bool {
        matches!(self, GraphicsBackend::Dx12)
    }

    /// 检查是否为 Vulkan 后端
    #[allow(dead_code)]
    pub fn is_vulkan(&self) -> bool {
        matches!(self, GraphicsBackend::Vulkan)
    }

    /// 检查是否为 wgpu 后端
    #[allow(dead_code)]
    pub fn is_wgpu(&self) -> bool {
        matches!(self, GraphicsBackend::Wgpu)
    }

    /// 获取后端名称
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            GraphicsBackend::Vulkan => "Vulkan",
            GraphicsBackend::Dx12 => "DirectX 12",
            GraphicsBackend::Wgpu => "wgpu",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.window.width, 800);
        assert_eq!(config.window.height, 600);
        assert_eq!(config.graphics.backend, GraphicsBackend::Vulkan);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.window.width = 0;
        assert!(config.validate().is_err());
    }
}
