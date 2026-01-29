//! Vulkan 图形后端实现
//!
//! 本模块提供了基于 Vulkan API 的图形后端实现。
//! Vulkan 是一个跨平台的现代图形和计算 API，提供了对 GPU 的底层访问能力。
//!
//! # 主要组件
//!
//! - `VulkanBackend`：Vulkan 后端的主要结构体，封装了设备、队列、分配器等核心资源
//!
//! # 初始化流程
//!
//! 1. 创建 Vulkan 实例
//! 2. 创建窗口表面（Surface）
//! 3. 选择物理设备
//! 4. 创建逻辑设备和队列
//! 5. 创建内存和命令缓冲分配器

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

/// Vulkan 图形后端
///
/// 封装了 Vulkan 图形 API 的核心资源和功能。
/// 包括实例、设备、队列、内存分配器等，为渲染器提供底层支持。
///
/// # 字段说明
///
/// - `instance`：Vulkan 实例，是使用 Vulkan 的入口点
/// - `device`：逻辑设备，用于创建和管理 GPU 资源
/// - `queue`：命令队列，用于提交渲染命令到 GPU
/// - `surface`：窗口表面，连接窗口系统和 Vulkan
/// - `memory_allocator`：内存分配器，管理 GPU 内存
/// - `command_buffer_allocator`：命令缓冲分配器，管理命令缓冲区
pub struct VulkanBackend {
    /// Vulkan 实例
    #[allow(dead_code)]  // 保留供将来使用
    pub instance: Arc<Instance>,
    /// 逻辑设备
    pub device: Arc<Device>,
    /// 命令队列
    pub queue: Arc<Queue>,
    /// 窗口表面
    pub surface: Arc<Surface>,
    /// 窗口引用
    window: Arc<Window>,
    /// 内存分配器
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    /// 命令缓冲分配器
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    /// 描述符集分配器
    pub descriptor_allocator: StandardDescriptorSetAllocator,
}

impl VulkanBackend {
    /// 创建新的 Vulkan 后端
    ///
    /// 初始化 Vulkan 的所有核心组件，包括实例、设备、队列和分配器。
    /// 会自动选择最合适的物理设备（优先选择独立显卡）。
    ///
    /// # 参数
    ///
    /// * `event_loop` - Winit 事件循环的引用，用于创建窗口表面
    /// * `config` - 引擎配置，用于设置窗口大小、标题等参数
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `VulkanBackend` 实例
    ///
    /// # Panics
    ///
    /// 如果无法加载 Vulkan 库、创建实例、找到合适的设备或创建逻辑设备，会 panic
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use winit::event_loop::EventLoop;
    /// use crate::gfx::VulkanBackend;
    /// use crate::core::Config;
    ///
    /// let event_loop = EventLoop::new();
    /// let config = Config::from_file_or_default("config.toml");
    /// let backend = VulkanBackend::new(&event_loop, &config);
    /// ```
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        // 1. 加载 Vulkan 库
        let library = VulkanLibrary::new().expect("Failed to load Vulkan library");

        // 2. 创建 Vulkan 实例（vulkano_win 会自动处理所需的表面扩展）
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

        // 3. 创建窗口和表面（使用配置中的窗口参数）
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(&config.window.title)
                .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
                .with_resizable(config.window.resizable)
                .build(event_loop)
                .expect("Failed to create window")
        );

        // 手动创建表面以处理 raw-window-handle 版本不匹配
        // winit 0.29 使用 raw-window-handle 0.6，vulkano 0.34 使用 0.5
        let surface = Arc::new(unsafe {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            
            
            // 获取 winit 0.29 的 window handle (raw-window-handle 0.6)
            let window_handle = window.as_ref().window_handle().expect("Failed to get window handle");
            
            // 提取 HWND (Windows) 或其他平台的句柄
            #[cfg(target_os = "windows")]
            let (hwnd, _hinstance) = {
                if let RawWindowHandle::Win32(win32_handle) = window_handle.as_raw() {
                    (win32_handle.hwnd.get() as *const std::ffi::c_void,
                     win32_handle.hinstance.map(|h| h.get() as *const std::ffi::c_void))
                } else {
                    panic!("Expected Win32 window handle on Windows");
                }
            };
            
            // 手动创建 VkWin32SurfaceKHR
            let ash_entry = ash::Entry::load().expect("Failed to load Vulkan entry");
            let ash_instance = ash::Instance::load(
                ash_entry.static_fn(), 
                vulkano::VulkanObject::handle(&*instance)
            );
            
            use ash::vk;
            let win32_surface_loader = ash::extensions::khr::Win32Surface::new(&ash_entry, &ash_instance);
            let surface_create_info = vk::Win32SurfaceCreateInfoKHR::builder()
                .hwnd(hwnd)
                .build();
            
            let vk_surface = win32_surface_loader
                .create_win32_surface(&surface_create_info, None)
                .expect("Failed to create Win32 surface");
            
            // 将 ash 的 SurfaceKHR 包装为 vulkano Surface  
            Surface::from_handle(
                instance.clone(),
                vk_surface,
                vulkano::swapchain::SurfaceApi::Win32,
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

        // 5. 选择物理设备和队列族
        // 优先级：独立显卡 > 集成显卡 > 虚拟显卡 > CPU > 其他
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

        // 6. 创建逻辑设备和队列
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

        // 7. 创建分配器
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

impl GraphicsBackend for VulkanBackend {
    fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        VulkanBackend::new(event_loop, config)
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn backend_name(&self) -> &str {
        "Vulkan"
    }
}
