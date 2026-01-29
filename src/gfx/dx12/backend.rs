//! DirectX 12 图形后端实现
//!
//! 本模块提供了基于 DirectX 12 API 的图形后端实现。
//! DirectX 12 是 Microsoft 为 Windows 平台开发的高性能图形 API，
//! 提供了对 GPU 的底层控制能力。
//!
//! # 主要组件
//!
//! - `Dx12Backend`：DirectX 12 后端的主要结构体，封装了设备、命令队列、交换链等核心资源
//!
//! # 初始化流程
//!
//! 1. 启用调试层（Debug 模式）
//! 2. 创建 DXGI 工厂
//! 3. 创建 D3D12 设备
//! 4. 创建命令队列
//! 5. 创建交换链
//! 6. 创建描述符堆和渲染目标视图
//! 7. 创建同步对象（Fence）

use std::sync::Arc;
use tracing::{debug, info, warn};
use windows::{
    core::*, Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D12::*,
    Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Dxgi::*,
};
use winit::window::{Window, WindowBuilder};
use winit::event_loop::EventLoop;
use winit::dpi::LogicalSize;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::gfx::backend::GraphicsBackend;
use crate::core::Config;

/// DirectX 12 图形后端
///
/// 封装了 DirectX 12 图形 API 的核心资源和功能。
/// 包括设备、命令队列、交换链、描述符堆、同步对象等，为渲染器提供底层支持。
///
/// # 字段说明
///
/// - `device`：D3D12 设备，用于创建和管理 GPU 资源
/// - `command_queue`：命令队列，用于提交渲染命令到 GPU
/// - `swap_chain`：交换链，管理前后缓冲区
/// - `rtv_heap`：渲染目标视图（RTV）描述符堆
/// - `rtv_descriptor_size`：RTV 描述符的大小（字节）
/// - `frame_index`：当前帧在交换链中的索引
/// - `fence`：栅栏对象，用于 CPU-GPU 同步
/// - `fence_value`：当前栅栏值
/// - `fence_event`：栅栏事件句柄
/// - `window`：窗口引用
/// - `width`：窗口宽度
/// - `height`：窗口高度
pub struct Dx12Backend {
    /// D3D12 设备
    pub device: ID3D12Device,
    /// 命令队列
    pub command_queue: ID3D12CommandQueue,
    /// 交换链
    pub swap_chain: IDXGISwapChain3,
    /// 渲染目标视图描述符堆
    pub rtv_heap: ID3D12DescriptorHeap,
    /// RTV 描述符大小
    pub rtv_descriptor_size: usize,
    /// 当前帧索引
    pub frame_index: usize,
    /// 同步栅栏
    pub fence: ID3D12Fence,
    /// 栅栏值
    pub fence_value: u64,
    /// 栅栏事件句柄
    pub fence_event: windows::Win32::Foundation::HANDLE,
    /// 窗口引用
    pub window: Arc<Window>,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
}

// 为了在多线程环境中使用，需要实现 Send 和 Sync
// DirectX 12 的对象是线程安全的
unsafe impl Send for Dx12Backend {}
unsafe impl Sync for Dx12Backend {}

impl Dx12Backend {
    /// 创建新的 DirectX 12 后端
    ///
    /// 初始化 DirectX 12 的所有核心组件，包括设备、命令队列、交换链等。
    /// 在 Debug 模式下会启用调试层以便于开发时的错误检查。
    ///
    /// # 参数
    ///
    /// * `event_loop` - Winit 事件循环的引用，用于创建窗口
    /// * `config` - 引擎配置，用于设置窗口大小、标题等参数
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `Dx12Backend` 实例
    ///
    /// # Panics
    ///
    /// 如果无法创建设备、命令队列、交换链或同步对象，会 panic
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use winit::event_loop::EventLoop;
    /// use crate::gfx::Dx12Backend;
    /// use crate::core::Config;
    ///
    /// let event_loop = EventLoop::new();
    /// let config = Config::from_file_or_default("config.toml");
    /// let backend = Dx12Backend::new(&event_loop, &config);
    /// ```
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        // 从配置获取窗口参数
        let width = config.window.width;
        let height = config.window.height;

