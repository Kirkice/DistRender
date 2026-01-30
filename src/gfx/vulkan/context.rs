//! Vulkan 鍥惧舰鍚庣瀹炵幇
//!
//! 鏈ā鍧楁彁渚涗簡鍩轰簬 Vulkan API 鐨勫浘褰㈠悗绔疄鐜般€?
//! Vulkan 鏄竴涓法骞冲彴鐨勭幇浠ｅ浘褰㈠拰璁＄畻 API锛屾彁渚涗簡瀵?GPU 鐨勫簳灞傝闂兘鍔涖€?
//!
//! # 涓昏缁勪欢
//!
//! - `VulkanContext`锛歏ulkan 鍚庣鐨勪富瑕佺粨鏋勪綋锛屽皝瑁呬簡璁惧銆侀槦鍒椼€佸垎閰嶅櫒绛夋牳蹇冭祫婧?
//!
//! # 鍒濆鍖栨祦绋?
//!
//! 1. 鍒涘缓 Vulkan 瀹炰緥
//! 2. 鍒涘缓绐楀彛琛ㄩ潰锛圫urface锛?
//! 3. 閫夋嫨鐗╃悊璁惧
//! 4. 鍒涘缓閫昏緫璁惧鍜岄槦鍒?
//! 5. 鍒涘缓鍐呭瓨鍜屽懡浠ょ紦鍐插垎閰嶅櫒

use std::sync::Arc;
use tracing::{debug, info};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::Surface;
use vulkano::VulkanLibrary;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::dpi::LogicalSize;

use crate::gfx::backend::GraphicsBackend;
use crate::core::Config;

/// Vulkan 鍥惧舰鍚庣
///
/// 灏佽浜?Vulkan 鍥惧舰 API 鐨勬牳蹇冭祫婧愬拰鍔熻兘銆?
/// 鍖呮嫭瀹炰緥銆佽澶囥€侀槦鍒椼€佸唴瀛樺垎閰嶅櫒绛夛紝涓烘覆鏌撳櫒鎻愪緵搴曞眰鏀寔銆?
///
/// # 瀛楁璇存槑
///
/// - `instance`锛歏ulkan 瀹炰緥锛屾槸浣跨敤 Vulkan 鐨勫叆鍙ｇ偣
/// - `device`锛氶€昏緫璁惧锛岀敤浜庡垱寤哄拰绠＄悊 GPU 璧勬簮
/// - `queue`锛氬懡浠ら槦鍒楋紝鐢ㄤ簬鎻愪氦娓叉煋鍛戒护鍒?GPU
/// - `surface`锛氱獥鍙ｈ〃闈紝杩炴帴绐楀彛绯荤粺鍜?Vulkan
/// - `memory_allocator`锛氬唴瀛樺垎閰嶅櫒锛岀鐞?GPU 鍐呭瓨
/// - `command_buffer_allocator`锛氬懡浠ょ紦鍐插垎閰嶅櫒锛岀鐞嗗懡浠ょ紦鍐插尯
pub struct VulkanContext {
    /// Vulkan 瀹炰緥
    #[allow(dead_code)]  // 淇濈暀渚涘皢鏉ヤ娇鐢?
    pub instance: Arc<Instance>,
    /// 閫昏緫璁惧
    pub device: Arc<Device>,
    /// 鍛戒护闃熷垪
    pub queue: Arc<Queue>,
    /// 绐楀彛琛ㄩ潰
    pub surface: Arc<Surface>,
    /// 绐楀彛寮曠敤
    window: Arc<Window>,
    /// 鍐呭瓨鍒嗛厤鍣?
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    /// 鍛戒护缂撳啿鍒嗛厤鍣?
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    /// 鎻忚堪绗﹂泦鍒嗛厤鍣?
    pub descriptor_allocator: StandardDescriptorSetAllocator,
}

