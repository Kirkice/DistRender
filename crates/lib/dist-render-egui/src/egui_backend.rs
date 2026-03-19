use std::{
    collections::HashMap,
    ffi::CStr,
    mem,
    os::raw::c_void,
    ptr,
    slice,
    sync::Arc,
};

use dist_render::{
    backend::{
        ash::{self, vk},
        Device, Image, ImageDesc, ImageViewDesc,
    },
    ui_renderer::UiRenderer,
};
use egui::{
    epaint::{ClippedMesh, Texture, TextureId, Vertex},
    Color32, CtxRef, FontDefinitions, FontFamily, Stroke, Style, Visuals,
};
use egui_winit::State as EguiWinitState;
use memoffset::offset_of;
use parking_lot::Mutex;

pub const VIEWPORT_TEXTURE_ID: TextureId = TextureId::User(1);

struct GfxResources {
    egui_render_pass: vk::RenderPass,
    egui_framebuffer: vk::Framebuffer,
    egui_texture: Arc<Image>,
}

struct FontTextureResources {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    width: u32,
    height: u32,
    uploaded_version: u64,
}

struct UserTextureResources {
    _image: Arc<Image>,
    descriptor_set: vk::DescriptorSet,
}

struct HostBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    mapping: *mut c_void,
    size: vk::DeviceSize,
}

impl HostBuffer {
    fn new(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let buffer_create_info = vk::BufferCreateInfo {
            size,
            usage,
            ..Default::default()
        };
        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }.unwrap();
        let mem_req = unsafe { device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = get_memory_type_index(
            memory_properties,
            mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .unwrap();

        let memory_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: mem_req.size.max(size),
            memory_type_index,
            ..Default::default()
        };
        let memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.unwrap();
        unsafe { device.bind_buffer_memory(buffer, memory, 0) }.unwrap();

        let mapping = unsafe { device.map_memory(memory, 0, size, Default::default()) }.unwrap();

        Self {
            buffer,
            memory,
            mapping,
            size,
        }
    }

    fn ensure_capacity(
        &mut self,
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        required_size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) {
        if self.size >= required_size {
            return;
        }

        self.destroy(device);
        *self = Self::new(device, memory_properties, required_size, usage);
    }

    fn destroy(&mut self, device: &ash::Device) {
        if self.mapping.is_null() {
            return;
        }

        unsafe {
            device.unmap_memory(self.memory);
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }

        self.mapping = ptr::null_mut();
        self.buffer = vk::Buffer::null();
        self.memory = vk::DeviceMemory::null();
        self.size = 0;
    }
}

pub struct EguiBackendInner {
    renderer: Renderer,
    gfx: Option<GfxResources>,
}

pub struct EguiBackend {
    inner: Arc<Mutex<EguiBackendInner>>,
    device: Arc<Device>,
    egui_platform: EguiWinitState,
}

impl EguiBackend {
    pub fn new(device: Arc<Device>, window: &winit::window::Window, egui: &CtxRef) -> Self {
        setup_egui_style(egui);

        let renderer = Renderer::new(
            &device.raw,
            &device.physical_device().properties,
            &device.physical_device().memory_properties,
        );

        let egui_platform = EguiWinitState::from_pixels_per_point(1.0);

        let _ = window;

        Self {
            device,
            egui_platform,
            inner: Arc::new(Mutex::new(EguiBackendInner {
                renderer,
                gfx: None,
            })),
        }
    }

    pub fn create_graphics_resources(&mut self, surface_resolution: [u32; 2]) {
        self.inner
            .lock()
            .create_graphics_resources(self.device.as_ref(), surface_resolution);
    }

    pub fn register_user_texture(&mut self, texture_id: TextureId, image: Arc<Image>) {
        self.inner
            .lock()
            .renderer
            .register_user_texture(self.device.as_ref(), texture_id, image);
    }

    pub fn viewport_texture_id() -> TextureId {
        VIEWPORT_TEXTURE_ID
    }

    pub fn handle_event(
        &mut self,
        _window: &winit::window::Window,
        egui: &CtxRef,
        event: &winit::event::Event<'_, ()>,
    ) -> bool {
        match event {
            winit::event::Event::WindowEvent { event, .. } => self.egui_platform.on_event(egui, event),
            _ => false,
        }
    }