        // 创建窗口
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(format!("{} [{}]", config.window.title, config.graphics.backend.name()))
                .with_inner_size(LogicalSize::new(width, height))
                .with_resizable(config.window.resizable)
                .build(event_loop)
                .expect("Failed to create window")
        );

        unsafe {
            // 1. 启用调试层（仅 Debug 模式）
            #[cfg(debug_assertions)]
            {
                let mut debug: Option<ID3D12Debug> = None;
                if let Ok(_) = D3D12GetDebugInterface(&mut debug) {
                    let debug = debug.unwrap();
                    debug.EnableDebugLayer();
                    debug!("DX12 Debug Layer enabled");
                } else {
                    warn!("Failed to enable DX12 Debug Layer");
                }
            }

            // 2. 创建 DXGI 工厂
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG).unwrap();

            // 3. 创建 D3D12 设备
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device)
                .expect("Failed to create D3D12 Device");
            let device = device.unwrap();

            #[cfg(debug_assertions)]
            debug!("D3D12 Device created successfully");

            // 4. 创建命令队列
            let queue_desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                ..Default::default()
            };
            let command_queue: ID3D12CommandQueue = device.CreateCommandQueue(&queue_desc).unwrap();

            // 5. 创建交换链
            // 从 winit 0.29 获取 HWND（使用 raw_window_handle）
            let window_handle = window.window_handle().expect("Failed to get window handle");
            let hwnd = match window_handle.as_raw() {
                RawWindowHandle::Win32(win32_handle) => {
                    windows::Win32::Foundation::HWND(win32_handle.hwnd.get() as *mut core::ffi::c_void)
                }
                _ => panic!("Expected Win32 window handle on Windows platform"),
            };
            let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    ..Default::default()
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                ..Default::default()
            };

            let swap_chain: IDXGISwapChain1 = factory
                .CreateSwapChainForHwnd(&command_queue, hwnd, &swap_chain_desc, None, None)
                .expect("Failed to create swap chain");
            let swap_chain: IDXGISwapChain3 = swap_chain.cast()
                .expect("Failed to cast swap chain to IDXGISwapChain3");

            #[cfg(debug_assertions)]
            info!(width, height, buffers = 2, "Swap chain created");

            // 6. 创建描述符堆
            let rtv_heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                NumDescriptors: 2,
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };
            let rtv_heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&rtv_heap_desc).unwrap();
            let rtv_descriptor_size = device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;

            // 7. 创建渲染目标视图（RTV）
            let rtv_handle = rtv_heap.GetCPUDescriptorHandleForHeapStart();
            for i in 0..2 {
                let surface: ID3D12Resource = swap_chain.GetBuffer(i).unwrap();
                let handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + (i as usize * rtv_descriptor_size),
                };
                device.CreateRenderTargetView(&surface, None, handle);
            }

            // 8. 创建同步对象
            let frame_index = swap_chain.GetCurrentBackBufferIndex() as usize;
            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE)
                .expect("Failed to create fence");
            let fence_value = 1;
            let fence_event = windows::Win32::System::Threading::CreateEventA(None, false, false, None)
                .expect("Failed to create fence event");

            #[cfg(debug_assertions)]
            debug!("Synchronization objects created");

            #[cfg(debug_assertions)]
            info!("DX12 Backend initialization complete");

            Self {
                device,
                command_queue,
                swap_chain,
                rtv_heap,
                rtv_descriptor_size,
                frame_index,
                fence,
                fence_value,
                fence_event,
                window,
                width,
                height,
            }
        }
    }
}

impl GraphicsBackend for Dx12Backend {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        Dx12Backend::new(event_loop, config)
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn backend_name(&self) -> &str {
        "DirectX 12"
    }
}
