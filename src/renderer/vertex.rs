//! 顶点数据定义
//!
//! 本模块定义了渲染管线使用的顶点结构体。
//! 顶点是 3D 图形的基本构建块，包含位置、颜色、纹理坐标等属性。
//!
//! # 设计说明
//!
//! - 使用 `#[repr(C)]` 确保内存布局与 C/C++ 兼容
//! - 实现 `Pod` 和 `Zeroable` trait 以支持零拷贝传输到 GPU
//! - 使用 Vulkano 的 `impl_vertex!` 宏自动实现 Vulkan 顶点输入绑定
//! - 使用数学库的 Vector 类型确保类型安全

use bytemuck::{Pod, Zeroable};
use crate::core::math::{Vector2, Vector3};

/// 顶点结构体
///
/// 定义了每个顶点的属性数据，包括位置和颜色。
/// 这个结构体会被直接传输到 GPU 的顶点缓冲区。
///
/// # 内存布局
///
/// 使用 `#[repr(C)]` 保证内存布局的一致性：
/// - `position`：前 8 字节（2 个 f32）
/// - `color`：后 12 字节（3 个 f32）
///
/// 总大小：20 字节
///
/// # 字段说明
///
/// - `position`：顶点在 2D 空间中的位置 [x, y]
/// - `color`：顶点的 RGB 颜色值，范围 [0.0, 1.0]
///
/// # 示例
///
/// ```
/// # use crate::renderer::vertex::MyVertex;
/// # use crate::core::math::{Vector2, Vector3};
/// // 使用数学库类型创建顶点
/// let vertex = MyVertex::from_vectors(
///     Vector2::new(0.0, 0.0),
///     Vector3::new(1.0, 0.0, 0.0)
/// );
/// ```
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
pub struct MyVertex {
    /// 顶点位置（2D 坐标）
    pub position: [f32; 2],
    /// 顶点颜色（RGB，范围 0.0-1.0）
    pub color: [f32; 3],
}

impl MyVertex {
    /// 从数学库的 Vector 类型创建顶点
    ///
    /// 这是一个便利方法，允许使用类型安全的 Vector 类型创建顶点，
    /// 然后自动转换为 GPU 兼容的数组格式。
    ///
    /// # 示例
    ///
    /// ```
    /// # use crate::renderer::vertex::MyVertex;
    /// # use crate::core::math::{Vector2, Vector3};
    /// let vertex = MyVertex::from_vectors(
    ///     Vector2::new(1.0, 2.0),
    ///     Vector3::new(1.0, 0.5, 0.0)
    /// );
    /// ```
    pub fn from_vectors(position: Vector2, color: Vector3) -> Self {
        Self {
            position: [position.x, position.y],
            color: [color.x, color.y, color.z],
        }
    }

    /// 创建一个新顶点（使用原始值）
    ///
    /// # 示例
    ///
    /// ```
    /// # use crate::renderer::vertex::MyVertex;
    /// let vertex = MyVertex::new(1.0, 2.0, 1.0, 0.5, 0.0);
    /// ```
    pub fn new(px: f32, py: f32, r: f32, g: f32, b: f32) -> Self {
        Self {
            position: [px, py],
            color: [r, g, b],
        }
    }
}

/// 创建默认的彩色三角形顶点数据
///
/// 这是一个便利函数，用于创建一个标准的演示三角形。
/// 三角形由三个顶点组成，分别为红色、绿色和蓝色。
///
/// # 返回值
///
/// 返回包含三个顶点的数组：
/// - 顶点 1：顶部中心 (0.0, 0.5)，红色
/// - 顶点 2：右下角 (0.5, -0.5)，绿色
/// - 顶点 3：左下角 (-0.5, -0.5)，蓝色
///
/// # 示例
///
/// ```
/// # use crate::renderer::vertex::create_default_triangle;
/// let vertices = create_default_triangle();
/// assert_eq!(vertices.len(), 3);
/// ```
pub fn create_default_triangle() -> [MyVertex; 3] {
    [
        MyVertex::from_vectors(
            Vector2::new(0.0, 0.5),
            Vector3::new(1.0, 0.0, 0.0)  // 红色
        ),
        MyVertex::from_vectors(
            Vector2::new(0.5, -0.5),
            Vector3::new(0.0, 1.0, 0.0)  // 绿色
        ),
        MyVertex::from_vectors(
            Vector2::new(-0.5, -0.5),
            Vector3::new(0.0, 0.0, 1.0)  // 蓝色
        ),
    ]
}

