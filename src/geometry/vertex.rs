/// 几何体顶点定义模块
///
/// 定义用于3D模型加载的完整顶点结构，包含位置、法线、UV坐标和切线向量。
/// 对应 DistEngine 的 Vertex 结构。

use bytemuck::{Pod, Zeroable};

/// 完整的3D顶点结构
///
/// 包含所有常用的顶点属性，用于支持光照、纹理映射和法线贴图。
/// 内存布局与GPU兼容，使用 `#[repr(C)]` 保证顺序和对齐。
///
/// # 内存布局
///
/// - position: 12 bytes (3 * f32)
/// - normal: 12 bytes (3 * f32)
/// - texcoord: 8 bytes (2 * f32)
/// - tangent: 12 bytes (3 * f32)
/// - **总计**: 44 bytes
///
/// # 示例
///
/// ```rust
/// use distrender::geometry::vertex::Vertex;
///
/// let vertex = Vertex {
///     position: [0.0, 1.0, 0.0],
///     normal: [0.0, 1.0, 0.0],
///     texcoord: [0.5, 0.5],
///     tangent: [1.0, 0.0, 0.0],
/// };
/// ```
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// 顶点位置 (x, y, z)
    ///
    /// 3D空间中的顶点坐标。
    pub position: [f32; 3],

    /// 法线向量 (nx, ny, nz)
    ///
    /// 用于光照计算的表面法线，应该是归一化的单位向量。
    pub normal: [f32; 3],

    /// 纹理坐标 (u, v)
    ///
    /// UV坐标用于纹理映射，通常范围在 [0.0, 1.0]。
    pub texcoord: [f32; 2],

    /// 切线向量 (tx, ty, tz)
    ///
    /// 用于法线贴图的切线空间计算，应该与法线正交且归一化。
    pub tangent: [f32; 3],
}

impl Vertex {
    /// 创建一个新的顶点
    ///
    /// # 参数
    ///
    /// - `position`: 3D位置坐标
    /// - `normal`: 法线向量
    /// - `texcoord`: UV纹理坐标
    /// - `tangent`: 切线向量
    #[inline]
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        texcoord: [f32; 2],
        tangent: [f32; 3],
    ) -> Self {
        Self {
            position,
            normal,
            texcoord,
            tangent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_vertex_size() {
        // 验证顶点结构的大小
        // 3*4 + 3*4 + 2*4 + 3*4 = 44 bytes
        assert_eq!(size_of::<Vertex>(), 44);
    }

    #[test]
    fn test_vertex_alignment() {
        // 验证顶点结构的对齐
        assert_eq!(std::mem::align_of::<Vertex>(), 4);
    }

    #[test]
    fn test_vertex_creation() {
        let vertex = Vertex::new(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.5],
            [1.0, 0.0, 0.0],
        );

        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vertex.texcoord, [0.5, 0.5]);
        assert_eq!(vertex.tangent, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_vertex_default() {
        let vertex = Vertex::default();

        assert_eq!(vertex.position, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.normal, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.texcoord, [0.0, 0.0]);
        assert_eq!(vertex.tangent, [0.0, 0.0, 0.0]);
    }
}
