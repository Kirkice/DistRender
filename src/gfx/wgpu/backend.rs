//! wgpu 后端设备管理
//!
//! 本模块负责 wgpu 图形设备的初始化和管理，包括：
//! - 创建 wgpu 实例
//! - 创建窗口表面
//! - 选择和创建图形适配器
//! - 创建逻辑设备和命令队列
//! - 配置交换链

use std::sync::Arc;
use tracing::{info, debug};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use wgpu;

use crate::gfx::GraphicsBackend;
use crate::core::Config;
use crate::core::error::{Result, GraphicsError};

/// wgpu 图形后端
///
/// 封装了 wgpu 的核心设备和表面管理。
pub struct WgpuBackend {
    /// wgpu 实例（入口点）
    pub instance: wgpu::Instance,
    /// 窗口表面
    pub surface: wgpu::Surface<'static>,
    /// 图形适配器（GPU）
    pub adapter: wgpu::Adapter,
    /// 逻辑设备
    pub device: wgpu::Device,
    /// 命令队列
    pub queue: wgpu::Queue,
    /// 表面配置
    pub surface_config: wgpu::SurfaceConfiguration,
    /// 窗口引用
    window: Arc<Window>,
}

impl WgpuBackend {
    /// 创建 wgpu 后端实例
    ///
    /// # 参数
    ///
    /// * `event_loop` - winit 事件循环引用
    /// * `config` - 引擎配置
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 WgpuBackend 实例
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Result<Self> {
        info!("Initializing wgpu backend");

        // 1. 创建 wgpu 实例
        debug!("Creating wgpu instance");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),  // 支持所有后端（Vulkan, Metal, DX12, OpenGL）
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        // 2. 创建窗口
        debug!("Creating window");
        let title = format!("{} [{}]", config.window.title, config.graphics.backend.name());
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                config.window.width,
                config.window.height,
            ))
            .with_resizable(config.window.resizable)
            .build(event_loop)
            .map_err(|e| GraphicsError::DeviceCreation(format!("Failed to create window: {}", e)))?;

        let window = Arc::new(window);

        // 3. 创建表面（wgpu 0.19 API）
        debug!("Creating surface");
        let surface = instance.create_surface(window.clone())
            .map_err(|e| GraphicsError::DeviceCreation(format!("Failed to create surface: {}", e)))?;

        // 4. 请求适配器（选择 GPU）
        debug!("Requesting adapter");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,  // 优先选择高性能 GPU
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| GraphicsError::DeviceCreation("Failed to find suitable adapter".to_string()))?;

        info!("Selected adapter: {:?}", adapter.get_info());

        // 5. 请求设备和队列
        debug!("Requesting device and queue");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,  // 不跟踪 API 调用
        ))
        .map_err(|e| GraphicsError::DeviceCreation(format!("Failed to create device: {}", e)))?;

        // 6. 配置表面
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb))  // 优先选择 sRGB 格式
            .unwrap_or(surface_caps.formats[0]);

        debug!("Surface format: {:?}", surface_format);

        let present_mode = if config.graphics.vsync {
            wgpu::PresentMode::Fifo  // 垂直同步
        } else {
            wgpu::PresentMode::Immediate  // 立即呈现
        };

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        info!("wgpu backend initialized successfully");

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_config,
            window,
        })
    }

    /// 获取窗口引用
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// 重新配置表面（用于窗口调整）
    pub fn reconfigure_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

impl GraphicsBackend for WgpuBackend {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self
    where
        Self: Sized,
    {
        WgpuBackend::new(event_loop, config).expect("Failed to create wgpu backend")
    }

    fn window(&self) -> &Window {
        self.window()
    }

    fn backend_name(&self) -> &str {
        "wgpu"
    }
}
