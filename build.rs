use std::process::Command;
use std::path::Path;
use std::env;

/// Build script to compile HLSL shaders to SPIR-V (for Vulkan) and DXIL (for DX12)
fn main() {
    // Get the directories
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let shader_dir = Path::new(&manifest_dir).join("src/renderer/shaders");
    let spirv_output_dir = shader_dir.join("spirv_code");

    // Ensure spirv_code directory exists
    std::fs::create_dir_all(&spirv_output_dir).unwrap();

    // Shader source and output paths
    let vertex_shader = shader_dir.join("vertex.hlsl");
    let fragment_shader = shader_dir.join("fragment.hlsl");
    let spirv_vs_out = spirv_output_dir.join("vertex.hlsl.spv");
    let spirv_ps_out = spirv_output_dir.join("fragment.hlsl.spv");

    println!("cargo:rerun-if-changed=src/renderer/shaders/vertex.hlsl");
    println!("cargo:rerun-if-changed=src/renderer/shaders/fragment.hlsl");

    // Check if DXC is available
    let dxc_available = Command::new("dxc")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !dxc_available {
        println!("cargo:warning=DXC (DirectX Shader Compiler) not found. Please install DXC to compile HLSL shaders.");
        println!("cargo:warning=On Windows, you can install it via Windows SDK or download from Microsoft.");
        println!("cargo:warning=On Linux/macOS, you can install via Vulkan SDK or build from source.");
        return;
    }

    // Compile HLSL to SPIR-V for Vulkan (Vertex Shader)
    println!("cargo:warning=Compiling vertex.hlsl to SPIR-V...");
    let vs_spirv_result = Command::new("dxc")
        .args([
            "-T", "vs_6_0",
            "-E", "VSMain",
            "-spirv",
            "-Fo", &spirv_vs_out.to_string_lossy(),
            &vertex_shader.to_string_lossy(),
        ])
        .output();

    match vs_spirv_result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Vertex shader SPIR-V compilation successful");
        }
        Ok(output) => {
            println!("cargo:warning=Vertex shader SPIR-V compilation failed:");
            println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("cargo:warning=Failed to run DXC for vertex shader: {}", e);
        }
    }

    // Compile HLSL to SPIR-V for Vulkan (Pixel Shader)
    println!("cargo:warning=Compiling fragment.hlsl to SPIR-V...");
    let ps_spirv_result = Command::new("dxc")
        .args([
            "-T", "ps_6_0",
            "-E", "PSMain",
            "-spirv",
            "-Fo", &spirv_ps_out.to_string_lossy(),
            &fragment_shader.to_string_lossy(),
        ])
        .output();

    match ps_spirv_result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Pixel shader SPIR-V compilation successful");
        }
        Ok(output) => {
            println!("cargo:warning=Pixel shader SPIR-V compilation failed:");
            println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("cargo:warning=Failed to run DXC for pixel shader: {}", e);
        }
    }

    println!("cargo:warning=Shader compilation complete. SPIR-V files will be used by Vulkan backend.");
    println!("cargo:warning=DX12 backend will compile HLSL at runtime using D3DCompile.");
}