    pub fn prepare_frame(&mut self, window: &winit::window::Window, egui: &mut CtxRef) {
        let raw_input = self.egui_platform.take_egui_input(window);
        egui.begin_frame(raw_input);
    }

    pub fn finish_frame(
        &mut self,
        egui: &CtxRef,
        window: &winit::window::Window,
        ui_renderer: &mut UiRenderer,
    ) {
        let pixels_per_point = egui.pixels_per_point();
        let (output, shapes) = egui.end_frame();
        let clipped_meshes = egui.tessellate(shapes);
        self.egui_platform.handle_output(window, egui, output);

        let font_texture = egui.texture();
        let ui_target_image = self.inner.lock().get_target_image().unwrap();
        let inner = self.inner.clone();
        let device = self.device.clone();
        let gui_extent = [window.inner_size().width, window.inner_size().height];

        ui_renderer.ui_frame = Some((
            Box::new(move |cb| {
                inner.lock().render(
                    gui_extent,
                    pixels_per_point,
                    clipped_meshes,
                    font_texture,
                    device,
                    cb,
                );

                Ok(())
            }),
            ui_target_image,
        ));
    }
}

impl EguiBackendInner {
    fn create_graphics_resources(&mut self, device: &Device, surface_resolution: [u32; 2]) {
        assert!(self.gfx.is_none());

        let egui_render_pass = create_egui_render_pass(&device.raw);
        let (egui_framebuffer, egui_texture) =
            create_egui_framebuffer(device, egui_render_pass, surface_resolution);

        let gfx = GfxResources {
            egui_render_pass,
            egui_framebuffer,
            egui_texture,
        };

        self.renderer
            .create_pipeline(&device.raw, gfx.egui_render_pass);

        self.gfx = Some(gfx);
    }

    fn get_target_image(&self) -> Option<Arc<Image>> {
        self.gfx.as_ref().map(|gfx| gfx.egui_texture.clone())
    }

    fn render(
        &mut self,
        physical_size: [u32; 2],
        pixels_per_point: f32,
        clipped_meshes: Vec<ClippedMesh>,
        font_texture: Arc<Texture>,
        device: Arc<Device>,
        cb: vk::CommandBuffer,
    ) {
        let raw_device = &device.raw;

        if let Some(ref gfx) = self.gfx {
            self.renderer
                .prepare_font_texture(raw_device, cb, font_texture.as_ref());

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            }];

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(gfx.egui_render_pass)
                .framebuffer(gfx.egui_framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: physical_size[0],
                        height: physical_size[1],
                    },
                })
                .clear_values(&clear_values);

            unsafe {
                raw_device.cmd_begin_render_pass(
                    cb,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
            }

            self.renderer
                .render(raw_device, cb, physical_size, pixels_per_point, &clipped_meshes);

            unsafe {
                raw_device.cmd_end_render_pass(cb);
            }
        }
    }
}

struct Renderer {
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    _descriptor_pool: vk::DescriptorPool,
    font_descriptor_set: vk::DescriptorSet,
    sampler: vk::Sampler,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline: Option<vk::Pipeline>,
    vertex_buffers: [HostBuffer; Renderer::FRAME_COUNT],
    index_buffers: [HostBuffer; Renderer::FRAME_COUNT],
    staging_buffer: HostBuffer,
    font_texture: Option<FontTextureResources>,
    user_textures: HashMap<u64, UserTextureResources>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    frame_index: usize,
}

impl Renderer {
    const FRAME_COUNT: usize = 2;
    const QUAD_COUNT_PER_FRAME: usize = 64 * 1024;
    const VERTEX_COUNT_PER_FRAME: usize = 4 * Renderer::QUAD_COUNT_PER_FRAME;
    const INDEX_COUNT_PER_FRAME: usize = 6 * Renderer::QUAD_COUNT_PER_FRAME;
    const PUSH_CONSTANT_SIZE: usize = 8;
    const INITIAL_STAGING_CAPACITY: vk::DeviceSize = 1024 * 1024;