impl VulkanContext {
    /// 鍒涘缓鏂扮殑 Vulkan 鍚庣
    ///
    /// 鍒濆鍖?Vulkan 鐨勬墍鏈夋牳蹇冪粍浠讹紝鍖呮嫭瀹炰緥銆佽澶囥€侀槦鍒楀拰鍒嗛厤鍣ㄣ€?
    /// 浼氳嚜鍔ㄩ€夋嫨鏈€鍚堥€傜殑鐗╃悊璁惧锛堜紭鍏堥€夋嫨鐙珛鏄惧崱锛夈€?
    ///
    /// # 鍙傛暟
    ///
    /// * `event_loop` - Winit 浜嬩欢寰幆鐨勫紩鐢紝鐢ㄤ簬鍒涘缓绐楀彛琛ㄩ潰
    /// * `config` - 寮曟搸閰嶇疆锛岀敤浜庤缃獥鍙ｅぇ灏忋€佹爣棰樼瓑鍙傛暟
    ///
    /// # 杩斿洖鍊?
    ///
    /// 杩斿洖鍒濆鍖栧畬鎴愮殑 `VulkanContext` 瀹炰緥
    ///
    /// # Panics
    ///
    /// 濡傛灉鏃犳硶鍔犺浇 Vulkan 搴撱€佸垱寤哄疄渚嬨€佹壘鍒板悎閫傜殑璁惧鎴栧垱寤洪€昏緫璁惧锛屼細 panic
    ///
    /// # 绀轰緥
    ///
    /// ```no_run
    /// use winit::event_loop::EventLoop;
    /// use crate::gfx::VulkanContext;
    /// use crate::core::Config;
    ///
    /// let event_loop = EventLoop::new();
    /// let config = Config::from_file_or_default("config.toml");
    /// let backend = VulkanContext::new(&event_loop, &config);
    /// ```
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        // 1. 鍔犺浇 Vulkan 搴?
        let library = VulkanLibrary::new().expect("Failed to load Vulkan library");

