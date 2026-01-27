use std::sync::Arc;
use tracing::{trace, debug, info, warn, error};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, ImageUsage, SwapchainImage};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::renderer::vertex::{MyVertex, create_default_triangle};
use crate::renderer::shaders::{vs, fs};
use crate::renderer::resource::FrameResourcePool;
use crate::renderer::sync::{FenceManager, FenceValue};
use crate::gfx::{GraphicsBackend, VulkanBackend as GfxDevice};
use crate::core::Config;
use crate::core::error::{Result, DistRenderError, GraphicsError};

pub struct Renderer {
    gfx: GfxDevice,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[MyVertex]>>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    // 新增：帧资源管理
    frame_resource_pool: FrameResourcePool,
    // 新增：Fence同步管理
    fence_manager: FenceManager,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Result<Self> {
        let gfx = GfxDevice::new(event_loop, config);

        let (swapchain, images) = {
            let surface_capabilities = gfx.device
                .physical_device()
                .surface_capabilities(&gfx.surface, Default::default())
                .map_err(|e| DistRenderError::Graphics(
                    GraphicsError::DeviceCreation(format!("Failed to get surface capabilities: {:?}", e))
                ))?;

            let surface_formats = gfx.device
                .physical_device()
                .surface_formats(&gfx.surface, Default::default())
                .map_err(|e| DistRenderError::Graphics(
                    GraphicsError::DeviceCreation(format!("Failed to get surface formats: {:?}", e))
                ))?;

            let image_format = surface_formats.get(0)
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::DeviceCreation("No surface formats available".to_string())
                ))?
                .0;

            let window = gfx.window();

