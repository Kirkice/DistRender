use std::sync::Arc;
use winit::event_loop::EventLoop;
use crate::renderer::vulkan::Renderer as VulkanRenderer;
use crate::renderer::dx12::Renderer as Dx12Renderer;

pub mod vertex;
pub mod shaders;
pub mod vulkan;
pub mod dx12;

pub enum Backend {
    Vulkan(VulkanRenderer),
    Dx12(Dx12Renderer),
}

pub struct Renderer {
    backend: Backend,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, use_dx12: bool) -> Self {
        let backend = if use_dx12 {
            println!("Initializing DX12 Backend...");
            let renderer = Dx12Renderer::new(event_loop);
            Backend::Dx12(renderer)
        } else {
            println!("Initializing Vulkan Backend...");
            let renderer = VulkanRenderer::new(event_loop);
            Backend::Vulkan(renderer)
        };

        Self { backend }
    }

    pub fn resize(&mut self) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.resize(),
            Backend::Dx12(r) => r.resize(),
        }
    }

    pub fn draw(&mut self) {
        match &mut self.backend {
            Backend::Vulkan(r) => r.draw(),
            Backend::Dx12(r) => r.draw(),
        }
    }
}
