use std::sync::Arc;
use tracing::{trace, debug, info, warn, error};
use vulkano::buffer::{Buffer, BufferUsage, BufferCreateInfo, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageUsage};
use vulkano::format::Format;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexInputState, VertexInputBindingDescription, VertexInputAttributeDescription};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode, FrontFace};
use vulkano::pipeline::graphics::depth_stencil::{DepthStencilState, DepthState};
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, ColorBlendAttachmentState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    acquire_next_image, PresentMode, Swapchain, SwapchainCreateInfo,
    SwapchainPresentInfo,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
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
use crate::component::{Camera, DirectionalLight};
use crate::core::math::Vector3;
use crate::gui::ipc::GuiStatePacket;
use std::path::Path;
use std::f32::consts::PI;

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
    vertex_buffer: Subbuffer<[MyVertex]>,
    index_buffer: Subbuffer<[u32]>,
    viewport: Viewport,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    depth_image: Arc<Image>,

    // 新增：帧资源管理
    frame_resource_pool: FrameResourcePool,
    // 新增：Fence同步管理
    fence_manager: FenceManager,
    // 新增：描述符管理
    descriptor_manager: VulkanDescriptorManager,
    // 新增：场景配置
    scene: SceneConfig,
    // 新增：相机组件
    camera: Camera,
    // 新增：方向光组件
    directional_light: DirectionalLight,
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
                .into_iter()
                .next()
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::SwapchainError("No supported composite alpha modes".to_string())
                ))?;

            Swapchain::new(
                gfx.device.clone(),
                gfx.surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    // 使用 Mailbox 模式实现三重缓冲，提供流畅的渲染
                    // 如果不支持则回退到 Immediate（无垂直同步）
                    present_mode: PresentMode::Mailbox,
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
        let obj_path = Path::new(&scene.model.path);
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

        let vertex_buffer = Buffer::from_iter(
            gfx.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices.into_iter(),
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create vertex buffer: {:?}", e))
        ))?;

        let index_buffer = Buffer::from_iter(
            gfx.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices.into_iter(),
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create index buffer: {:?}", e))
        ))?;

        info!("Index buffer created: {} indices", index_buffer.len());

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
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth: {
                    format: Format::D32_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create render pass: {:?}", e))
        ))?;

        #[cfg(debug_assertions)]
        debug!("Render pass created");

        let pipeline = {
            let vs_entry = vs.entry_point("main")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Vertex shader 'main' entry point not found".to_string())
                ))?;

            let fs_entry = fs.entry_point("main")
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ShaderCompilation("Fragment shader 'main' entry point not found".to_string())
                ))?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs_entry),
                PipelineShaderStageCreateInfo::new(fs_entry),
            ];

            let layout = PipelineLayout::new(
                gfx.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(gfx.device.clone())
                    .map_err(|e| DistRenderError::Graphics(
                        GraphicsError::ResourceCreation(format!("Failed to create pipeline layout info: {:?}", e))
                    ))?,
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create pipeline layout: {:?}", e))
            ))?;

            let subpass = Subpass::from(render_pass.clone(), 0)
                .ok_or_else(|| DistRenderError::Graphics(
                    GraphicsError::ResourceCreation("Failed to create subpass".to_string())
                ))?;

            GraphicsPipeline::new(
                gfx.device.clone(),
                None,
                vulkano::pipeline::graphics::GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some({
                        let desc = MyVertex::per_vertex();
                        let binding_desc = VertexInputBindingDescription {
                            stride: desc.stride,
                            input_rate: desc.input_rate,
                        };
                        let attr_descs: Vec<(u32, VertexInputAttributeDescription)> = desc.members.iter().enumerate().map(|(location, (_name, member))| {
                            (location as u32, VertexInputAttributeDescription {
                                binding: 0,
                                format: member.format,
                                offset: member.offset as u32,
                            })
                        }).collect();
                        
                        let mut state = VertexInputState::new().binding(0, binding_desc);
                        for (location, attr) in attr_descs {
                            state = state.attribute(location, attr);
                        }
                        state
                    }),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState {
                        cull_mode: CullMode::Back,
                        front_face: FrontFace::Clockwise,
                        ..Default::default()
                    }),
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState::simple()),
                        ..Default::default()
                    }),
                    multisample_state: Some(Default::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        1,  // 渲染通道中有 1 个 color attachment
                        ColorBlendAttachmentState::default(),
                    )),
                    dynamic_state: [vulkano::pipeline::DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.into()),
                    ..vulkano::pipeline::graphics::GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create graphics pipeline: {:?}", e))
            ))?
        };

        #[cfg(debug_assertions)]
        debug!("Graphics pipeline created");

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        // 创建深度图像
        let dimensions = images[0].extent();
        let depth_image = Image::new(
            gfx.memory_allocator.clone(),
            vulkano::image::ImageCreateInfo {
                image_type: vulkano::image::ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: [dimensions[0], dimensions[1], 1],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create depth image: {:?}", e))
        ))?;

        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), depth_image.clone(), &mut viewport)?;

        let previous_frame_end = Some(sync::now(gfx.device.clone()).boxed());

        // 初始化帧资源池（三缓冲）
        let frame_resource_pool = FrameResourcePool::triple_buffering();

        // 初始化Fence管理器
        let fence_manager = FenceManager::new();

        // 初始化描述符管理器
        let descriptor_manager = VulkanDescriptorManager::new(gfx.device.clone());

        #[cfg(debug_assertions)]
        {
            info!("Vulkan Renderer initialized successfully with triple buffering");
            debug!("Descriptor manager initialized");
        }

        // 创建相机组件（从场景配置初始化）
        let mut camera = Camera::main_camera();
        camera.set_position(Vector3::new(
            scene.camera.transform.position[0],
            scene.camera.transform.position[1],
            scene.camera.transform.position[2],
        ));
        let aspect_ratio = viewport.extent[0] / viewport.extent[1];
        camera.set_lens(
            scene.camera.fov * PI / 180.0,
            aspect_ratio,
            scene.camera.near_clip,
            scene.camera.far_clip,
        );

        // 如果有旋转，使用 look_at 设置相机朝向
        let pitch = scene.camera.transform.rotation[0] * PI / 180.0;
        let yaw = scene.camera.transform.rotation[1] * PI / 180.0;
        let forward = Vector3::new(
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let target = camera.position() + forward;
        camera.look_at(camera.position(), target, Vector3::new(0.0, 1.0, 0.0));

        info!("Camera component initialized at position {:?}", camera.position());

        // 初始化方向光组件
        let directional_light = scene.light.to_directional_light("MainLight");
        info!(
            "DirectionalLight component initialized: color={:?}, intensity={}, direction={:?}",
            directional_light.color.to_array(),
            directional_light.intensity,
            directional_light.direction
        );

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
            depth_image,
            frame_resource_pool,
            fence_manager,
            descriptor_manager,
            scene: scene.clone(),
            camera,
            directional_light,
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
            debug!("Recreating swapchain, waiting for current frame...");

            // 只等待当前帧完成，不使用 flush() 避免阻塞
            if let Some(ref mut future) = self.previous_frame_end {
                future.cleanup_finished();
            }

            let result = self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            });

            let (new_swapchain, new_images) = match result {
                Ok(r) => r,
                Err(e) => {
                    // Check if it's an ImageExtentNotSupported error
                    let err_string = format!("{:?}", e);
                    if err_string.contains("ImageExtentNotSupported") {
                        #[cfg(debug_assertions)]
                        warn!("Swapchain recreation skipped: extent not supported");
                        return Ok(());
                    }
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

            // 重新创建深度图像
            let new_dimensions = new_images[0].extent();
            self.depth_image = Image::new(
                self.gfx.memory_allocator.clone(),
                vulkano::image::ImageCreateInfo {
                    image_type: vulkano::image::ImageType::Dim2d,
                    format: Format::D32_SFLOAT,
                    extent: [new_dimensions[0], new_dimensions[1], 1],
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create depth image: {:?}", e))
            ))?;

            self.framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_pass.clone(),
                self.depth_image.clone(),
                &mut self.viewport,
            )?;
            self.recreate_swapchain = false;

            // 重置 previous_frame_end 以确保干净的同步状态
            self.previous_frame_end = Some(sync::now(self.gfx.device.clone()).boxed());

            #[cfg(debug_assertions)]
            debug!("Framebuffers rebuilt, synchronization reset");
        }

        let acquire_result = acquire_next_image(self.swapchain.clone(), None);

        let (image_index, suboptimal, acquire_future) =
            match acquire_result {
                Ok(r) => {
                    #[cfg(debug_assertions)]
                    trace!(image_index = %r.0, "Acquired swapchain image");
                    r
                }
                Err(e) => {
                    // Check if it's an OutOfDate error
                    let err_string = format!("{:?}", e);
                    if err_string.contains("OutOfDate") {
                        #[cfg(debug_assertions)]
                        warn!("Swapchain out of date, will recreate");
                        self.recreate_swapchain = true;
                        return Ok(());
                    }
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

        // 更新相机的宽高比（如果窗口大小改变）
        let aspect_ratio = self.viewport.extent[0] / self.viewport.extent[1];
        self.camera.set_aspect(aspect_ratio);

        // 计算 MVP 矩阵（使用 Camera 组件）
        let model = self.scene.model.transform.to_matrix();
        let view = self.camera.view_matrix();
        let mut projection = self.camera.proj_matrix();

        // Vulkan 的 NDC 坐标系 Y 轴与 DX12 相反，需要翻转
        // nalgebra 生成的是 OpenGL 风格的投影矩阵（Y 向上）
        // Vulkan 的 Y 轴向下，所以需要翻转投影矩阵的 Y 分量

        // 使用 DirectionalLight 组件获取光照参数
        let light_direction = self.directional_light.direction;
        let light_color_intensity = self.directional_light.color.with_intensity(self.directional_light.intensity);
        let light_col_int = [
            light_color_intensity[0],
            light_color_intensity[1],
            light_color_intensity[2],
            self.directional_light.intensity,
        ];
        let camera_pos = self.camera.position();
        let ubo = UniformBufferObject::new(
            &model,
            &view,
            &projection,
            [light_direction.x, light_direction.y, light_direction.z],
            light_col_int,
            [camera_pos.x, camera_pos.y, camera_pos.z],
        );

        // 创建 uniform buffer
        let uniform_subbuffer = Buffer::from_data(
            self.gfx.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            ubo,
        )
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
            []
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
                    clear_values: vec![
                        Some(self.scene.clear_color.into()),
                        Some(1.0f32.into()),  // 深度缓冲清空为1.0（最远）
                    ],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo {
                    contents: vulkano::command_buffer::SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to begin render pass: {:?}", e))
            ))?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to set viewport: {:?}", e))
            ))?
            .bind_pipeline_graphics(self.pipeline.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to bind pipeline: {:?}", e))
            ))?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to bind descriptor sets: {:?}", e))
            ))?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to bind vertex buffer: {:?}", e))
            ))?
            .bind_index_buffer(self.index_buffer.clone())
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to bind index buffer: {:?}", e))
            ))?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::CommandExecution(format!("Failed to record draw command: {:?}", e))
            ))?
            .end_render_pass(SubpassEndInfo::default())
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
            Err(e) => {
                // Check if it's an OutOfDate error
                let err_string = format!("{:?}", e);
                if err_string.contains("OutOfDate") {
                    #[cfg(debug_assertions)]
                    debug!("Flush error: swapchain out of date");
                    self.recreate_swapchain = true;
                } else {
                    error!("Failed to flush future: {:?}", e);
                }
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

    /// Update camera based on input system state
    ///
    /// Called every frame before draw() to apply user input to camera
    pub fn update(&mut self, input_system: &mut crate::core::input::InputSystem, delta_time: f32) {
        input_system.update_camera(&mut self.camera, delta_time);
    }

    pub fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        self.scene.clear_color = packet.clear_color;
        self.scene.model.transform.position = packet.model_position;
        self.scene.model.transform.rotation = packet.model_rotation;
        self.scene.model.transform.scale = packet.model_scale;

        self.directional_light.intensity = packet.light_intensity;
        self.directional_light.direction = Vector3::new(
            packet.light_direction[0],
            packet.light_direction[1],
            packet.light_direction[2],
        )
        .normalize();

        if (self.camera.fov_x() - packet.camera_fov * PI / 180.0).abs() > 0.01 {
            self.camera.set_lens(
                packet.camera_fov * PI / 180.0,
                self.camera.aspect(),
                packet.camera_near,
                packet.camera_far,
            );
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        debug!("Dropping Vulkan Renderer...");
        
        // 只清理 previous_frame_end，不调用 flush 避免卡死
        if let Some(mut future) = self.previous_frame_end.take() {
            future.cleanup_finished();
        }
        
        #[cfg(debug_assertions)]
        debug!("Vulkan Renderer dropped successfully");
    }
}

fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    depth_image: Arc<Image>,
    viewport: &mut Viewport,
) -> Result<Vec<Arc<Framebuffer>>> {
    let dimensions = images[0].extent();
    viewport.extent = [dimensions[0] as f32, dimensions[1] as f32];

    let depth_view = ImageView::new_default(depth_image)
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create depth image view: {:?}", e))
        ))?;

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
                    attachments: vec![view, depth_view.clone()],
                    ..Default::default()
                },
            )
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create framebuffer: {:?}", e))
            ))
        })
        .collect::<Result<Vec<_>>>()
}
