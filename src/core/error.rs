//! 错误处理模块
//!
//! 定义了引擎中使用的统一错误类型，使用 `thiserror` 提供友好的错误消息。
//!
//! # 设计原则
//!
//! - 使用 `thiserror` 自动实现 `Error` trait
//! - 为每种错误类型提供清晰的上下文信息
//! - 支持错误链（error source）
//! - 易于模式匹配和错误处理

use std::fmt;
use std::path::PathBuf;

/// 引擎统一的 Result 类型
///
/// 所有可能返回错误的函数都应该使用这个类型。
pub type Result<T> = std::result::Result<T, DistRenderError>;

/// DistRender 引擎的错误类型
///
/// 包含了引擎运行过程中可能遇到的各种错误情况。
#[derive(Debug)]
pub enum DistRenderError {
    /// 配置错误
    Config(ConfigError),

    /// 图形 API 错误
    Graphics(GraphicsError),

    /// 网格加载错误
    MeshLoading(MeshLoadError),

    /// IO 错误
    Io(std::io::Error),

    /// 日志系统错误
    Log(String),

    /// 初始化错误
    Initialization(String),

    /// 运行时错误
    Runtime(String),
}

/// 配置相关的错误
#[derive(Debug)]
pub enum ConfigError {
    /// 配置文件未找到
    FileNotFound(String),

    /// 配置文件解析失败
    ParseError(String),

    /// 配置项缺失
    MissingField(String),

    /// 配置值无效
    InvalidValue { field: String, reason: String },
}

/// 图形 API 相关的错误
#[derive(Debug)]
pub enum GraphicsError {
    /// 设备创建失败
    DeviceCreation(String),

    /// 交换链错误
    SwapchainError(String),

    /// 着色器编译失败
    ShaderCompilation(String),

    /// 资源创建失败
    ResourceCreation(String),

    /// 渲染命令执行失败
    CommandExecution(String),
}

/// 网格加载相关的错误
#[derive(Debug)]
pub enum MeshLoadError {
    /// 文件不存在
    FileNotFound(PathBuf),

    /// 不支持的文件格式
    UnsupportedFormat(String),

    /// 解析失败
    ParseError(String),

    /// 数据验证失败
    ValidationError(String),

    /// 几何数据无效
    InvalidGeometry(String),

    /// 外部库错误
    ExternalLibraryError(String),
}

impl fmt::Display for DistRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DistRenderError::Config(e) => write!(f, "Configuration error: {}", e),
            DistRenderError::Graphics(e) => write!(f, "Graphics error: {}", e),
            DistRenderError::MeshLoading(e) => write!(f, "Mesh loading error: {}", e),
            DistRenderError::Io(e) => write!(f, "IO error: {}", e),
            DistRenderError::Log(msg) => write!(f, "Log error: {}", msg),
            DistRenderError::Initialization(msg) => write!(f, "Initialization error: {}", msg),
            DistRenderError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse config: {}", msg),
            ConfigError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ConfigError::InvalidValue { field, reason } => {
                write!(f, "Invalid value for '{}': {}", field, reason)
            }
        }
    }
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphicsError::DeviceCreation(msg) => write!(f, "Device creation failed: {}", msg),
            GraphicsError::SwapchainError(msg) => write!(f, "Swapchain error: {}", msg),
            GraphicsError::ShaderCompilation(msg) => write!(f, "Shader compilation failed: {}", msg),
            GraphicsError::ResourceCreation(msg) => write!(f, "Resource creation failed: {}", msg),
            GraphicsError::CommandExecution(msg) => write!(f, "Command execution failed: {}", msg),
        }
    }
}

impl fmt::Display for MeshLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshLoadError::FileNotFound(path) => write!(f, "Mesh file not found: {}", path.display()),
            MeshLoadError::UnsupportedFormat(msg) => write!(f, "Unsupported mesh format: {}", msg),
            MeshLoadError::ParseError(msg) => write!(f, "Failed to parse mesh: {}", msg),
            MeshLoadError::ValidationError(msg) => write!(f, "Mesh validation failed: {}", msg),
            MeshLoadError::InvalidGeometry(msg) => write!(f, "Invalid geometry data: {}", msg),
            MeshLoadError::ExternalLibraryError(msg) => write!(f, "External library error: {}", msg),
        }
    }
}

impl std::error::Error for DistRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DistRenderError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for ConfigError {}
impl std::error::Error for GraphicsError {}
impl std::error::Error for MeshLoadError {}

// 实现 From trait 以便于错误转换
impl From<std::io::Error> for DistRenderError {
    fn from(err: std::io::Error) -> Self {
        DistRenderError::Io(err)
    }
}

impl From<ConfigError> for DistRenderError {
    fn from(err: ConfigError) -> Self {
        DistRenderError::Config(err)
    }
}

impl From<GraphicsError> for DistRenderError {
    fn from(err: GraphicsError) -> Self {
        DistRenderError::Graphics(err)
    }
}

impl From<MeshLoadError> for DistRenderError {
    fn from(err: MeshLoadError) -> Self {
        DistRenderError::MeshLoading(err)
    }
}