    fn new(
        device: &ash::Device,
        _physical_device_properties: &vk::PhysicalDeviceProperties,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Self {
        let vertex_shader = load_shader_module(device, include_bytes!("../../ash-imgui/src/imgui.vert.spv"));
        let fragment_shader = load_shader_module(device, include_bytes!("../../ash-imgui/src/imgui.frag.spv"));

        let sampler = {
            let sampler_create_info = vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                ..Default::default()
            };
            unsafe { device.create_sampler(&sampler_create_info, None) }.unwrap()
        };

        let descriptor_set_layout = {
            let binding = vk::DescriptorSetLayoutBinding::builder()
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(slice::from_ref(&sampler));
            let descriptor_set_layout_create_info =
                vk::DescriptorSetLayoutCreateInfo::builder().bindings(slice::from_ref(&binding));
            unsafe { device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None) }
                .unwrap()
        };

        let pipeline_layout = {
            let push_constant_range = vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX,
                offset: 0,
                size: Renderer::PUSH_CONSTANT_SIZE as u32,
            };
            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(slice::from_ref(&descriptor_set_layout))
                .push_constant_ranges(slice::from_ref(&push_constant_range));
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None) }.unwrap()
        };

        let descriptor_pool = {
            let descriptor_pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 8,
            }];
            let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(8)
                .pool_sizes(&descriptor_pool_sizes);
            unsafe { device.create_descriptor_pool(&descriptor_pool_create_info, None) }.unwrap()
        };

        let font_descriptor_set = Self::allocate_descriptor_set(device, descriptor_pool, descriptor_set_layout);

        let vertex_buffers = std::array::from_fn(|_| {
            HostBuffer::new(
                device,
                memory_properties,
                (Renderer::VERTEX_COUNT_PER_FRAME * mem::size_of::<Vertex>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            )
        });

        let index_buffers = std::array::from_fn(|_| {
            HostBuffer::new(
                device,
                memory_properties,
                (Renderer::INDEX_COUNT_PER_FRAME * mem::size_of::<u32>()) as vk::DeviceSize,
                vk::BufferUsageFlags::INDEX_BUFFER,
            )
        });

        let staging_buffer = HostBuffer::new(
            device,
            memory_properties,
            Renderer::INITIAL_STAGING_CAPACITY,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );

        Self {
            pipeline_layout,
            descriptor_set_layout,
            _descriptor_pool: descriptor_pool,
            font_descriptor_set,
            sampler,
            vertex_shader,
            fragment_shader,
            pipeline: None,
            vertex_buffers,
            index_buffers,
            staging_buffer,
            font_texture: None,
            user_textures: HashMap::new(),
            memory_properties: *memory_properties,
            frame_index: 0,
        }
    }

    fn allocate_descriptor_set(
        device: &ash::Device,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::DescriptorSet {
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(slice::from_ref(&descriptor_set_layout));
        unsafe { device.allocate_descriptor_sets(&descriptor_set_allocate_info) }.unwrap()[0]
    }

    fn update_descriptor_set_image(
        &self,
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        image_view: vk::ImageView,
    ) {
        let image_info = vk::DescriptorImageInfo {
            sampler: self.sampler,
            image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let write_descriptor_set = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(slice::from_ref(&image_info));
        unsafe { device.update_descriptor_sets(slice::from_ref(&write_descriptor_set), &[]) };
    }

    fn register_user_texture(&mut self, device: &Device, texture_id: TextureId, image: Arc<Image>) {
        let TextureId::User(id) = texture_id else {
            return;
        };

        let descriptor_set = self
            .user_textures
            .get(&id)
            .map(|existing| existing.descriptor_set)
            .unwrap_or_else(|| {
                Self::allocate_descriptor_set(
                    &device.raw,
                    self._descriptor_pool,
                    self.descriptor_set_layout,
                )
            });

        let image_view = image.view(device, &ImageViewDesc::default()).unwrap();
        self.update_descriptor_set_image(&device.raw, descriptor_set, image_view);

        self.user_textures.insert(
            id,
            UserTextureResources {
                _image: image,
                descriptor_set,
            },
        );
    }

    fn prepare_font_texture(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        font_texture: &Texture,
    ) {
        let mut texture_pixels = Vec::with_capacity(font_texture.pixels.len() * 4);
        for color in font_texture.srgba_pixels(1.0) {
            texture_pixels.extend_from_slice(&color.to_array());
        }

        let texture_size = texture_pixels.len() as vk::DeviceSize;
        self.staging_buffer.ensure_capacity(
            device,
            &self.memory_properties,
            texture_size.max(1),
            vk::BufferUsageFlags::TRANSFER_SRC,
        );

        let recreate_image = self.font_texture.as_ref().map_or(true, |existing| {
            existing.width != font_texture.width as u32 || existing.height != font_texture.height as u32
        });

        if recreate_image {
            self.recreate_font_image(device, font_texture.width as u32, font_texture.height as u32);
        }

        let needs_upload = self.font_texture.as_ref().map_or(true, |existing| {
            existing.uploaded_version != font_texture.version
        });

        if !needs_upload {
            return;
        }

        unsafe {
            (self.staging_buffer.mapping as *mut u8)
                .copy_from_nonoverlapping(texture_pixels.as_ptr(), texture_pixels.len());
        }

        let font_image = self.font_texture.as_mut().unwrap();

        let old_layout = if font_image.uploaded_version == u64::MAX {
            vk::ImageLayout::UNDEFINED
        } else {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        };
        let src_stage = if old_layout == vk::ImageLayout::UNDEFINED {
            vk::PipelineStageFlags::TOP_OF_PIPE
        } else {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        };
        let src_access = if old_layout == vk::ImageLayout::UNDEFINED {
            vk::AccessFlags::empty()
        } else {
            vk::AccessFlags::SHADER_READ
        };

        let transfer_barrier = vk::ImageMemoryBarrier {
            src_access_mask: src_access,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            old_layout,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: font_image.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&transfer_barrier),
            );
        }

        let buffer_image_copy = vk::BufferImageCopy {
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                layer_count: 1,
                ..Default::default()
            },
            image_extent: vk::Extent3D {
                width: font_image.width,
                height: font_image.height,
                depth: 1,
            },
            ..Default::default()
        };

        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                self.staging_buffer.buffer,
                font_image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&buffer_image_copy),
            );
        }

        let shader_barrier = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: font_image.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&shader_barrier),
            );
        }

        font_image.uploaded_version = font_texture.version;
    }

    fn recreate_font_image(&mut self, device: &ash::Device, width: u32, height: u32) {
        if let Some(mut existing) = self.font_texture.take() {
            unsafe {
                device.destroy_image_view(existing.view, None);
                device.destroy_image(existing.image, None);
                device.free_memory(existing.memory, None);
            }
            existing.view = vk::ImageView::null();
            existing.image = vk::Image::null();
            existing.memory = vk::DeviceMemory::null();
        }

        let image_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            ..Default::default()
        };
        let image = unsafe { device.create_image(&image_create_info, None) }.unwrap();
        let mem_req = unsafe { device.get_image_memory_requirements(image) };

        let memory_type_index = get_memory_type_index(
            &self.memory_properties,
            mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .unwrap();

        let memory_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: mem_req.size,
            memory_type_index,
            ..Default::default()
        };
        let memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.unwrap();
        unsafe { device.bind_image_memory(image, memory, 0) }.unwrap();

        let view_create_info = vk::ImageViewCreateInfo {
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let view = unsafe { device.create_image_view(&view_create_info, None) }.unwrap();

        self.update_descriptor_set_image(device, self.font_descriptor_set, view);

        self.font_texture = Some(FontTextureResources {
            image,
            memory,
            view,
            width,
            height,
            uploaded_version: u64::MAX,
        });
    }

    fn create_pipeline(&mut self, device: &ash::Device, render_pass: vk::RenderPass) {
        let shader_entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let shader_stage_create_info = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: self.vertex_shader,
                p_name: shader_entry_name.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: self.fragment_shader,
                p_name: shader_entry_name.as_ptr(),
                ..Default::default()
            },
        ];

        let vertex_input_binding = vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        };
        let vertex_input_attributes = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R8G8B8A8_UNORM,
                offset: offset_of!(Vertex, color) as u32,
            },
        ];

        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(slice::from_ref(&vertex_input_binding))
            .vertex_attribute_descriptions(&vertex_input_attributes);

        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            line_width: 1.0,
            ..Default::default()
        };
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::all(),
        };
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(slice::from_ref(&color_blend_attachment_state));
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_create_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_info)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .color_blend_state(&color_blend_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .layout(self.pipeline_layout)
            .render_pass(render_pass);

        self.pipeline = Some(
            unsafe {
                device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    slice::from_ref(&pipeline_create_info),
                    None,
                )
            }
            .unwrap()[0],
        );
    }

    fn render(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        physical_size: [u32; 2],
        pixels_per_point: f32,
        clipped_meshes: &[ClippedMesh],
    ) {
        self.frame_index = (self.frame_index + 1) % Renderer::FRAME_COUNT;

        let width = physical_size[0] as f32;
        let height = physical_size[1] as f32;
        let dims_rcp = [pixels_per_point / width.max(1.0), pixels_per_point / height.max(1.0)];

        let vertex_buffer = &self.vertex_buffers[self.frame_index];
        let index_buffer = &self.index_buffers[self.frame_index];

        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.unwrap(),
            );
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                std::slice::from_raw_parts(
                    dims_rcp.as_ptr() as *const u8,
                    Renderer::PUSH_CONSTANT_SIZE,
                ),
            );
        }

        let viewport = vk::Viewport {
            width,
            height,
            max_depth: 1.0,
            ..Default::default()
        };
        unsafe {
            device.cmd_set_viewport(command_buffer, 0, slice::from_ref(&viewport));
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                slice::from_ref(&vertex_buffer.buffer),
                &[0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
        }

        let vertex_capacity = Renderer::VERTEX_COUNT_PER_FRAME;
        let index_capacity = Renderer::INDEX_COUNT_PER_FRAME;
        let vertex_base = vertex_buffer.mapping as *mut Vertex;
        let index_base = index_buffer.mapping as *mut u32;

        let mut vertex_offset = 0usize;
        let mut index_offset = 0usize;
        let mut bound_descriptor_set = None;

        for egui::epaint::ClippedMesh(clip_rect, mesh) in clipped_meshes {
            let descriptor_set = match mesh.texture_id {
                TextureId::Egui => self.font_descriptor_set,
                TextureId::User(id) => match self.user_textures.get(&id) {
                    Some(texture) => texture.descriptor_set,
                    None => continue,
                },
            };

            let next_vertex_offset = vertex_offset + mesh.vertices.len();
            let next_index_offset = index_offset + mesh.indices.len();
            if next_vertex_offset > vertex_capacity || next_index_offset > index_capacity {
                log::warn!("egui vertex/index budget exhausted; skipping remaining UI meshes");
                break;
            }

            unsafe {
                vertex_base
                    .add(vertex_offset)
                    .copy_from_nonoverlapping(mesh.vertices.as_ptr(), mesh.vertices.len());
                index_base
                    .add(index_offset)
                    .copy_from_nonoverlapping(mesh.indices.as_ptr(), mesh.indices.len());
            }

            let clip_min_x = (clip_rect.min.x * pixels_per_point).clamp(0.0, width);
            let clip_min_y = (clip_rect.min.y * pixels_per_point).clamp(0.0, height);
            let clip_max_x = (clip_rect.max.x * pixels_per_point).clamp(clip_min_x, width);
            let clip_max_y = (clip_rect.max.y * pixels_per_point).clamp(clip_min_y, height);

            if clip_max_x <= clip_min_x || clip_max_y <= clip_min_y {
                vertex_offset = next_vertex_offset;
                index_offset = next_index_offset;
                continue;
            }

            let scissor = vk::Rect2D {
                offset: vk::Offset2D {
                    x: clip_min_x.floor() as i32,
                    y: clip_min_y.floor() as i32,
                },
                extent: vk::Extent2D {
                    width: (clip_max_x - clip_min_x).ceil() as u32,
                    height: (clip_max_y - clip_min_y).ceil() as u32,
                },
            };

            unsafe {
                if bound_descriptor_set != Some(descriptor_set) {
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        slice::from_ref(&descriptor_set),
                        &[],
                    );
                    bound_descriptor_set = Some(descriptor_set);
                }

                device.cmd_set_scissor(command_buffer, 0, slice::from_ref(&scissor));
                device.cmd_draw_indexed(
                    command_buffer,
                    mesh.indices.len() as u32,
                    1,
                    index_offset as u32,
                    vertex_offset as i32,
                    0,
                );
            }

            vertex_offset = next_vertex_offset;
            index_offset = next_index_offset;
        }
    }
}

