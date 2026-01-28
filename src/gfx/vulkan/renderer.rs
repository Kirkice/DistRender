use std::sync::Arc;
use tracing::{trace, debug, info, warn, error};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, ImageUsage, SwapchainImage};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use winit::event_loop::EventLoop;
use winit::window::Window;
use bytemuck::{Pod, Zeroable};

use crate::renderer::vertex::{MyVertex, create_default_triangle, convert_geometry_vertex};
use crate::gfx::vulkan::shaders::{vs, fs};
use crate::renderer::resource::FrameResourcePool;
use crate::renderer::sync::FenceManager;
use crate::gfx::vulkan::descriptor::VulkanDescriptorManager;
use crate::gfx::{GraphicsBackend, VulkanBackend as GfxDevice};
use crate::core::{Config, SceneConfig, Matrix4};
use crate::core::error::{Result, DistRenderError, GraphicsError};
use crate::geometry::loaders::{MeshLoader, ObjLoader};
use std::path::Path;

/// Uniform Buffer Object - MVP 矩阵数据
///
/// 这个结构体会被传输到 GPU 的 uniform buffer 中。
/// 必须使用 #[repr(C)] 保证内存布局与着色器一致。
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    camera_pos: [f32; 4],
}

impl UniformBufferObject {
    fn new(model: &Matrix4, view: &Matrix4, projection: &Matrix4, light_dir: [f32;3], light_color_intensity: [f32;4], camera_pos: [f32;3]) -> Self {
        Self {
            model: *model.as_ref(),
            view: *view.as_ref(),
            projection: *projection.as_ref(),
            light_dir: [light_dir[0], light_dir[1], light_dir[2], 0.0],
            light_color: light_color_intensity,
            camera_pos: [camera_pos[0], camera_pos[1], camera_pos[2], 0.0],
        }
    }
}

pub struct Renderer {
    gfx: GfxDevice,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[MyVertex]>>,
    index_buffer: Arc<CpuAccessibleBuffer<[u32]>>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    // 新增：帧资源管理
    frame_resource_pool: FrameResourcePool,
    // 新增：Fence同步管理
    fence_manager: FenceManager,
    // 新增：描述符管理
    descriptor_manager: VulkanDescriptorManager,
    // 新增：Uniform buffer pool
    uniform_buffer_pool: CpuBufferPool<UniformBufferObject>,
    // 新增：场景配置
    scene: SceneConfig,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &SceneConfig) -> Result<Self> {
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

        // 加载 OBJ 模型文件
        let obj_path = Path::new("assets/models/sphere.obj");
        let (vertices, indices) = if obj_path.exists() {
            info!("Loading mesh from: {}", obj_path.display());
            match ObjLoader::load_from_file(obj_path) {
                Ok(mesh_data) => {
                    info!(
                        "Mesh loaded successfully: {} vertices, {} indices",
                        mesh_data.vertex_count(),
                        mesh_data.index_count()
                    );
                    // 转换 GeometryVertex 为 MyVertex
                    let verts = mesh_data
                        .vertices
                        .iter()
                        .map(|v| convert_geometry_vertex(v))
                        .collect::<Vec<_>>();
                    let inds = mesh_data.indices.clone();
                    (verts, inds)
                }
                Err(e) => {
                    warn!("Failed to load OBJ file: {}, using default triangle", e);
                    (create_default_triangle().to_vec(), vec![0, 1, 2])
                }
            }
        } else {
            warn!("OBJ file not found: {}, using default triangle", obj_path.display());
            (create_default_triangle().to_vec(), vec![0, 1, 2])
        };

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &gfx.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vertices.into_iter(),
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create vertex buffer: {:?}", e))
        ))?;

        let index_buffer = CpuAccessibleBuffer::from_iter(
            &gfx.memory_allocator,
            BufferUsage {
                index_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            indices.into_iter(),
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create index buffer: {:?}", e))
        ))?;

        info!("Index buffer created: {} indices", index_buffer.len());

        let vs_module = vs::load(gfx.device.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ShaderCompilation(format!("Failed to load vertex shader: {:?}", e))
            ))?;
        let fs_module = fs::load(gfx.device.clone())
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

            // 从 SPIR-V 加载的着色器使用 "VSMain" 和 "PSMain" 作为入口点
            // (这是 HLSL 中定义的入口点名称)
            let vs_entry = vs_module.entry_point("VSMain")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Vertex shader 'VSMain' entry point not found".to_string())
                ))?;

            let fs_entry = fs_module.entry_point("PSMain")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Fragment shader 'PSMain' entry point not found".to_string())
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

        // 初始化描述符管理器
        let descriptor_manager = VulkanDescriptorManager::new(gfx.device.clone());

        // 初始化 Uniform Buffer Pool
        let uniform_buffer_pool = CpuBufferPool::uniform_buffer(gfx.memory_allocator.clone());

        #[cfg(debug_assertions)]
        {
            info!("Vulkan Renderer initialized successfully with triple buffering");
            debug!("Descriptor manager initialized");
            info!("Uniform buffer pool created");
        }

        Ok(Self {
            gfx,
            swapchain,
            render_pass,
            pipeline,
            framebuffers,
            vertex_buffer,
            index_buffer,
            viewport,
            recreate_swapchain: false,
            previous_frame_end,
            frame_resource_pool,
            fence_manager,
            descriptor_manager,
            uniform_buffer_pool,
            scene: scene.clone(),
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

        // 计算 MVP 矩阵
        let aspect_ratio = self.viewport.dimensions[0] / self.viewport.dimensions[1];
        let model = self.scene.model.transform.to_matrix();
        let view = self.scene.camera.view_matrix();
        let projection = self.scene.camera.projection_matrix(aspect_ratio);

        // 计算灯光参数
        let light_rot = self.scene.light.transform.rotation;
        let pitch = light_rot[0].to_radians();
        let yaw = light_rot[1].to_radians();
        let dir = nalgebra::Vector3::new(
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        ).normalize();
        let light_color = self.scene.light.color;
        let intensity = self.scene.light.intensity;
        let light_col_int = [
            light_color[0] * intensity,
            light_color[1] * intensity,
            light_color[2] * intensity,
            intensity,
        ];
        let camera_pos = self.scene.camera.transform.position;
        let ubo = UniformBufferObject::new(
            &model,
            &view,
            &projection,
            [dir.x, dir.y, dir.z],
            light_col_int,
            camera_pos,
        );

        // 创建 uniform buffer
        let uniform_subbuffer = self.uniform_buffer_pool
            .from_data(ubo)
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create uniform buffer: {:?}", e))
            ))?;

        // 创建描述符集
        let layout = self.pipeline.layout().set_layouts().get(0)
            .ok_or_else(|| DistRenderError::Graphics(
                GraphicsError::ResourceCreation("Pipeline has no descriptor set layouts".to_string())
            ))?;

        let descriptor_set = PersistentDescriptorSet::new(
            &self.gfx.descriptor_allocator,
            layout.clone(),
            [WriteDescriptorSet::buffer(0, uniform_subbuffer)],
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create descriptor set: {:?}", e))
        ))?;

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
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .bind_index_buffer(self.index_buffer.clone())
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)
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
