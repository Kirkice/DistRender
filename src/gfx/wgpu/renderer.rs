//! wgpu 渲染器实现
//!
//! 本模块实现了基于 wgpu 的渲染器，包括：
//! - 渲染管线创建
//! - 资源管理（顶点缓冲、索引缓冲、Uniform缓冲）
//! - 渲染循环
//! - 相机和光照集成

use tracing::{debug, info, warn};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::gfx::wgpu::backend::WgpuBackend;
use crate::renderer::vertex::{MyVertex, create_default_triangle, convert_geometry_vertex};
use crate::renderer::resource::FrameResourcePool;
use crate::renderer::sync::FenceManager;
use crate::core::{Config, SceneConfig, Matrix4};
use crate::core::error::{Result, GraphicsError};
use crate::geometry::loaders::{MeshLoader, ObjLoader};
use crate::component::{Camera, DirectionalLight};
use crate::core::input::InputSystem;
use crate::core::math::Vector3;
use crate::gui::{GuiManager, GuiState};
use std::path::Path;
use std::f32::consts::PI;

/// Uniform Buffer Object - MVP 矩阵和光照数据
///
/// 这个结构体会被传输到 GPU 的 uniform buffer 中。
/// 必须使用 #[repr(C)] 保证内存布局与着色器一致。
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    camera_pos: [f32; 4],
}

impl UniformBufferObject {
    fn new(
        model: &Matrix4,
        view: &Matrix4,
        projection: &Matrix4,
        light_dir: [f32; 3],
        light_color_intensity: [f32; 4],
        camera_pos: [f32; 3],
    ) -> Self {
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

/// wgpu 渲染器
pub struct Renderer {
    gfx: WgpuBackend,

    // 渲染管线和资源
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // 场景对象
    camera: Camera,
    directional_light: DirectionalLight,
    scene: SceneConfig,

    // 通用管理器
    frame_resource_pool: FrameResourcePool,
    fence_manager: FenceManager,

    // GUI 管理器
    gui_manager: GuiManager,

    // 渲染状态
    num_indices: u32,
}

impl Renderer {
    /// 创建新的 wgpu 渲染器
    pub fn new(
        event_loop: &winit::event_loop::EventLoop<()>,
        config: &Config,
        scene: &SceneConfig,
    ) -> Result<Self> {
        info!("Creating wgpu renderer");

        // 1. 创建 wgpu 后端
        let gfx = WgpuBackend::new(event_loop, config)?;

        // 2. 加载着色器模块
        debug!("Loading shaders");
        let shader_source = include_str!("../../renderer/shaders/shader.wgsl");
        let shader_module = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // 3. 创建 Uniform Buffer
        debug!("Creating uniform buffer");
        let uniform_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<UniformBufferObject>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 4. 创建 Bind Group Layout
        debug!("Creating bind group layout");
        let bind_group_layout = gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // 5. 创建 Bind Group
        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // 6. 创建渲染管线布局
        let pipeline_layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 7. 创建深度纹理
        debug!("Creating depth texture");
        let size = gfx.window().inner_size();
        let depth_texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 8. 创建渲染管线
        debug!("Creating render pipeline");
        let render_pipeline = gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MyVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // normal
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // color
                        wgpu::VertexAttribute {
                            offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gfx.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // 9. 加载模型数据或使用默认三角形
        debug!("Loading mesh data");
        let obj_path = Path::new(&scene.model.path);
        let (vertices, indices) = if obj_path.exists() {
            info!("Loading model from: {}", scene.model.path);
            match ObjLoader::load_from_file(obj_path) {
                Ok(mesh_data) => {
                    let vertices: Vec<MyVertex> = mesh_data
                        .vertices
                        .iter()
                        .map(convert_geometry_vertex)
                        .collect();
                    let indices = mesh_data.indices;
                    info!("Model loaded: {} vertices, {} indices", vertices.len(), indices.len());
                    (vertices, indices)
                }
                Err(e) => {
                    warn!("Failed to load model: {}, using default triangle", e);
                    let vertices = create_default_triangle().to_vec();
                    let indices = vec![0, 1, 2];
                    (vertices, indices)
                }
            }
        } else {
            warn!("Model file not found: {}, using default triangle", scene.model.path);
            let vertices = create_default_triangle().to_vec();
            let indices = vec![0, 1, 2];
            (vertices, indices)
        };

        let num_indices = indices.len() as u32;

        // 10. 创建顶点缓冲
        debug!("Creating vertex buffer");
        let vertex_buffer = gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 11. 创建索引缓冲
        debug!("Creating index buffer");
        let index_buffer = gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // 12. 初始化相机
        debug!("Initializing camera");
        let mut camera = Camera::main_camera();
        camera.set_position(Vector3::new(
            scene.camera.transform.position[0],
            scene.camera.transform.position[1],
            scene.camera.transform.position[2],
        ));

        let aspect_ratio = size.width as f32 / size.height as f32;
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

        // 13. 初始化光照
        debug!("Initializing lights");
        let directional_light = scene.light.to_directional_light("MainLight");
        info!(
            "DirectionalLight component initialized: color={:?}, intensity={}, direction={:?}",
            directional_light.color.to_array(),
            directional_light.intensity,
            directional_light.direction
        );

        // 14. 初始化帧资源管理
        let frame_resource_pool = FrameResourcePool::triple_buffering();
        let fence_manager = FenceManager::new();

        // 15. 初始化 GUI
        debug!("Initializing GUI");
        let gui_state = GuiState::new(config, scene);
        let gui_manager = GuiManager::new(
            &gfx.device,
            gfx.surface_config.format,
            gfx.window(),
            gui_state,
        )?;
        info!("GUI manager initialized");

        info!("wgpu renderer created successfully");

        Ok(Self {
            gfx,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            bind_group,
            depth_texture,
            depth_view,
            camera,
            directional_light,
            scene: scene.clone(),
            frame_resource_pool,
            fence_manager,
            gui_manager,
            num_indices,
        })
    }

    /// 绘制一帧
    pub fn draw(&mut self) -> Result<()> {
        // 1. 获取交换链纹理
        let output = self.gfx.surface.get_current_texture()
            .map_err(|e| GraphicsError::SwapchainError(format!("Failed to acquire next image: {}", e)))?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. 创建命令编码器
        let mut encoder = self.gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // 3. 更新 MVP 矩阵
        let model = self.scene.model.transform.to_matrix();
        let view_matrix = self.camera.view_matrix();
        let proj_matrix = self.camera.proj_matrix();

        // 4. 准备光照参数
        let light_dir = self.directional_light.direction;
        let light_dir_array = [light_dir.x, light_dir.y, light_dir.z];
        let light_color = self.directional_light.color.to_array();
        let light_intensity = self.directional_light.intensity;
        let light_color_intensity = [
            light_color[0] * light_intensity,
            light_color[1] * light_intensity,
            light_color[2] * light_intensity,
            1.0,
        ];

        let camera_pos = self.camera.position();
        let camera_pos_array = [camera_pos.x, camera_pos.y, camera_pos.z];

        // 5. 创建 UBO 并写入缓冲
        let ubo = UniformBufferObject::new(
            &model,
            &view_matrix,
            &proj_matrix,
            light_dir_array,
            light_color_intensity,
            camera_pos_array,
        );

        self.gfx.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[ubo]));

        // 6. 开始渲染通道
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.scene.clear_color[0] as f64,
                            g: self.scene.clear_color[1] as f64,
                            b: self.scene.clear_color[2] as f64,
                            a: self.scene.clear_color[3] as f64,
                        }),
                        store: true,  // wgpu 0.17: store 是布尔值
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,  // wgpu 0.17: store 是布尔值
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // 7. 更新和渲染 GUI
        self.gui_manager.update(self.gfx.window());
        self.gui_manager.render(
            &self.gfx.device,
            &self.gfx.queue,
            &mut encoder,
            &view,
            self.gfx.window(),
        )?;

