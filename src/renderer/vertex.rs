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

// 实现 Vulkano 的顶点特征
// 这个宏会自动生成必要的代码，告诉 Vulkan 如何解释顶点数据
vulkano::impl_vertex!(MyVertex, position, color);

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