fn load_shader_module(device: &ash::Device, bytes: &[u8]) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        code_size: bytes.len(),
        p_code: bytes.as_ptr() as *const u32,
        ..Default::default()
    };
    unsafe { device.create_shader_module(&shader_module_create_info, None) }.unwrap()
}

fn get_memory_type_index(
    physical_device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    property_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..physical_device_memory_properties.memory_type_count {
        let memory_type = &physical_device_memory_properties.memory_types[i as usize];
        if (type_filter & (1 << i)) != 0 && memory_type.property_flags.contains(property_flags) {
            return Some(i);
        }
    }

    None
}

fn create_egui_render_pass(device: &ash::Device) -> vk::RenderPass {
    let render_pass_attachments = [vk::AttachmentDescription {
        format: vk::Format::R8G8B8A8_UNORM,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ..Default::default()
    }];
    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
    }];

    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build()];

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&render_pass_attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe { device.create_render_pass(&render_pass_create_info, None) }.unwrap()
}

fn create_egui_framebuffer(
    device: &Device,
    render_pass: vk::RenderPass,
    surface_resolution: [u32; 2],
) -> (vk::Framebuffer, Arc<Image>) {
    let texture = device
        .create_image(
            ImageDesc::new_2d(vk::Format::R8G8B8A8_UNORM, surface_resolution)
                .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT),
            vec![],
        )
        .unwrap();

    let framebuffer_attachments = [texture.view(device, &ImageViewDesc::default()).unwrap()];
    let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&framebuffer_attachments)
        .width(surface_resolution[0])
        .height(surface_resolution[1])
        .layers(1);

    let framebuffer = unsafe {
        device
            .raw
            .create_framebuffer(&framebuffer_create_info, None)
    }
    .unwrap();

    (framebuffer, Arc::new(texture))
}

