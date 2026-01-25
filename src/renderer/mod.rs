use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::{VulkanLibrary};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::renderer::vertex::MyVertex;
use crate::renderer::shaders::{vs, fs};

pub mod vertex;
pub mod shaders;

/// 渲染器结构体，持有所有Vulkan相关的资源对象
pub struct Renderer {
    device: Arc<Device>, // 逻辑设备，主要用于创建资源（Buffer, Image等）
    queue: Arc<Queue>,   // 命令队列，用于提交指令给GPU执行
    surface: Arc<Surface>, // 渲染表面，用于连接Vulkan和窗口系统
    swapchain: Arc<Swapchain>, // 交换链，管理一系列用于展示的图像
    render_pass: Arc<RenderPass>, // 渲染通道，定义了渲染过程的输入输出附件
    pipeline: Arc<GraphicsPipeline>, // 图形管线，包含Shader和渲染状态配置
    framebuffers: Vec<Arc<Framebuffer>>, // 帧缓冲，绑定后的图像资源，对应RenderPass的附件
    vertex_buffer: Arc<CpuAccessibleBuffer<[MyVertex]>>, // 顶点缓冲区
    command_buffer_allocator: StandardCommandBufferAllocator, // 命令缓冲分配器
    viewport: Viewport, // 视口，定义裁剪空间到屏幕空间的映射
    recreate_swapchain: bool, // 标记是否需要重建交换链（例如窗口调整大小时）
    previous_frame_end: Option<Box<dyn GpuFuture>>, // 上一帧的GPU执行将来对象，用于同步
}

impl Renderer {
    /// 初始化渲染器
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // 1. 加载Vulkan库
        // VulkanLibrary::new(): 加载系统的vulkan-1.dll或libvulkan.so
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        
        // 获取Winit窗口系统需要的Vulkan扩展
        let required_extensions = vulkano_win::required_extensions(&library);

