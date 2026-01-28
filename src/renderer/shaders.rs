// 从编译好的 SPIR-V 字节码加载 shader
// 这些 .spv 文件由 build.rs 从 vertex.hlsl 和 fragment.hlsl 编译生成
//
// 注意：这里使用 include_bytes! 宏在编译时嵌入 SPIR-V 二进制数据

use vulkano::shader::ShaderModule;
use std::sync::Arc;
use vulkano::device::Device;

pub mod vs {
    use super::*;

    /// 加载顶点着色器
    /// 从 vertex.hlsl 编译的 SPIR-V 字节码
    pub fn load(device: Arc<Device>) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
        // 在编译时嵌入 SPIR-V 文件
        let spirv = include_bytes!("shaders/spirv_code/vertex.hlsl.spv");

        unsafe {
            ShaderModule::from_bytes(device, spirv)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
    }
}

pub mod fs {
    use super::*;

    /// 加载片段着色器
    /// 从 fragment.hlsl 编译的 SPIR-V 字节码
    pub fn load(device: Arc<Device>) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
        // 在编译时嵌入 SPIR-V 文件
        let spirv = include_bytes!("shaders/spirv_code/fragment.hlsl.spv");

        unsafe {
            ShaderModule::from_bytes(device, spirv)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
    }
}
