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

use crate::renderer::vertex::MyVertex;
use crate::renderer::shaders::{vs, fs};
use crate::gfx::{GraphicsBackend, VulkanBackend as GfxDevice};
use crate::core::Config;
use crate::core::math::{Vector2, Vector3};

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
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config) -> Self {
        let gfx = GfxDevice::new(event_loop, config);

        let (swapchain, images) = {
            let surface_capabilities = gfx.device
                .physical_device()
                .surface_capabilities(&gfx.surface, Default::default())
                .expect("failed to get surface capabilities");

            let image_format = gfx.device
                .physical_device()
                .surface_formats(&gfx.surface, Default::default())
                .expect("failed to get surface formats")[0]
                .0;

            let window = gfx.window();

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
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .expect("Failed to create swapchain")
        };

        #[cfg(debug_assertions)]
        info!(
            width = gfx.window().inner_size().width,
            height = gfx.window().inner_size().height,
            images = images.len(),
            "Swapchain created"
        );

        // 使用数学库类型创建顶点数据
        let vertex1 = MyVertex::from_vectors(
            Vector2::new(0.0, 0.5),
            Vector3::new(1.0, 0.0, 0.0)  // 红色
        );
        let vertex2 = MyVertex::from_vectors(
            Vector2::new(0.5, -0.5),
            Vector3::new(0.0, 1.0, 0.0)  // 绿色
        );
        let vertex3 = MyVertex::from_vectors(
            Vector2::new(-0.5, -0.5),
            Vector3::new(0.0, 0.0, 1.0)  // 蓝色
        );

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &gfx.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vec![vertex1, vertex2, vertex3],
        )
        .unwrap();

        let vs = vs::load(gfx.device.clone()).expect("Failed to load vertex shader");
        let fs = fs::load(gfx.device.clone()).expect("Failed to load fragment shader");

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
        .expect("Failed to create render pass");

        #[cfg(debug_assertions)]
        debug!("Render pass created");

        let pipeline = {
            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<MyVertex>())
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .input_assembly_state(InputAssemblyState::new())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .render_pass(subpass)
                .build(gfx.device.clone())
                .expect("Failed to create graphics pipeline")
        };

        #[cfg(debug_assertions)]
        debug!("Graphics pipeline created");

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

        let previous_frame_end = Some(sync::now(gfx.device.clone()).boxed());

        #[cfg(debug_assertions)]
        info!("Vulkan Renderer initialized successfully");

        Self {
            gfx,
            swapchain,
            render_pass,
            pipeline,
            framebuffers,
            vertex_buffer,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
        }
    }

    pub fn window(&self) -> &Window {
        self.gfx.window()
    }

    pub fn resize(&mut self) {
        #[cfg(debug_assertions)]
        debug!("Swapchain resize requested");

        self.recreate_swapchain = true;
    }

    pub fn draw(&mut self) {
        let window = self.window();
        let dimensions = window.inner_size();
        if dimensions.width == 0 || dimensions.height == 0 {
            return;
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

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
                    return;
                }
                Err(e) => {
                    error!("Failed to recreate swapchain: {:?}", e);
                    panic!("failed to recreate swapchain: {:?}", e);
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
            );
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
                    return;
                }
                Err(e) => {
                    error!("Failed to acquire next image: {:?}", e);
                    panic!("failed to acquire next image: {:?}", e);
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
        .expect("Failed to create command buffer builder");

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
            .expect("Failed to begin render pass")
            .set_viewport(0, [self.viewport.clone()].into_iter())
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .expect("Failed to record draw command")
            .end_render_pass()
            .expect("Failed to end render pass");

        let command_buffer = builder.build()
            .expect("Failed to build command buffer");

        #[cfg(debug_assertions)]
        trace!("Command buffer built, submitting to queue");

        let future = self.previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.gfx.queue.clone(), command_buffer)
            .expect("Failed to execute command buffer")
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
    }
}

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