/// 将 GeometryVertex 转换为 MyVertex
///
/// 将 3D 模型顶点转换为 2D 渲染顶点。
/// - 使用 x, y 坐标作为 2D 位置
/// - 使用法线的绝对值作为颜色（简单的可视化方式）
///
/// # 参数
///
/// - `geo_vertex`: 3D 几何体顶点
///
/// # 返回值
///
/// 转换后的 2D 渲染顶点
pub fn convert_geometry_vertex(geo_vertex: &GeometryVertex) -> MyVertex {
    MyVertex {
        position: [geo_vertex.position[0], geo_vertex.position[1]],
        color: [
            geo_vertex.normal[0].abs(),
            geo_vertex.normal[1].abs(),
            geo_vertex.normal[2].abs(),
        ],
    }
}

// 实现 Vulkano 的顶点特征
// 这个宏会自动生成必要的代码，告诉 Vulkan 如何解释顶点数据
vulkano::impl_vertex!(MyVertex, position, color);

// ==================== 几何体顶点支持 ====================

/// 重新导出 geometry 模块的完整顶点定义
///
/// 这个顶点结构包含更多属性（位置、法线、UV、切线），
/// 用于加载和渲染 3D 模型（OBJ、FBX 等格式）。
pub use crate::geometry::vertex::Vertex as GeometryVertex;

// 为 GeometryVertex 实现 Vulkano 的顶点 trait
// 这使得 Vulkan 能够理解如何从顶点缓冲区读取这些属性
vulkano::impl_vertex!(GeometryVertex, position, normal, texcoord, tangent);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_vertex_layout() {
        // 验证顶点结构的大小和对齐
        assert_eq!(mem::size_of::<MyVertex>(), 20, "Vertex size should be 20 bytes");
        assert_eq!(mem::align_of::<MyVertex>(), 4, "Vertex alignment should be 4 bytes");

        // 验证字段偏移量
        let vertex = MyVertex::default();
        let vertex_ptr = &vertex as *const MyVertex as usize;
        let position_ptr = &vertex.position as *const [f32; 2] as usize;
        let color_ptr = &vertex.color as *const [f32; 3] as usize;

        assert_eq!(position_ptr - vertex_ptr, 0, "position should be at offset 0");
        assert_eq!(color_ptr - vertex_ptr, 8, "color should be at offset 8");
    }

    #[test]
    fn test_vertex_creation() {
        // 测试使用 Vector 类型创建顶点
        let vertex = MyVertex::from_vectors(
            Vector2::new(1.0, 2.0),
            Vector3::new(1.0, 0.5, 0.0)
        );

        assert_eq!(vertex.position[0], 1.0);
        assert_eq!(vertex.position[1], 2.0);
        assert_eq!(vertex.color[0], 1.0);
        assert_eq!(vertex.color[1], 0.5);
        assert_eq!(vertex.color[2], 0.0);
    }

    #[test]
    fn test_vertex_new() {
        // 测试使用原始值创建顶点
        let vertex = MyVertex::new(1.0, 2.0, 1.0, 0.5, 0.0);

        assert_eq!(vertex.position[0], 1.0);
        assert_eq!(vertex.position[1], 2.0);
        assert_eq!(vertex.color[0], 1.0);
        assert_eq!(vertex.color[1], 0.5);
        assert_eq!(vertex.color[2], 0.0);
    }

    #[test]
    fn test_pod_zeroable() {
        // 验证可以安全地零初始化
        let vertex: MyVertex = bytemuck::Zeroable::zeroed();
        assert_eq!(vertex.position[0], 0.0);
        assert_eq!(vertex.position[1], 0.0);
        assert_eq!(vertex.color[0], 0.0);
        assert_eq!(vertex.color[1], 0.0);
        assert_eq!(vertex.color[2], 0.0);

        // 验证可以转换为字节切片
        let vertex = MyVertex::from_vectors(
            Vector2::new(1.0, 2.0),
            Vector3::new(1.0, 0.5, 0.0)
        );
        let bytes: &[u8] = bytemuck::bytes_of(&vertex);
        assert_eq!(bytes.len(), 20);
    }
}
