//! wgpu 娓叉煋鍣ㄥ疄鐜?
//!
//! 鏈ā鍧楀疄鐜颁簡鍩轰簬 wgpu 鐨勬覆鏌撳櫒锛屽寘鎷細
//! - 娓叉煋绠＄嚎鍒涘缓
//! - 璧勬簮绠＄悊锛堥《鐐圭紦鍐层€佺储寮曠紦鍐层€乁niform缂撳啿锛?
//! - 娓叉煋寰幆
//! - 鐩告満鍜屽厜鐓ч泦鎴?

use tracing::{debug, info, warn};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::gfx::wgpu::context::WgpuContext;
use crate::renderer::vertex::{MyVertex, create_default_triangle, convert_geometry_vertex};
use crate::renderer::resource::FrameResourcePool;
use crate::renderer::sync::FenceManager;
use crate::core::{Config, SceneConfig};
use crate::core::error::{Result, GraphicsError};
use crate::geometry::loaders::{MeshLoader, ObjLoader};
use crate::component::{Camera, DirectionalLight};
use crate::core::input::InputSystem;
use crate::math::{Vector3, Matrix4};
use crate::gui::{GuiManager, GuiState};
use crate::gui::ipc::GuiStatePacket;
use std::path::Path;
use std::f32::consts::PI;

/// Uniform Buffer Object - MVP 鐭╅樀鍜屽厜鐓ф暟鎹?
///
/// 杩欎釜缁撴瀯浣撲細琚紶杈撳埌 GPU 鐨?uniform buffer 涓€?
/// 蹇呴』浣跨敤 #[repr(C)] 淇濊瘉鍐呭瓨甯冨眬涓庣潃鑹插櫒涓€鑷淬€?
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

/// wgpu 娓叉煋鍣?
pub struct Renderer {
    gfx: WgpuContext,

    // 娓叉煋绠＄嚎鍜岃祫婧?
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // 鍦烘櫙瀵硅薄
    camera: Camera,
    directional_light: DirectionalLight,
    scene: SceneConfig,

    // 閫氱敤绠＄悊鍣?
    frame_resource_pool: FrameResourcePool,
    fence_manager: FenceManager,

    // GUI 绠＄悊鍣?
    gui_manager: GuiManager,

    // 娓叉煋鐘舵€?
    num_indices: u32,
}

impl Renderer {
    /// 鍒涘缓鏂扮殑 wgpu 娓叉煋鍣?
    pub fn new(
        event_loop: &winit::event_loop::EventLoop<()>,
        config: &Config,
        scene: &SceneConfig,
    ) -> Result<Self> {
        info!("Creating wgpu renderer");

        // 1. 鍒涘缓 wgpu 鍚庣
        let gfx = WgpuContext::new(event_loop, config)?;

        // 2. 鍔犺浇鐫€鑹插櫒妯″潡
        debug!("Loading shaders");
        let shader_source = include_str!("shaders/shader.wgsl");
        let shader_module = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // 3. 鍒涘缓 Uniform Buffer
        debug!("Creating uniform buffer");
        let uniform_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<UniformBufferObject>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 4. 鍒涘缓 Bind Group Layout
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

        // 5. 鍒涘缓 Bind Group
        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // 6. 鍒涘缓娓叉煋绠＄嚎甯冨眬
        let pipeline_layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 7. 鍒涘缓娣卞害绾圭悊
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

        // 8. 鍒涘缓娓叉煋绠＄嚎
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

        // 9. 鍔犺浇妯″瀷鏁版嵁鎴栦娇鐢ㄩ粯璁や笁瑙掑舰
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

        // 10. 鍒涘缓椤剁偣缂撳啿
        debug!("Creating vertex buffer");
        let vertex_buffer = gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 11. 鍒涘缓绱㈠紩缂撳啿
        debug!("Creating index buffer");
        let index_buffer = gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // 12. 鍒濆鍖栫浉鏈?
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

        // 濡傛灉鏈夋棆杞紝浣跨敤 look_at 璁剧疆鐩告満鏈濆悜
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

        // 13. 鍒濆鍖栧厜鐓?
        debug!("Initializing lights");
        let directional_light = scene.light.to_directional_light("MainLight");
        info!(
            "DirectionalLight component initialized: color={:?}, intensity={}, direction={:?}",
            directional_light.color.to_array(),
            directional_light.intensity,
            directional_light.direction
        );

        // 14. 鍒濆鍖栧抚璧勬簮绠＄悊
        let frame_resource_pool = FrameResourcePool::triple_buffering();
        let fence_manager = FenceManager::new();

        // 15. 鍒濆鍖?GUI
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

    /// 缁樺埗涓€甯?
    pub fn draw(&mut self) -> Result<()> {
        // 1. 鑾峰彇浜ゆ崲閾剧汗鐞?
        let output = self.gfx.surface.get_current_texture()
            .map_err(|e| GraphicsError::SwapchainError(format!("Failed to acquire next image: {}", e)))?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. 鍒涘缓鍛戒护缂栫爜鍣?
        let mut encoder = self.gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // 3. 鏇存柊 MVP 鐭╅樀
        let model = self.scene.model.transform.to_matrix();
        let view_matrix = self.camera.view_matrix();
        let mut proj_matrix = self.camera.proj_matrix();
        proj_matrix[(1, 1)] *= -1.0;

        // 4. 鍑嗗鍏夌収鍙傛暟
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

        // 5. 鍒涘缓 UBO 骞跺啓鍏ョ紦鍐?
        let ubo = UniformBufferObject::new(
            &model,
            &view_matrix,
            &proj_matrix,
            light_dir_array,
            light_color_intensity,
            camera_pos_array,
        );

        self.gfx.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[ubo]));