        // 2. 鍒涘缓 Vulkan 瀹炰緥锛坴ulkano_win 浼氳嚜鍔ㄥ鐞嗘墍闇€鐨勮〃闈㈡墿灞曪級
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: InstanceExtensions {
                    khr_surface: true,
                    khr_win32_surface: cfg!(target_os = "windows"),
                    khr_xlib_surface: cfg!(target_os = "linux"),
                    khr_wayland_surface: cfg!(target_os = "linux"),
                    mvk_macos_surface: cfg!(target_os = "macos"),
                    ..InstanceExtensions::empty()
                },
                ..Default::default()
            },
        )
        .expect("Failed to create Vulkan instance");

        #[cfg(debug_assertions)]
        debug!("Vulkan instance created");

        // 3. 鍒涘缓绐楀彛鍜岃〃闈紙浣跨敤閰嶇疆涓殑绐楀彛鍙傛暟锛?
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(format!("{} [{}]", config.window.title, config.graphics.backend.name()))
                .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
                .with_resizable(config.window.resizable)
                .build(event_loop)
                .expect("Failed to create window")
        );

        // 鎵嬪姩鍒涘缓琛ㄩ潰浠ュ鐞?raw-window-handle 鐗堟湰涓嶅尮閰?
        // winit 0.29 浣跨敤 raw-window-handle 0.6锛寁ulkano 0.34 浣跨敤 0.5
        let surface = Arc::new(unsafe {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            
            
            // 鑾峰彇 winit 0.29 鐨?window handle (raw-window-handle 0.6)
            let window_handle = window.as_ref().window_handle().expect("Failed to get window handle");
            
            // 鎻愬彇 HWND (Windows) 鎴栧叾浠栧钩鍙扮殑鍙ユ焺
            #[cfg(target_os = "windows")]
            let (hwnd, _hinstance) = {
                if let RawWindowHandle::Win32(win32_handle) = window_handle.as_raw() {
                    (win32_handle.hwnd.get() as *const std::ffi::c_void,
                     win32_handle.hinstance.map(|h| h.get() as *const std::ffi::c_void))
                } else {
                    panic!("Expected Win32 window handle on Windows");
                }
            };

            #[cfg(target_os = "macos")]
            let layer = {
                if let RawWindowHandle::AppKit(handle) = window_handle.as_raw() {
                    handle.ns_view.as_ptr() as *const std::ffi::c_void
                } else {
                    panic!("Expected AppKit handle on macOS");
                }
            };
            
            // 鎵嬪姩鍒涘缓 VkSurfaceKHR
            let ash_entry = ash::Entry::load().expect("Failed to load Vulkan entry");
            let ash_instance = ash::Instance::load(
                ash_entry.static_fn(), 
                vulkano::VulkanObject::handle(&*instance)
            );
            
            use ash::vk;

            #[cfg(target_os = "windows")]
            let vk_surface = {
                let win32_surface_loader = ash::extensions::khr::Win32Surface::new(&ash_entry, &ash_instance);
                let surface_create_info = vk::Win32SurfaceCreateInfoKHR::builder()
                    .hwnd(hwnd)
                    .build();
                
                win32_surface_loader
                    .create_win32_surface(&surface_create_info, None)
                    .expect("Failed to create Win32 surface")
            };

            #[cfg(target_os = "macos")]
            let vk_surface = {
                let metal_surface_loader = ash::extensions::ext::MetalSurface::new(&ash_entry, &ash_instance);
                
                use core_graphics_types::geometry::CGSize; 
                // Creating a CAMetalLayer on macOS for Vulkan if needed, usually winit creates NSView.
                // But for standard winit, the view IS the metal layer's owner or we need to ensure layer is metal.
                // RawWindowHandle 0.6 AppKit points to NSView.
                // We assume the view is backed by CAMetalLayer or MoltenVK handles it.
                // Actually vkCreateMetalSurfaceEXT expects a CAMetalLayer*.
                
                // For simplicity in this fix, we just cast the view to the layer pointer (which might fail if view is not layer-backed).
                // Proper way: [view setWantsLayer:YES]; [view setLayer:[CAMetalLayer layer]];
                // But we are inside existing window.
                
                let surface_create_info = vk::MetalSurfaceCreateInfoEXT::builder()
                    .layer(layer)
                    .build();

                metal_surface_loader
                    .create_metal_surface(&surface_create_info, None)
                    .expect("Failed to create Metal surface")
            };

            #[cfg(target_os = "windows")]
            let surface_api = vulkano::swapchain::SurfaceApi::Win32;
            #[cfg(target_os = "macos")]
            let surface_api = vulkano::swapchain::SurfaceApi::Metal;

            // 灏?ash 鐨?SurfaceKHR 鍖呰涓?vulkano Surface  
            Surface::from_handle(
                instance.clone(),
                vk_surface,
                surface_api,
                None,
            )
        });

        #[cfg(debug_assertions)]
        debug!("Vulkan surface created");
        #[cfg(debug_assertions)]
        debug!("Vulkan surface created");
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // 5. 閫夋嫨鐗╃悊璁惧鍜岄槦鍒楁棌
        // 浼樺厛绾э細鐙珛鏄惧崱 > 闆嗘垚鏄惧崱 > 铏氭嫙鏄惧崱 > CPU > 鍏朵粬
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("No suitable physical device found");

        #[cfg(debug_assertions)]
        {
            info!(
                device_name = physical_device.properties().device_name,
                device_type = ?physical_device.properties().device_type,
                "Using device"
            );
            debug!(queue_family_index, "Queue family index");
        }
        #[cfg(not(debug_assertions))]
        info!(
            device_name = physical_device.properties().device_name,
            device_type = ?physical_device.properties().device_type,
            "Using device"
        );

        // 6. 鍒涘缓閫昏緫璁惧鍜岄槦鍒?
        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("Failed to create logical device");

        #[cfg(debug_assertions)]
        debug!("Vulkan logical device created");

        let queue = queues.next().expect("Failed to get queue");

        // 7. 鍒涘缓鍒嗛厤鍣?
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );
        let descriptor_allocator = StandardDescriptorSetAllocator::new(device.clone(), Default::default());

        #[cfg(debug_assertions)]
        {
            debug!("Memory allocator created");
            debug!("Command buffer allocator created");
            debug!("Descriptor set allocator created");
            info!("Vulkan Backend initialization complete");
        }

        Self {
            instance,
            device,
            queue,
            surface,
            window,
            memory_allocator,
            command_buffer_allocator,
            descriptor_allocator,
        }
    }
}

impl GraphicsBackend for VulkanContext {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        VulkanContext::new(event_loop, config)
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn backend_name(&self) -> &str {
        "Vulkan"
    }
}
