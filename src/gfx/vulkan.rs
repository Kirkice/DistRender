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
use tracing::{trace, debug, info, warn, error};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::Surface;
use vulkano::VulkanLibrary;
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::dpi::LogicalSize;

use super::GraphicsBackend;
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
        let required_extensions = vulkano_win::required_extensions(&library);

        // 2. 创建 Vulkan 实例
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enumerate_portability: true,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("Failed to create Vulkan instance");

        #[cfg(debug_assertions)]
        debug!("Vulkan instance created");

        // 3. 创建窗口表面（使用配置中的窗口参数）
        let surface = WindowBuilder::new()
            .with_title(&config.window.title)
            .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
            .with_resizable(config.window.resizable)
            .build_vk_surface(event_loop, instance.clone())
            .expect("Failed to create window surface");

        #[cfg(debug_assertions)]
        debug!("Vulkan surface created");

        // 4. 配置设备扩展（启用交换链支持）
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
                        q.queue_flags.graphics
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
        let descriptor_allocator = StandardDescriptorSetAllocator::new(device.clone());

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
        self.surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap()
    }

    fn backend_name(&self) -> &str {
        "Vulkan"
    }
}
