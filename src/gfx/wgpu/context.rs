//! wgpu 鍚庣璁惧绠＄悊
//!
//! 鏈ā鍧楄礋璐?wgpu 鍥惧舰璁惧鐨勫垵濮嬪寲鍜岀鐞嗭紝鍖呮嫭锛?
//! - 鍒涘缓 wgpu 瀹炰緥
//! - 鍒涘缓绐楀彛琛ㄩ潰
//! - 閫夋嫨鍜屽垱寤哄浘褰㈤€傞厤鍣?
//! - 鍒涘缓閫昏緫璁惧鍜屽懡浠ら槦鍒?
//! - 閰嶇疆浜ゆ崲閾?

use std::sync::Arc;
use tracing::{info, debug};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use wgpu;

use crate::gfx::GraphicsBackend;
use crate::core::Config;
use crate::core::error::{Result, GraphicsError};

/// wgpu 鍥惧舰鍚庣
///
/// 灏佽浜?wgpu 鐨勬牳蹇冭澶囧拰琛ㄩ潰绠＄悊銆?
pub struct WgpuContext {
    /// wgpu 瀹炰緥锛堝叆鍙ｇ偣锛?
    pub instance: wgpu::Instance,
    /// 绐楀彛琛ㄩ潰
    pub surface: wgpu::Surface<'static>,
    /// 鍥惧舰閫傞厤鍣紙GPU锛?
    pub adapter: wgpu::Adapter,
    /// 閫昏緫璁惧
    pub device: wgpu::Device,
    /// 鍛戒护闃熷垪
    pub queue: wgpu::Queue,
    /// 琛ㄩ潰閰嶇疆
    pub surface_config: wgpu::SurfaceConfiguration,
    /// 绐楀彛寮曠敤
    window: Arc<Window>,
}

impl WgpuContext {
    /// 鍒涘缓 wgpu 鍚庣瀹炰緥
    ///
    /// # 鍙傛暟
    ///
    /// * `event_loop` - winit 浜嬩欢寰幆寮曠敤
    /// * `config` - 寮曟搸閰嶇疆
    ///
    /// # 杩斿洖鍊?
    ///
    /// 杩斿洖鍒濆鍖栧畬鎴愮殑 WgpuContext 瀹炰緥
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Result<Self> {
        info!("Initializing wgpu backend");

        // 1. 鍒涘缓 wgpu 瀹炰緥
        debug!("Creating wgpu instance");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),  // 鏀寔鎵€鏈夊悗绔紙Vulkan, Metal, DX12, OpenGL锛?
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        // 2. 鍒涘缓绐楀彛
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

        // 3. 鍒涘缓琛ㄩ潰锛坵gpu 0.19 API锛?
        debug!("Creating surface");
        let surface = instance.create_surface(window.clone())
            .map_err(|e| GraphicsError::DeviceCreation(format!("Failed to create surface: {}", e)))?;

        // 4. 璇锋眰閫傞厤鍣紙閫夋嫨 GPU锛?
        debug!("Requesting adapter");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,  // 浼樺厛閫夋嫨楂樻€ц兘 GPU
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| GraphicsError::DeviceCreation("Failed to find suitable adapter".to_string()))?;

        info!("Selected adapter: {:?}", adapter.get_info());

        // 5. 璇锋眰璁惧鍜岄槦鍒?
        debug!("Requesting device and queue");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,  // 涓嶈窡韪?API 璋冪敤
        ))
        .map_err(|e| GraphicsError::DeviceCreation(format!("Failed to create device: {}", e)))?;

        // 6. 閰嶇疆琛ㄩ潰
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb))  // 浼樺厛閫夋嫨 sRGB 鏍煎紡
            .unwrap_or(surface_caps.formats[0]);

        debug!("Surface format: {:?}", surface_format);

        let present_mode = if config.graphics.vsync {
            wgpu::PresentMode::Fifo  // 鍨傜洿鍚屾
        } else {
            wgpu::PresentMode::Immediate  // 绔嬪嵆鍛堢幇
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

    /// 鑾峰彇绐楀彛寮曠敤
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// 閲嶆柊閰嶇疆琛ㄩ潰锛堢敤浜庣獥鍙ｈ皟鏁达級
    pub fn reconfigure_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

impl GraphicsBackend for WgpuContext {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self
    where
        Self: Sized,
    {
        WgpuContext::new(event_loop, config).expect("Failed to create wgpu backend")
    }

    fn window(&self) -> &Window {
        self.window()
    }

    fn backend_name(&self) -> &str {
        "wgpu"
    }
}
