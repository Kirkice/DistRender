use std::sync::{Arc, Mutex};
use windows::{
    core::*, Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D12::*,
    Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Dxgi::*, Win32::System::Threading::*,
};
use winit::platform::windows::WindowExtWindows;
use winit::window::Window;

pub struct Dx12Backend {
    pub device: ID3D12Device,
    pub command_queue: ID3D12CommandQueue,
    pub swap_chain: IDXGISwapChain3,
    pub rtv_heap: ID3D12DescriptorHeap,
    pub rtv_descriptor_size: usize,
    pub frame_index: usize,
    pub fence: ID3D12Fence,
    pub fence_value: u64,
    pub fence_event: windows::Win32::Foundation::HANDLE,
    pub window: Arc<Window>, // Keep window alive
    pub width: u32,
    pub height: u32,
}

unsafe impl Send for Dx12Backend {}
unsafe impl Sync for Dx12Backend {}

impl Dx12Backend {
    pub fn new(window: Arc<Window>) -> Self {
        let width = window.inner_size().width;
        let height = window.inner_size().height;

        unsafe {
            // Enable Debug Layer
            #[cfg(debug_assertions)]
            {
                let mut debug: Option<ID3D12Debug> = None;
                if let Ok(_) = D3D12GetDebugInterface(&mut debug) {
                    debug.unwrap().EnableDebugLayer();
                }
            }

            // Create Factory
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG).unwrap();

            // Create Device
            let mut device: Option<ID3D12Device> = None;
            // Provide D3D_FEATURE_LEVEL_11_0 by default
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device).expect("Failed to create D3D12 Device");
            let device = device.unwrap();

            // Create Command Queue
            let queue_desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                ..Default::default()
            };
            let command_queue: ID3D12CommandQueue = device.CreateCommandQueue(&queue_desc).unwrap();

            // Create SwapChain
            let hwnd = windows::Win32::Foundation::HWND(window.hwnd() as _);
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
                .unwrap();
            let swap_chain: IDXGISwapChain3 = swap_chain.cast().unwrap();

            // Create Descriptor Heaps
            let rtv_heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                NumDescriptors: 2,
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };
            let rtv_heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&rtv_heap_desc).unwrap();
            let rtv_descriptor_size = device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;

            // Create Frame Resources (RTVs)
            let rtv_handle = rtv_heap.GetCPUDescriptorHandleForHeapStart();
            for i in 0..2 {
                let surface: ID3D12Resource = swap_chain.GetBuffer(i).unwrap();
                let handle = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + (i as usize * rtv_descriptor_size),
                };
                device.CreateRenderTargetView(&surface, None, handle);
            }

            // Create Synchronization Objects
            let frame_index = swap_chain.GetCurrentBackBufferIndex() as usize;
            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let fence_value = 1;
            let fence_event = windows::Win32::System::Threading::CreateEventA(None, false, false, None).unwrap();

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
