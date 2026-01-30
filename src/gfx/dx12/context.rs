//! DirectX 12 鍥惧舰鍚庣瀹炵幇
//!
//! 鏈ā鍧楁彁渚涗簡鍩轰簬 DirectX 12 API 鐨勫浘褰㈠悗绔疄鐜般€?
//! DirectX 12 鏄?Microsoft 涓?Windows 骞冲彴寮€鍙戠殑楂樻€ц兘鍥惧舰 API锛?
//! 鎻愪緵浜嗗 GPU 鐨勫簳灞傛帶鍒惰兘鍔涖€?
//!
//! # 涓昏缁勪欢
//!
//! - `Dx12Context`锛欴irectX 12 鍚庣鐨勪富瑕佺粨鏋勪綋锛屽皝瑁呬簡璁惧銆佸懡浠ら槦鍒椼€佷氦鎹㈤摼绛夋牳蹇冭祫婧?
//!
//! # 鍒濆鍖栨祦绋?
//!
//! 1. 鍚敤璋冭瘯灞傦紙Debug 妯″紡锛?
//! 2. 鍒涘缓 DXGI 宸ュ巶
//! 3. 鍒涘缓 D3D12 璁惧
//! 4. 鍒涘缓鍛戒护闃熷垪
//! 5. 鍒涘缓浜ゆ崲閾?
//! 6. 鍒涘缓鎻忚堪绗﹀爢鍜屾覆鏌撶洰鏍囪鍥?
//! 7. 鍒涘缓鍚屾瀵硅薄锛團ence锛?

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

/// DirectX 12 鍥惧舰鍚庣
///
/// 灏佽浜?DirectX 12 鍥惧舰 API 鐨勬牳蹇冭祫婧愬拰鍔熻兘銆?
/// 鍖呮嫭璁惧銆佸懡浠ら槦鍒椼€佷氦鎹㈤摼銆佹弿杩扮鍫嗐€佸悓姝ュ璞＄瓑锛屼负娓叉煋鍣ㄦ彁渚涘簳灞傛敮鎸併€?
///
/// # 瀛楁璇存槑
///
/// - `device`锛欴3D12 璁惧锛岀敤浜庡垱寤哄拰绠＄悊 GPU 璧勬簮
/// - `command_queue`锛氬懡浠ら槦鍒楋紝鐢ㄤ簬鎻愪氦娓叉煋鍛戒护鍒?GPU
/// - `swap_chain`锛氫氦鎹㈤摼锛岀鐞嗗墠鍚庣紦鍐插尯
/// - `rtv_heap`锛氭覆鏌撶洰鏍囪鍥撅紙RTV锛夋弿杩扮鍫?
/// - `rtv_descriptor_size`锛歊TV 鎻忚堪绗︾殑澶у皬锛堝瓧鑺傦級
/// - `frame_index`锛氬綋鍓嶅抚鍦ㄤ氦鎹㈤摼涓殑绱㈠紩
/// - `fence`锛氭爡鏍忓璞★紝鐢ㄤ簬 CPU-GPU 鍚屾
/// - `fence_value`锛氬綋鍓嶆爡鏍忓€?
/// - `fence_event`锛氭爡鏍忎簨浠跺彞鏌?
/// - `window`锛氱獥鍙ｅ紩鐢?
/// - `width`锛氱獥鍙ｅ搴?
/// - `height`锛氱獥鍙ｉ珮搴?
pub struct Dx12Context {
    /// D3D12 璁惧
    pub device: ID3D12Device,
    /// 鍛戒护闃熷垪
    pub command_queue: ID3D12CommandQueue,
    /// 浜ゆ崲閾?
    pub swap_chain: IDXGISwapChain3,
    /// 娓叉煋鐩爣瑙嗗浘鎻忚堪绗﹀爢
    pub rtv_heap: ID3D12DescriptorHeap,
    /// RTV 鎻忚堪绗﹀ぇ灏?
    pub rtv_descriptor_size: usize,
    /// 褰撳墠甯х储寮?
    pub frame_index: usize,
    /// 鍚屾鏍呮爮
    pub fence: ID3D12Fence,
    /// 鏍呮爮鍊?
    pub fence_value: u64,
    /// 鏍呮爮浜嬩欢鍙ユ焺
    pub fence_event: windows::Win32::Foundation::HANDLE,
    /// 绐楀彛寮曠敤
    pub window: Arc<Window>,
    /// 绐楀彛瀹藉害
    pub width: u32,
    /// 绐楀彛楂樺害
    pub height: u32,
}

