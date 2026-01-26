//! 着色器定义
//!
//! 本模块包含了渲染管线使用的着色器程序（Shaders）。
//! 着色器是在 GPU 上运行的小程序，用于处理顶点和像素的渲染。
//!
//! # 着色器类型
//!
//! - **顶点着色器（Vertex Shader）**：处理每个顶点的位置和属性
//! - **片段着色器（Fragment Shader）**：计算每个像素的最终颜色
//!
//! # 实现方式
//!
//! 使用 Vulkano 的 `vulkano_shaders::shader!` 宏编译内联的 GLSL 代码。
//! 这种方式的优点：
//! - 编译时检查着色器语法
//! - 自动生成 Rust 类型绑定
//! - 无需管理外部着色器文件
//!
//! # 渲染管线
//!
//! ```text
//! 顶点数据 -> 顶点着色器 -> 图元装配 -> 光栅化 -> 片段着色器 -> 帧缓冲
//! ```

/// 顶点着色器模块
///
/// 顶点着色器负责处理顶点数据的变换和传递。
///
/// # 输入
///
/// - `position`：顶点的 2D 位置（location = 0）
/// - `color`：顶点颜色（location = 1）
///
/// # 输出
///
/// - `gl_Position`：转换后的顶点位置（裁剪空间坐标）
/// - `fragColor`：传递给片段着色器的颜色（location = 0）
///
/// # 实现细节
///
/// 将 2D 坐标扩展为 4D 齐次坐标 (x, y, 0.0, 1.0)，
/// 并将顶点颜色原封不动地传递给片段着色器。
pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450

            // 输入：顶点属性
            layout(location = 0) in vec2 position;  // 顶点位置
            layout(location = 1) in vec3 color;     // 顶点颜色

            // 输出：传递给片段着色器的数据
            layout(location = 0) out vec3 fragColor;

            void main() {
                // 将 2D 位置转换为 4D 齐次坐标
                // z = 0.0 表示在屏幕平面上
                // w = 1.0 表示这是一个位置向量（而非方向向量）
                gl_Position = vec4(position, 0.0, 1.0);

                // 将顶点颜色传递给片段着色器
                // 光栅化阶段会自动在顶点之间插值这个颜色
                fragColor = color;
            }
        "
    }
}

/// 片段着色器模块
///
/// 片段着色器负责计算每个像素的最终颜色。
///
/// # 输入
///
/// - `fragColor`：从顶点着色器插值得到的颜色（location = 0）
///
/// # 输出
///
/// - `f_color`：像素的最终颜色（RGBA，location = 0）
///
/// # 实现细节
///
/// 直接使用插值后的颜色，添加 alpha 通道值 1.0（完全不透明）。
/// 由于使用了顶点颜色插值，三角形会自动产生平滑的颜色渐变效果。
pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450

            // 输入：从顶点着色器传来的插值颜色
            layout(location = 0) in vec3 fragColor;

            // 输出：最终的像素颜色
            layout(location = 0) out vec4 f_color;

            void main() {
                // 将 RGB 颜色扩展为 RGBA
                // alpha = 1.0 表示完全不透明
                f_color = vec4(fragColor, 1.0);
            }
        "
    }
}
