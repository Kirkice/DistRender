/// Build script for DistRender
///
/// # Shader Compilation Strategy:
/// - Vulkan: Uses GLSL shaders compiled at build time via vulkano_shaders macro
/// - DX12: Uses HLSL shaders compiled at runtime via D3DCompile
fn main() {
    // Trigger rebuild if shader files change
    println!("cargo:rerun-if-changed=src/gfx/vulkan/shaders/vertex.glsl");
    println!("cargo:rerun-if-changed=src/gfx/vulkan/shaders/fragment.glsl");
    println!("cargo:rerun-if-changed=src/gfx/dx12/shaders/vertex.hlsl");
    println!("cargo:rerun-if-changed=src/gfx/dx12/shaders/fragment.hlsl");
}
