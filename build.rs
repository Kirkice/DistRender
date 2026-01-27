use std::process::Command;
use std::path::Path;
use std::env;

/// Build script to compile HLSL shaders to SPIR-V (for Vulkan) and DXIL (for DX12)
fn main() {
    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Shader source and output paths
    let shader_source = Path::new(&manifest_dir).join("src/renderer/shaders/shader.hlsl");
    let spirv_vs_out = Path::new(&out_dir).join("shader_vs.spv");
    let spirv_ps_out = Path::new(&out_dir).join("shader_ps.spv");
    let dxil_vs_out = Path::new(&out_dir).join("shader_vs.dxil");
    let dxil_ps_out = Path::new(&out_dir).join("shader_ps.dxil");

    println!("cargo:rerun-if-changed=src/renderer/shaders/shader.hlsl");

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
    println!("cargo:warning=Compiling HLSL vertex shader to SPIR-V...");
    let vs_spirv_result = Command::new("dxc")
        .args(&[
            "-T", "vs_6_0",
            "-E", "VSMain",
            "-spirv",
            "-Fo", &spirv_vs_out.to_string_lossy(),
            &shader_source.to_string_lossy(),
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
    println!("cargo:warning=Compiling HLSL pixel shader to SPIR-V...");
    let ps_spirv_result = Command::new("dxc")
        .args(&[
            "-T", "ps_6_0",
            "-E", "PSMain",
            "-spirv",
            "-Fo", &spirv_ps_out.to_string_lossy(),
            &shader_source.to_string_lossy(),
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

    // Compile HLSL to DXIL for DX12 (Vertex Shader)
    println!("cargo:warning=Compiling HLSL vertex shader to DXIL...");
    let vs_dxil_result = Command::new("dxc")
        .args(&[
            "-T", "vs_6_0",
            "-E", "VSMain",
            "-Fo", &dxil_vs_out.to_string_lossy(),
            &shader_source.to_string_lossy(),
        ])
        .output();

    match vs_dxil_result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Vertex shader DXIL compilation successful");
        }
        Ok(output) => {
            println!("cargo:warning=Vertex shader DXIL compilation failed:");
            println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("cargo:warning=Failed to run DXC for vertex DXIL: {}", e);
        }
    }

    // Compile HLSL to DXIL for DX12 (Pixel Shader)
    println!("cargo:warning=Compiling HLSL pixel shader to DXIL...");
    let ps_dxil_result = Command::new("dxc")
        .args(&[
            "-T", "ps_6_0",
            "-E", "PSMain",
            "-Fo", &dxil_ps_out.to_string_lossy(),
            &shader_source.to_string_lossy(),
        ])
        .output();

    match ps_dxil_result {
        Ok(output) if output.status.success() => {
            println!("cargo:warning=Pixel shader DXIL compilation successful");
        }
        Ok(output) => {
            println!("cargo:warning=Pixel shader DXIL compilation failed:");
            println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("cargo:warning=Failed to run DXC for pixel DXIL: {}", e);
        }
    }
}