        // 6. 寮€濮嬫覆鏌撻€氶亾
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
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // 7. 鏇存柊鍜屾覆鏌?GUI
        self.gui_manager.update(self.gfx.window());
        self.gui_manager.render(
            &self.gfx.device,
            &self.gfx.queue,
            &mut encoder,
            &view,
            self.gfx.window(),
        )?;

        // 8. 鎻愪氦鍛戒护
        self.gfx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // 9. 搴旂敤 GUI 鐘舵€佸埌鍦烘櫙
        self.apply_gui_state();

        // 10. 鏇存柊甯ц祫婧愮姸鎬?
        self.fence_manager.next_value();
        self.frame_resource_pool.current_mut().mark_in_use(self.fence_manager.current_value().value());
        self.frame_resource_pool.advance();

        Ok(())
    }

    /// 澶勭悊绐楀彛澶у皬璋冩暣
    pub fn resize(&mut self) {
        let size = self.gfx.window().inner_size();

        if size.width > 0 && size.height > 0 {
            debug!("Resizing to {}x{}", size.width, size.height);

            // 閲嶆柊閰嶇疆琛ㄩ潰
            self.gfx.reconfigure_surface(size.width, size.height);

            // 閲嶅缓娣卞害绾圭悊
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

            // 鏇存柊鐩告満瀹介珮姣?
            let aspect = size.width as f32 / size.height as f32;
            self.camera.set_aspect(aspect);
        }
    }

    /// 鏇存柊鐩告満锛堝熀浜庤緭鍏ョ郴缁燂級
    pub fn update(&mut self, input_system: &mut InputSystem, delta_time: f32) {
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

    /// 搴旂敤 GUI 鐘舵€佸埌鍦烘櫙
    fn apply_gui_state(&mut self) {
        let state = self.gui_manager.state();

        let packet = GuiStatePacket {
            clear_color: state.clear_color,
            light_intensity: state.light_intensity,
            light_direction: state.light_direction,
            model_position: state.model_position,
            model_rotation: state.model_rotation,
            model_scale: state.model_scale,
            camera_fov: state.camera_fov,
            camera_near: state.camera_near,
            camera_far: state.camera_far,
        };

        self.apply_gui_packet(&packet);
    }

    /// 澶勭悊 GUI 浜嬩欢
    /// 杩斿洖 true 濡傛灉浜嬩欢琚?GUI 娑堣垂
    pub fn handle_gui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.gui_manager.handle_event(self.gfx.window(), event)
    }

    /// 鑾峰彇绐楀彛寮曠敤
    pub fn window(&self) -> &winit::window::Window {
        self.gfx.window()
    }
}

/// 瀹炵幇缁熶竴鐨勬覆鏌撳悗绔帴鍙?
impl crate::renderer::backend_trait::RenderBackend for Renderer {
    fn window(&self) -> &winit::window::Window {
        self.window()
    }

    fn resize(&mut self) {
        self.resize()
    }

    fn draw(&mut self) -> crate::core::error::Result<()> {
        self.draw()
    }

    fn update(&mut self, input_system: &mut InputSystem, delta_time: f32) {
        self.update(input_system, delta_time)
    }

    fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        self.apply_gui_packet(packet)
    }

    fn handle_gui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.handle_gui_event(event)
    }
}