        // 8. 提交命令
        self.gfx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // 9. 应用 GUI 状态到场景
        self.apply_gui_state();

        // 10. 更新帧资源状态
        self.fence_manager.next_value();
        self.frame_resource_pool.current_mut().mark_in_use(self.fence_manager.current_value().value());
        self.frame_resource_pool.advance();

        Ok(())
    }

    /// 处理窗口大小调整
    pub fn resize(&mut self) {
        let size = self.gfx.window().inner_size();

        if size.width > 0 && size.height > 0 {
            debug!("Resizing to {}x{}", size.width, size.height);

            // 重新配置表面
            self.gfx.reconfigure_surface(size.width, size.height);

            // 重建深度纹理
            self.depth_texture = self.gfx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // 更新相机宽高比
            let aspect = size.width as f32 / size.height as f32;
            self.camera.set_aspect(aspect);
        }
    }

    /// 更新相机（基于输入系统）
    pub fn update(&mut self, input_system: &mut InputSystem, delta_time: f32) {
        input_system.update_camera(&mut self.camera, delta_time);
    }

    /// 应用 GUI 状态到场景
    fn apply_gui_state(&mut self) {
        let state = self.gui_manager.state();

        // 更新场景配置
        self.scene.clear_color = state.clear_color;
        self.scene.model.transform.position = state.model_position;
        self.scene.model.transform.rotation = state.model_rotation;
        self.scene.model.transform.scale = state.model_scale;

        // 更新光照
        self.directional_light.intensity = state.light_intensity;
        self.directional_light.direction = Vector3::new(
            state.light_direction[0],
            state.light_direction[1],
            state.light_direction[2],
        ).normalize();

        // 更新相机
        if (self.camera.fov() - state.camera_fov * PI / 180.0).abs() > 0.01 {
            self.camera.set_lens(
                state.camera_fov * PI / 180.0,
                self.camera.aspect(),
                state.camera_near,
                state.camera_far,
            );
        }
    }

    /// 处理 GUI 事件
    /// 返回 true 如果事件被 GUI 消费
    pub fn handle_gui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.gui_manager.handle_event(event)
    }

    /// 获取窗口引用
    pub fn window(&self) -> &winit::window::Window {
        self.gfx.window()
    }
}
