use std::sync::OnceLock;

use crate::core::config::GraphicsBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackendKind {
    Vulkan,
    Dx12,
    Wgpu,
}

static RENDERER_BACKEND: OnceLock<RendererBackendKind> = OnceLock::new();

pub fn init_renderer_backend(backend: GraphicsBackend) {
    let kind = match backend {
        GraphicsBackend::Vulkan => RendererBackendKind::Vulkan,
        GraphicsBackend::Dx12 => RendererBackendKind::Dx12,
        GraphicsBackend::Wgpu => RendererBackendKind::Wgpu,
    };

    let _ = RENDERER_BACKEND.set(kind);
}

pub fn renderer_backend() -> Option<RendererBackendKind> {
    RENDERER_BACKEND.get().copied()
}