// 涓轰簡鍦ㄥ绾跨▼鐜涓娇鐢紝闇€瑕佸疄鐜?Send 鍜?Sync
// DirectX 12 鐨勫璞℃槸绾跨▼瀹夊叏鐨?
unsafe impl Send for Dx12Context {}
unsafe impl Sync for Dx12Context {}

impl Dx12Context {
    /// 鍒涘缓鏂扮殑 DirectX 12 鍚庣
    ///
    /// 鍒濆鍖?DirectX 12 鐨勬墍鏈夋牳蹇冪粍浠讹紝鍖呮嫭璁惧銆佸懡浠ら槦鍒椼€佷氦鎹㈤摼绛夈€?
    /// 鍦?Debug 妯″紡涓嬩細鍚敤璋冭瘯灞備互渚夸簬寮€鍙戞椂鐨勯敊璇鏌ャ€?
    ///
    /// # 鍙傛暟
    ///
    /// * `event_loop` - Winit 浜嬩欢寰幆鐨勫紩鐢紝鐢ㄤ簬鍒涘缓绐楀彛
    /// * `config` - 寮曟搸閰嶇疆锛岀敤浜庤缃獥鍙ｅぇ灏忋€佹爣棰樼瓑鍙傛暟
    ///
    /// # 杩斿洖鍊?
    ///
    /// 杩斿洖鍒濆鍖栧畬鎴愮殑 `Dx12Context` 瀹炰緥
    ///
    /// # Panics
    ///
    /// 濡傛灉鏃犳硶鍒涘缓璁惧銆佸懡浠ら槦鍒椼€佷氦鎹㈤摼鎴栧悓姝ュ璞★紝浼?panic
    ///
    /// # 绀轰緥
    ///
    /// ```no_run
    /// use winit::event_loop::EventLoop;
    /// use crate::gfx::Dx12Context;
    /// use crate::core::Config;
    ///
    /// let event_loop = EventLoop::new();
    /// let config = Config::from_file_or_default("config.toml");
    /// let backend = Dx12Context::new(&event_loop, &config);
    /// ```
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        // 浠庨厤缃幏鍙栫獥鍙ｅ弬鏁?
        let width = config.window.width;
        let height = config.window.height;

        // 鍒涘缓绐楀彛
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(format!("{} [{}]", config.window.title, config.graphics.backend.name()))
                .with_inner_size(LogicalSize::new(width, height))
                .with_resizable(config.window.resizable)
                .build(event_loop)
                .expect("Failed to create window")
        );

        unsafe {
            // 1. 鍚敤璋冭瘯灞傦紙浠?Debug 妯″紡锛?
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

            // 2. 鍒涘缓 DXGI 宸ュ巶
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG).unwrap();

            // 3. 鍒涘缓 D3D12 璁惧
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device)
                .expect("Failed to create D3D12 Device");
            let device = device.unwrap();

            #[cfg(debug_assertions)]
            debug!("D3D12 Device created successfully");

            // 4. 鍒涘缓鍛戒护闃熷垪
            let queue_desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                ..Default::default()
            };
            let command_queue: ID3D12CommandQueue = device.CreateCommandQueue(&queue_desc).unwrap();

            // 5. 鍒涘缓浜ゆ崲閾?
            // 浠?winit 0.29 鑾峰彇 HWND锛堜娇鐢?raw_window_handle锛?
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

            // 6. 鍒涘缓鎻忚堪绗﹀爢
            let rtv_heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                NumDescriptors: 2,
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };
            let rtv_heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&rtv_heap_desc).unwrap();
            let rtv_descriptor_size = device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;

            // 7. 鍒涘缓娓叉煋鐩爣瑙嗗浘锛圧TV锛?
            let rtv_handle = rtv_heap.GetCPUDescriptorHandleForHeapStart();
            for i in 0..2 {
                let surface: ID3D12Resource = swap_chain.GetBuffer(i).unwrap();
                let handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + (i as usize * rtv_descriptor_size),
                };
                device.CreateRenderTargetView(&surface, None, handle);
            }

            // 8. 鍒涘缓鍚屾瀵硅薄
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

impl GraphicsBackend for Dx12Context {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        Dx12Context::new(event_loop, config)
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn backend_name(&self) -> &str {
        "DirectX 12"
    }
}
