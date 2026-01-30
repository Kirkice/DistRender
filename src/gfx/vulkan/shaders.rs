// Vulkan shader 加载模块
// 使用 vulkano_shaders 宏从 GLSL 源文件编译 shader

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/gfx/vulkan/shaders/vertex.glsl",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/gfx/vulkan/shaders/fragment.glsl",
    }
}
