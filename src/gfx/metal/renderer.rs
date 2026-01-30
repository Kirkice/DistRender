//! Metal 渲染器实现

use crate::core::{Config, SceneConfig};
use crate::core::error::{Result, DistRenderError};
use crate::gfx::metal::backend::MetalBackend;
use crate::gfx::GraphicsBackend;
use crate::renderer::vertex::{MyVertex, convert_geometry_vertex, create_default_triangle};
use crate::geometry::loaders::ObjLoader;
use crate::component::{Camera, DirectionalLight};
use crate::core::math::{Matrix4, Vector3};
use crate::core::input::InputSystem;
use winit::window::Window;
use crate::gui::ipc::GuiStatePacket;

use std::path::Path;
use std::f32::consts::PI;
use tracing::{info, warn};
use winit::event_loop::EventLoop;
use metal::*;
use objc::rc::autoreleasepool;
use core_graphics_types::geometry::CGSize;

use crate::geometry::loaders::MeshLoader;

#[repr(C)]
#[derive(Clone, Copy)]
struct Uniforms {
    model: Matrix4,
    view: Matrix4,
    projection: Matrix4,
    light_dir: [f32; 4],
    light_color: [f32; 4],
    camera_pos: [f32; 4],
}

pub struct Renderer {
    backend: MetalBackend,
    pipeline_state: RenderPipelineState,
    depth_stencil_state: DepthStencilState,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    depth_texture: Texture,
    index_count: u64,
    camera: Camera,
    directional_light: DirectionalLight,
    scene: SceneConfig,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &SceneConfig) -> Result<Self> {
        let backend = MetalBackend::new(event_loop, config);
        
        // 1. Load and Compile Shaders from file
        let shader_path = Path::new("src/gfx/metal/shaders/shader.metal");
        let shader_source = std::fs::read_to_string(shader_path)
            .map_err(|e| DistRenderError::Initialization(format!("Failed to load Metal shader file: {}", e)))?;
        
        let device = &backend.device;
        let library = device.new_library_with_source(&shader_source, &CompileOptions::new())
            .map_err(|e| DistRenderError::Initialization(format!("Shader compilation failed: {}", e)))?;
        
        let vertex_function = library.get_function("vertex_main", None)
            .map_err(|_| DistRenderError::Initialization("Vertex function not found".into()))?;
        let fragment_function = library.get_function("fragment_main", None)
            .map_err(|_| DistRenderError::Initialization("Fragment function not found".into()))?;

        // 2. Vertex Descriptor
        let vertex_descriptor = VertexDescriptor::new();
        
        // Position
        vertex_descriptor.attributes().object_at(0).unwrap().set_format(MTLVertexFormat::Float3);
        vertex_descriptor.attributes().object_at(0).unwrap().set_offset(0);
        vertex_descriptor.attributes().object_at(0).unwrap().set_buffer_index(0);
        
        // Normal
        vertex_descriptor.attributes().object_at(1).unwrap().set_format(MTLVertexFormat::Float3);
        vertex_descriptor.attributes().object_at(1).unwrap().set_offset(12);
        vertex_descriptor.attributes().object_at(1).unwrap().set_buffer_index(0);

        // Color
        vertex_descriptor.attributes().object_at(2).unwrap().set_format(MTLVertexFormat::Float3);
        vertex_descriptor.attributes().object_at(2).unwrap().set_offset(24);
        vertex_descriptor.attributes().object_at(2).unwrap().set_buffer_index(0);

        vertex_descriptor.layouts().object_at(0).unwrap().set_stride(36); 
        vertex_descriptor.layouts().object_at(0).unwrap().set_step_rate(1);
        vertex_descriptor.layouts().object_at(0).unwrap().set_step_function(MTLVertexStepFunction::PerVertex);

        // 3. Pipeline State
        let pipeline_descriptor = RenderPipelineDescriptor::new();
        pipeline_descriptor.set_vertex_function(Some(&vertex_function));
        pipeline_descriptor.set_fragment_function(Some(&fragment_function));
        pipeline_descriptor.set_vertex_descriptor(Some(&vertex_descriptor));
        pipeline_descriptor.color_attachments().object_at(0).unwrap().set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        pipeline_descriptor.set_depth_attachment_pixel_format(MTLPixelFormat::Depth32Float);

        let pipeline_state = device.new_render_pipeline_state(&pipeline_descriptor)
            .map_err(|e| DistRenderError::Initialization(format!("Pipeline state creation failed: {}", e)))?;

        // 3.1. Create Depth Stencil State (only once, not per frame!)
        let depth_stencil_desc = DepthStencilDescriptor::new();
        depth_stencil_desc.set_depth_compare_function(MTLCompareFunction::Less);
        depth_stencil_desc.set_depth_write_enabled(true);
        let depth_stencil_state = device.new_depth_stencil_state(&depth_stencil_desc);

        // 4. Load Mesh
        let obj_path = Path::new(&scene.model.path);
        let (vertices, indices) = if obj_path.exists() {
            info!("Loading mesh from: {}", obj_path.display());
            match ObjLoader::load_from_file(obj_path) {
                Ok(mesh_data) => {
                     let verts = mesh_data.vertices.iter().map(|v| convert_geometry_vertex(v)).collect::<Vec<_>>();
                     let inds = mesh_data.indices.clone();
                     (verts, inds)
                }
                Err(e) => {
                    warn!("Failed to load OBJ: {}, using default triangle", e);
                    (create_default_triangle().to_vec(), vec![0, 1, 2])
                }
            }
        } else {
             warn!("OBJ file not found, using default triangle");
             (create_default_triangle().to_vec(), vec![0, 1, 2])
        };

        let vertex_buffer = device.new_buffer_with_data(
            vertices.as_ptr() as *const _,
            (vertices.len() * std::mem::size_of::<MyVertex>()) as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache,
        );
        
        let index_buffer = device.new_buffer_with_data(
            indices.as_ptr() as *const _,
            (indices.len() * std::mem::size_of::<u32>()) as u64,
            MTLResourceOptions::CPUCacheModeDefaultCache,
        );

        // 5. Depth Texture
        let size = backend.window().inner_size();
        let depth_desc = TextureDescriptor::new();
        depth_desc.set_pixel_format(MTLPixelFormat::Depth32Float);
        depth_desc.set_width(size.width as u64);
        depth_desc.set_height(size.height as u64);
        depth_desc.set_usage(MTLTextureUsage::RenderTarget);
        let depth_texture = device.new_texture(&depth_desc);

        // 6. Camera init
        let mut camera = Camera::new("MainCamera");
        let pos = scene.camera.transform.position;
        let cam_pos = Vector3::new(pos[0], pos[1], pos[2]);
        
        // Set camera lens parameters
        let aspect_ratio = size.width as f32 / size.height as f32;
        camera.set_lens(
            scene.camera.fov * PI / 180.0,
            aspect_ratio,
            scene.camera.near_clip,
            scene.camera.far_clip,
        );
        
        // Set camera position and look at origin
        camera.look_at(cam_pos, Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
        
        info!("Camera initialized at position {:?}", cam_pos);
        
        // 7. Initialize directional light
        let directional_light = scene.light.to_directional_light("MainLight");
        info!(
            "DirectionalLight initialized: color={:?}, intensity={}, direction={:?}",
            directional_light.color.to_array(),
            directional_light.intensity,
            directional_light.direction
        );
        
        Ok(Self {
            backend,
            pipeline_state,
            depth_stencil_state,
            vertex_buffer,
            index_buffer,
            depth_texture,
            index_count: indices.len() as u64,
            camera,
            directional_light,
            scene: scene.clone(),
        })
    }

    pub fn resize(&mut self) {
        let window_size = self.backend.window().inner_size();
        self.backend.layer.set_drawable_size(CGSize::new(
            window_size.width as f64, 
            window_size.height as f64
        ));
        
        self.camera.set_aspect(window_size.width as f32 / window_size.height as f32);

        // Recreate depth texture
        let depth_desc = TextureDescriptor::new();
        depth_desc.set_pixel_format(MTLPixelFormat::Depth32Float);
        depth_desc.set_width(window_size.width as u64);
        depth_desc.set_height(window_size.height as u64);
        depth_desc.set_usage(MTLTextureUsage::RenderTarget);
        self.depth_texture = self.backend.device.new_texture(&depth_desc);
    }

    pub fn draw(&mut self) -> Result<()> {
        autoreleasepool(|| {
            if let Some(drawable) = self.backend.layer.next_drawable() {
                let render_pass_descriptor = RenderPassDescriptor::new();
                
                // Color Attachment - use scene clear color
                let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();
                color_attachment.set_texture(Some(drawable.texture()));
                color_attachment.set_load_action(MTLLoadAction::Clear);
                let cc = self.scene.clear_color;
                color_attachment.set_clear_color(MTLClearColor::new(cc[0] as f64, cc[1] as f64, cc[2] as f64, cc[3] as f64));
                color_attachment.set_store_action(MTLStoreAction::Store);

                // Depth Attachment
                let depth_attachment = render_pass_descriptor.depth_attachment().unwrap();
                depth_attachment.set_texture(Some(&self.depth_texture));
                depth_attachment.set_load_action(MTLLoadAction::Clear);
                depth_attachment.set_clear_depth(1.0);
                depth_attachment.set_store_action(MTLStoreAction::DontCare);

                let command_buffer = self.backend.command_queue.new_command_buffer();
                let encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
                
                encoder.set_render_pipeline_state(&self.pipeline_state);
                
                // Create Uniforms - following Vulkan implementation
                let model = self.scene.model.transform.to_matrix();
                let view = self.camera.view_matrix();
                let projection_gl = self.camera.proj_matrix();

                // Apply pre-computed depth correction (OpenGL [-1,1] to Metal [0,1])
                let mut projection = projection_gl;
                projection[(1, 1)] *= -1.0;
                
                // Get light parameters from DirectionalLight component
                let light_direction = self.directional_light.direction;
                let light_color_intensity = self.directional_light.color.with_intensity(self.directional_light.intensity);
                let cam_pos = self.camera.transform().position;
                
                let uniforms = Uniforms {
                    model,
                    view,
                    projection,
                    light_dir: [light_direction.x, light_direction.y, light_direction.z, 0.0],
                    light_color: [
                        light_color_intensity[0],
                        light_color_intensity[1],
                        light_color_intensity[2],
                        self.directional_light.intensity,
                    ],
                    camera_pos: [cam_pos.x, cam_pos.y, cam_pos.z, 1.0],
                };

                encoder.set_vertex_bytes(1, std::mem::size_of::<Uniforms>() as u64, &uniforms as *const _ as *const _);
                encoder.set_fragment_bytes(1, std::mem::size_of::<Uniforms>() as u64, &uniforms as *const _ as *const _);

                // Viewport is critical!
                let window_size = self.backend.window().inner_size();
                let viewport = MTLViewport {
                    originX: 0.0,
                    originY: 0.0,
                    width: window_size.width as f64,
                    height: window_size.height as f64,
                    znear: 0.0,
                    zfar: 1.0,
                };
                encoder.set_viewport(viewport);

                // Culling and Winding
                encoder.set_cull_mode(MTLCullMode::Back);
                encoder.set_front_facing_winding(MTLWinding::CounterClockwise); // OBJ uses CCW

                encoder.set_vertex_buffer(0, Some(&self.vertex_buffer), 0);
                
                // Set Depth Stencil State (created once during initialization)
                encoder.set_depth_stencil_state(&self.depth_stencil_state);

                // Draw Indexed
                encoder.draw_indexed_primitives(
                    MTLPrimitiveType::Triangle,
                    self.index_count,
                    MTLIndexType::UInt32,
                    &self.index_buffer,
                    0
                );

                encoder.end_encoding();

                command_buffer.present_drawable(drawable);
                command_buffer.commit();
            }
        });
        Ok(())
    }

    pub fn update(&mut self, input_system: &mut InputSystem, delta_time: f32) {
        // Update camera based on input system state
        input_system.update_camera(&mut self.camera, delta_time);
    }

    pub fn window(&self) -> &Window {
        self.backend.window()
    }

    pub fn apply_gui_packet(&mut self, packet: &GuiStatePacket) {
        // Update scene configuration from GUI
        self.scene.clear_color = packet.clear_color;
        self.scene.model.transform.position = packet.model_position;
        self.scene.model.transform.rotation = packet.model_rotation;
        self.scene.model.transform.scale = packet.model_scale;

        // Update light parameters
        self.directional_light.intensity = packet.light_intensity;
        self.directional_light.direction = Vector3::new(
            packet.light_direction[0],
            packet.light_direction[1],
            packet.light_direction[2],
        )
        .normalize();

        // Update camera FOV if changed
        if (self.camera.fov_y() - packet.camera_fov * PI / 180.0).abs() > 0.01 {
            self.camera.set_lens(
                packet.camera_fov * PI / 180.0,
                self.camera.aspect(),
                packet.camera_near,
                packet.camera_far,
            );
        }
    }
}

/// 实现统一的渲染后端接口
#[cfg(target_os = "macos")]
impl crate::renderer::backend_trait::RenderBackend for Renderer {
    fn window(&self) -> &Window {
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

    // handle_gui_event 使用默认实现（返回 false）
}