        // 2. 创建Vulkan实例 (Instance)
        // Instance是Vulkan API的入口，用于初始化Vulkan库
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enumerate_portability: true, // 允许枚举可移植性扩展（针对macOS等通过MoltenVK支持的平台）
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        // 3. 创建窗口和Surface
        // 使用winit创建窗口，并创建一个关联的Vulkan Surface
        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        // 定义我们需要启用的设备扩展，这里主要是KHR_SWAPCHAIN，用于屏幕呈现
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // 4. 选择物理设备 (Physical Device) 和 队列族 (Queue Family)
        // 遍历所有可用的物理设备（显卡），寻找符合要求的设备
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                // 寻找一个支持图形操作且支持Surface呈现的队列族
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.graphics && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                // 优先选择独立显卡
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("no suitable physical device found");

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type
        );

        // 5. 创建逻辑设备 (Device) 和 队列 (Queue)
        // Device::new(): 创建逻辑设备连接
        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                // 请求创建一个图形队列
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("failed to create device");

        // 获取创建的第一个队列
        let queue = queues.next().unwrap();

        // 6. 创建交换链 (Swapchain) 和 图像 (Images)
        // 交换链是图形呈现的核心，负责管理用于显示的图像队列
        let (swapchain, images) = {
            // 获取Surface的能力（如最小/最大图像数量，支持的变换等）
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .expect("failed to get surface capabilities");

            // 获取支持的图像格式，选择列表中的第一个作为格式
            let image_format = device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .expect("failed to get surface formats")[0]
                .0;

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

            // Swapchain::new(): 创建交换链
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2), // 至少请求2张图（双缓冲）
                    image_format: Some(image_format),
                    image_extent: window.inner_size().into(), // 图像大小跟随窗口大小
                    image_usage: ImageUsage {
                        color_attachment: true, // 图像将作为颜色附件被渲染
                        ..ImageUsage::empty()
                    },
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        // 创建内存分配器，用于分配Buffer和Image需要的内存
        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

        // 7. 创建顶点缓冲区 (Vertex Buffer)
        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [0.0, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.25],
        };

        // CpuAccessibleBuffer::from_iter(): 创建一个CPU可访问的Buffer，并上传数据
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &memory_allocator,
            BufferUsage {
                vertex_buffer: true, // 标记该缓冲区仅用于存放顶点数据
                ..BufferUsage::empty()
            },
            false,
            vec![vertex1, vertex2, vertex3],
        )
        .unwrap();

        // 加载Shader模块
        let vs = vs::load(device.clone()).unwrap();
        let fs = fs::load(device.clone()).unwrap();

        // 8. 创建渲染通道 (Render Pass)
        // 描述了渲染过程中附件(Attachments)的使用方式
        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear, // 渲染开始时清除内容
                    store: Store, // 渲染结束时保存内容
                    format: swapchain.image_format(),
                    samples: 1, // 不使用多重采样
                }
            },
            pass: {
                color: [color], // 子通道使用color附件作为颜色输出
                depth_stencil: {} // 无深度模板附件
            }
        )
        .unwrap();

        // 9. 创建图形管线 (Graphics Pipeline)
        // 将Shader、顶点输入、视口、光栅化状态等绑定在一起
        let pipeline = {
            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::start()
                // 定义顶点输入状态，使用MyVertex结构体定义
                .vertex_input_state(BuffersDefinition::new().vertex::<MyVertex>())
                // 绑定顶点着色器和入口函数
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                // 定义图元组装（默认三角形）
                .input_assembly_state(InputAssemblyState::new())
                // 定义视口状态
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                // 绑定片元着色器和入口函数
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                // 绑定RenderPass和Subpass
                .render_pass(subpass)
                // 构建Pipeline对象
                .build(device.clone())
                .unwrap()
        };

        // 初始化视口
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        // 10. 创建帧缓冲 (Framebuffers)
        // 将Swapchain的图像Image View绑定到RenderPass上
        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

        // 创建命令缓冲分配器
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        // 初始化上一帧结束的Future，初始值为"现在"
        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Self {
            device,
            queue,
            surface,
            swapchain,
            render_pass,
            pipeline,
            framebuffers,
            vertex_buffer,
            command_buffer_allocator,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
        }
    }

    /// 获取窗口引用
    pub fn window(&self) -> &Window {
        self.surface.object().unwrap().downcast_ref::<Window>().unwrap()
    }

    /// 处理窗口大小改变事件
    pub fn resize(&mut self) {
        self.recreate_swapchain = true;
    }

    /// 绘制一帧
    pub fn draw(&mut self) {
        let window = self.window();
        let dimensions = window.inner_size();
        if dimensions.width == 0 || dimensions.height == 0 {
            return;
        }

        // 清理上一帧已经完成的资源
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // 如果需要重建交换链（例如窗口大小改变）
        if self.recreate_swapchain {
            // swapchain.recreate(): 重建交换链以适应新的窗口尺寸
            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = new_swapchain;
            // 因为交换链图像变了，所以帧缓冲也需要重新创建
            self.framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_pass.clone(),
                &mut self.viewport,
            );
            self.recreate_swapchain = false;
        }

        // 1. 获取下一张可用的交换链图像
        // acquire_next_image(): 请求一张可以用来写入的图像索引
        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        // 2. 开始录制命令缓冲区
        // AutoCommandBufferBuilder::primary(): 创建一个主要的命令构建器
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            // 开始Render Pass
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())], // 清除颜色为黑色
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(), // 绑定对应的帧缓冲
                    )
                },
                SubpassContents::Inline,
            )
            .unwrap()
            // 设置视口动态状态
            .set_viewport(0, [self.viewport.clone()].into_iter())
            // 绑定图形管线
            .bind_pipeline_graphics(self.pipeline.clone())
            // 绑定顶点缓冲区
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            // 执行绘制命令
            // draw(vertex_count, instance_count, first_vertex, first_instance)
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap()
            // 结束Render Pass
            .end_render_pass()
            .unwrap();

        // 结束录制，生成CommandBuffer
        let command_buffer = builder.build().unwrap();

        // 3. 提交命令并在绘制完成后呈现
        // 构建Future链: 上一帧结束 -> 等待图像获取 -> 执行命令 -> 呈现图像 -> 信号通知GPU完成
        let future = self.previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future) // 等待acquire_next_image完成
            .then_execute(self.queue.clone(), command_buffer) // 提交命令缓冲区到队列执行
            .unwrap()
            .then_swapchain_present( // 执行完成后呈现图像
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush(); // 刷新并等待 fences 信号

        match future {
            Ok(future) => {
                // 更新上一帧的Future为此帧的Future
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }
}

/// 辅助函数：根据窗口大小和交换链图像创建帧缓冲
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            // Framebuffer::new(): 将Image View绑定到Render Pass的附件上
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