fn setup_egui_style(ctx: &CtxRef) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "roboto".to_owned(),
        std::borrow::Cow::Borrowed(include_bytes!("../../../../assets/fonts/Roboto-Regular.ttf")),
    );
    fonts
        .fonts_for_family
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "roboto".to_owned());
    fonts
        .fonts_for_family
        .entry(FontFamily::Monospace)
        .or_default()
        .push("roboto".to_owned());

    fonts
        .family_and_size
        .insert(egui::TextStyle::Heading, (FontFamily::Proportional, 22.0));
    fonts
        .family_and_size
        .insert(egui::TextStyle::Body, (FontFamily::Proportional, 15.0));
    fonts
        .family_and_size
        .insert(egui::TextStyle::Button, (FontFamily::Proportional, 14.0));
    fonts
        .family_and_size
        .insert(egui::TextStyle::Monospace, (FontFamily::Monospace, 14.0));
    ctx.set_fonts(fonts);

    let mut style: Style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_padding = egui::vec2(14.0, 12.0);
    style.spacing.indent = 18.0;

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(228, 235, 242));
    visuals.faint_bg_color = Color32::from_rgb(23, 32, 44);
    visuals.extreme_bg_color = Color32::from_rgb(4, 7, 12);
    visuals.code_bg_color = Color32::from_rgb(19, 28, 39);
    visuals.window_corner_radius = 12.0;
    visuals.selection.bg_fill = Color32::from_rgb(48, 130, 206);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(146, 205, 255));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 25, 36);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(20, 29, 40);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(31, 58, 84);
    visuals.widgets.active.bg_fill = Color32::from_rgb(34, 102, 146);
    visuals.widgets.open.bg_fill = Color32::from_rgb(27, 78, 111);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(29, 44, 62));
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(35, 56, 76));
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(90, 168, 229));
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(147, 216, 255));
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, Color32::from_rgb(104, 191, 247));
    visuals.widgets.noninteractive.corner_radius = 8.0;
    visuals.widgets.inactive.corner_radius = 8.0;
    visuals.widgets.hovered.corner_radius = 10.0;
    visuals.widgets.active.corner_radius = 10.0;
    visuals.widgets.open.corner_radius = 8.0;

    style.visuals = visuals;
    ctx.set_style(style);
}