            let composite_alpha = surface_capabilities
                .supported_composite_alpha
                .iter()
                .next()
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::SwapchainError("No supported composite alpha modes".to_string())
                ))?;

            Swapchain::new(
                gfx.device.clone(),
                gfx.surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format: Some(image_format),
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage {
                        color_attachment: true,
                        ..ImageUsage::empty()
                    },
                    composite_alpha,
                    ..Default::default()
                },
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::SwapchainError(format!("Failed to create swapchain: {:?}", e))
            ))?
        };

        #[cfg(debug_assertions)]
        info!(
            width = gfx.window().inner_size().width,
            height = gfx.window().inner_size().height,
            images = images.len(),
            "Swapchain created"
        );

        // 使用公共函数创建默认三角形顶点数据
        let vertices = create_default_triangle();

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &gfx.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vertices,
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create vertex buffer: {:?}", e))
        ))?;

        let vs = vs::load(gfx.device.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ShaderCompilation(format!("Failed to load vertex shader: {:?}", e))
            ))?;
        let fs = fs::load(gfx.device.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ShaderCompilation(format!("Failed to load fragment shader: {:?}", e))
            ))?;

        #[cfg(debug_assertions)]
        debug!("Shaders loaded successfully");

        let render_pass = vulkano::single_pass_renderpass!(
            gfx.device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create render pass: {:?}", e))
        ))?;

        #[cfg(debug_assertions)]
        debug!("Render pass created");

        let pipeline = {
            let subpass = Subpass::from(render_pass.clone(), 0)
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ResourceCreation("Failed to create subpass".to_string())
                ))?;

            let vs_entry = vs.entry_point("main")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Vertex shader 'main' entry point not found".to_string())
                ))?;

            let fs_entry = fs.entry_point("main")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Fragment shader 'main' entry point not found".to_string())
                ))?;

            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<MyVertex>())
                .vertex_shader(vs_entry, ())
                .input_assembly_state(InputAssemblyState::new())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .fragment_shader(fs_entry, ())
                .render_pass(subpass)
                .build(gfx.device.clone())
                .map_err(|e| DistRenderError::Graphics(
                    GraphicsError::ResourceCreation(format!("Failed to create graphics pipeline: {:?}", e))
                ))?
        };

        #[cfg(debug_assertions)]
        debug!("Graphics pipeline created");

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport)?;

        let previous_frame_end = Some(sync::now(gfx.device.clone()).boxed());

        // 初始化帧资源池（三缓冲）
        let frame_resource_pool = FrameResourcePool::triple_buffering();

        // 初始化Fence管理器
        let fence_manager = FenceManager::new();

        #[cfg(debug_assertions)]
        info!("Vulkan Renderer initialized successfully with triple buffering");

        Ok(Self {
            gfx,
            swapchain,
            render_pass,
            pipeline,
            framebuffers,
            vertex_buffer,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
            frame_resource_pool,
            fence_manager,
        })
    }

    pub fn window(&self) -> &Window {
        self.gfx.window()
    }

    pub fn resize(&mut self) {
        #[cfg(debug_assertions)]
        debug!("Swapchain resize requested");

        self.recreate_swapchain = true;
    }

    /// 等待GPU完成所有工作（类似DistEngine的FlushCommandQueue）
    ///
    /// 这是一个阻塞操作，会等待所有提交的GPU命令完成。
    /// 通常用于清理资源或同步点。
    pub fn flush(&mut self) -> Result<()> {
        #[cfg(debug_assertions)]
        debug!("Flushing command queue...");

        // 等待previous_frame_end完成
        if let Some(ref mut future) = self.previous_frame_end {
            future.cleanup_finished();
        }

        // 等待所有帧资源完成
        let current_fence = self.fence_manager.current_value();
        self.fence_manager.wait_for_value(current_fence)?;

        // 更新所有帧资源为可用
        self.frame_resource_pool.update_availability(current_fence.value());

        #[cfg(debug_assertions)]
        debug!("Command queue flushed");

        Ok(())
    }

    pub fn draw(&mut self) -> Result<()> {
        // 获取当前帧资源信息
        let current_frame = self.frame_resource_pool.current_index();

        #[cfg(debug_assertions)]
        trace!("Drawing frame {}", current_frame);

        let window = self.window();
        let dimensions = window.inner_size();
        if dimensions.width == 0 || dimensions.height == 0 {
            return Ok(());
        }

        self.previous_frame_end.as_mut()
            .ok_or_else(|| DistRenderError::Runtime("Previous frame end not initialized".to_string()))?
            .cleanup_finished();

        if self.recreate_swapchain {
            #[cfg(debug_assertions)]
            debug!("Recreating swapchain...");

            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => {
                    #[cfg(debug_assertions)]
                    warn!("Swapchain recreation skipped: extent not supported");
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to recreate swapchain: {:?}", e);
                    return Err(DistRenderError::Graphics(
                        GraphicsError::SwapchainError(format!("Failed to recreate swapchain: {:?}", e))
                    ));
                }
            };

            #[cfg(debug_assertions)]
            debug!(
                width = dimensions.width,
                height = dimensions.height,
                images = new_images.len(),
                "Swapchain recreated"
            );

            self.swapchain = new_swapchain;
            self.framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_pass.clone(),
                &mut self.viewport,
            )?;
            self.recreate_swapchain = false;

            #[cfg(debug_assertions)]
            debug!("Framebuffers rebuilt");
        }

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => {
                    #[cfg(debug_assertions)]
                    trace!(image_index = %r.0, "Acquired swapchain image");
                    r
                }
                Err(AcquireError::OutOfDate) => {
                    #[cfg(debug_assertions)]
                    warn!("Swapchain out of date, will recreate");
                    self.recreate_swapchain = true;
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to acquire next image: {:?}", e);
                    return Err(DistRenderError::Graphics(
                        GraphicsError::CommandExecution(format!("Failed to acquire next image: {:?}", e))
                    ));
                }
            };

        if suboptimal {
            #[cfg(debug_assertions)]
            debug!("Swapchain suboptimal, will recreate next frame");
            self.recreate_swapchain = true;
        }

        #[cfg(debug_assertions)]
        trace!(image_index, "Building command buffer");

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.gfx.command_buffer_allocator,
            self.gfx.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::CommandExecution(format!("Failed to create command buffer builder: {:?}", e))
        ))?;

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 0.2, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to begin render pass: {:?}", e))
            ))?
            .set_viewport(0, [self.viewport.clone()].into_iter())
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to record draw command: {:?}", e))
            ))?
            .end_render_pass()
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to end render pass: {:?}", e))
            ))?;

        let command_buffer = builder.build()
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to build command buffer: {:?}", e))
            ))?;

        #[cfg(debug_assertions)]
        trace!("Command buffer built, submitting to queue");

        let previous_frame = self.previous_frame_end.take()
            .ok_or_else(|| DistRenderError::Runtime("Previous frame end not initialized".to_string()))?;

        let future = previous_frame
            .join(acquire_future)
            .then_execute(self.gfx.queue.clone(), command_buffer)
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to execute command buffer: {:?}", e))
            ))?
            .then_swapchain_present(
                self.gfx.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                #[cfg(debug_assertions)]
                trace!("Frame presented successfully");
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                #[cfg(debug_assertions)]
                debug!("Flush error: swapchain out of date");
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.gfx.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.gfx.device.clone()).boxed());
            }
        }

        // 更新 Fence 管理器
        let fence_value = self.fence_manager.next_value();

        #[cfg(debug_assertions)]
        trace!("Frame {} submitted with fence value {}", current_frame, fence_value.value());

        // 标记当前帧资源为使用中
        self.frame_resource_pool.current_mut().mark_in_use(fence_value.value());

        // 推进到下一帧
        self.frame_resource_pool.advance();

        Ok(())
    }
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Result<Vec<Arc<Framebuffer>>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone())
                .map_err(|e| DistRenderError::Graphics(
                    GraphicsError::ResourceCreation(format!("Failed to create image view: {:?}", e))
                ))?;
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create framebuffer: {:?}", e))
            ))
        })
        .collect::<Result<Vec<_>>>()
}
