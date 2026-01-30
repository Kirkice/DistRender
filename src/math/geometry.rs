//! 几何数学工具模块
//!
//! 提供网格处理相关的数学函数，包括：
//! - 法线重建（从三角形面计算顶点法线）
//! - 切线空间计算（用于法线贴图）
//! - 法线平滑
//!
//! 这些函数用于后处理加载的网格数据。

use crate::geometry::vertex::Vertex;

/// 从三角形面重建顶点法线
///
/// 遍历所有三角形，计算每个面的法线，然后将面法线累加到该面的三个顶点。
/// 最后归一化所有顶点的法线向量。
///
/// # 算法
///
/// 1. 对于每个三角形 (v0, v1, v2):
///    - 计算边向量: edge1 = v1.position - v0.position
///    - 计算边向量: edge2 = v2.position - v0.position
///    - 计算面法线: face_normal = normalize(cross(edge1, edge2))
///    - 累加面法线到三个顶点
///
/// 2. 归一化所有顶点法线
///
/// # 参数
///
/// - `vertices`: 顶点数组（可变引用，法线字段将被更新）
/// - `indices`: 索引数组（每3个索引定义一个三角形）
///
/// # 示例
///
/// ```rust
/// use distrender::math::geometry::reconstruct_normals;
/// use distrender::geometry::vertex::Vertex;
///
/// let mut vertices = vec![
///     Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0], [0.0, 0.0, 0.0]),
///     Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0], [0.0, 0.0, 0.0]),
///     Vertex::new([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0], [0.0, 0.0, 0.0]),
/// ];
/// let indices = vec![0, 1, 2];
///
/// reconstruct_normals(&mut vertices, &indices);
///
/// // 现在所有顶点都有正确的法线
/// ```
pub fn reconstruct_normals(vertices: &mut [Vertex], indices: &[u32]) {
    // 首先将所有法线重置为零
    for vertex in vertices.iter_mut() {
        vertex.normal = [0.0, 0.0, 0.0];
    }

    // 遍历所有三角形
    for triangle in indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        // 获取三个顶点的位置
        let p0 = vertices[i0].position;
        let p1 = vertices[i1].position;
        let p2 = vertices[i2].position;

        // 计算边向量
        let edge1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let edge2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

        // 计算面法线（叉乘）
        let face_normal = cross(edge1, edge2);

        // 累加面法线到三个顶点
        vertices[i0].normal[0] += face_normal[0];
        vertices[i0].normal[1] += face_normal[1];
        vertices[i0].normal[2] += face_normal[2];

        vertices[i1].normal[0] += face_normal[0];
        vertices[i1].normal[1] += face_normal[1];
        vertices[i1].normal[2] += face_normal[2];

        vertices[i2].normal[0] += face_normal[0];
        vertices[i2].normal[1] += face_normal[1];
        vertices[i2].normal[2] += face_normal[2];
    }

    // 归一化所有顶点法线
    for vertex in vertices.iter_mut() {
        vertex.normal = normalize(vertex.normal);
    }
}

/// 根据位置平滑法线
///
/// 对于位置相近（在epsilon范围内）的顶点，将它们的法线平均化。
/// 这用于消除硬边，创建平滑的表面。
///
/// # 参数
///
/// - `vertices`: 顶点数组（可变引用，法线字段将被更新）
/// - `epsilon`: 位置相似度阈值
pub fn smooth_normals_by_position(vertices: &mut [Vertex], epsilon: f32) {
    if vertices.is_empty() {
        return;
    }

    let eps = epsilon.max(0.0);
    if eps == 0.0 {
        return;
    }

    fn quantize(v: [f32; 3], eps: f32) -> (i32, i32, i32) {
        (
            (v[0] / eps).round() as i32,
            (v[1] / eps).round() as i32,
            (v[2] / eps).round() as i32,
        )
    }

    use std::collections::HashMap;

    let mut reference: HashMap<(i32, i32, i32), [f32; 3]> = HashMap::new();
    let mut sums: HashMap<(i32, i32, i32), [f32; 3]> = HashMap::new();

    for v in vertices.iter() {
        let key = quantize(v.position, eps);
        let ref_n = reference.entry(key).or_insert(v.normal);

        let mut n = v.normal;
        if dot(*ref_n, n) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }

        let entry = sums.entry(key).or_insert([0.0, 0.0, 0.0]);
        entry[0] += n[0];
        entry[1] += n[1];
        entry[2] += n[2];
    }

    for v in vertices.iter_mut() {
        let key = quantize(v.position, eps);
        if let Some(sum) = sums.get(&key) {
            v.normal = normalize(*sum);
        }
    }
}

/// 计算顶点的切线空间向量
///
/// 使用UV坐标导数计算每个顶点的切线向量，用于法线贴图。
/// 切线向量与法线正交，指向UV坐标U增加的方向。
///
/// # 算法
///
/// 1. 对于每个三角形 (v0, v1, v2):
///    - 计算位置导数: dp1 = v1.position - v0.position, dp2 = v2.position - v0.position
///    - 计算UV导数: duv1 = v1.texcoord - v0.texcoord, duv2 = v2.texcoord - v0.texcoord
///    - 计算行列式: r = 1.0 / (duv1.x * duv2.y - duv1.y * duv2.x)
///    - 计算切线: tangent = (dp1 * duv2.y - dp2 * duv1.y) * r
///    - 累加切线到三个顶点
///
/// 2. 对每个顶点进行 Gram-Schmidt 正交化:
///    - tangent = normalize(tangent - normal * dot(normal, tangent))
///
/// # 参数
///
/// - `vertices`: 顶点数组（可变引用，切线字段将被更新）
/// - `indices`: 索引数组（每3个索引定义一个三角形）
///
/// # 前置条件
///
/// - 顶点必须已经有有效的法线向量（可通过 `reconstruct_normals` 生成）
/// - 顶点必须有UV坐标
///
/// # 示例
///
/// ```rust
/// use distrender::math::geometry::compute_tangent_space;
/// use distrender::geometry::vertex::Vertex;
///
/// let mut vertices = vec![
///     Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0], [0.0, 0.0, 0.0]),
///     Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0], [0.0, 0.0, 0.0]),
///     Vertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0], [0.0, 0.0, 0.0]),
/// ];
/// let indices = vec![0, 1, 2];
///
/// compute_tangent_space(&mut vertices, &indices);
///
/// // 现在所有顶点都有正确的切线向量
/// ```
pub fn compute_tangent_space(vertices: &mut [Vertex], indices: &[u32]) {
    // 首先将所有切线重置为零
    for vertex in vertices.iter_mut() {
        vertex.tangent = [0.0, 0.0, 0.0];
    }

    // 遍历所有三角形
    for triangle in indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        // 获取三个顶点
        let v0 = &vertices[i0];
        let v1 = &vertices[i1];
        let v2 = &vertices[i2];

        // 位置导数
        let dp1 = [
            v1.position[0] - v0.position[0],
            v1.position[1] - v0.position[1],
            v1.position[2] - v0.position[2],
        ];
        let dp2 = [
            v2.position[0] - v0.position[0],
            v2.position[1] - v0.position[1],
            v2.position[2] - v0.position[2],
        ];

        // UV 导数
        let duv1 = [
            v1.texcoord[0] - v0.texcoord[0],
            v1.texcoord[1] - v0.texcoord[1],
        ];
        let duv2 = [
            v2.texcoord[0] - v0.texcoord[0],
            v2.texcoord[1] - v0.texcoord[1],
        ];

        // 计算行列式
        let det = duv1[0] * duv2[1] - duv1[1] * duv2[0];

        // 避免除以零
        if det.abs() < 1e-6 {
            continue;
        }

        let r = 1.0 / det;

        // 计算切线向量
        let tangent = [
            (dp1[0] * duv2[1] - dp2[0] * duv1[1]) * r,
            (dp1[1] * duv2[1] - dp2[1] * duv1[1]) * r,
            (dp1[2] * duv2[1] - dp2[2] * duv1[1]) * r,
        ];

        // 累加切线到三个顶点
        vertices[i0].tangent[0] += tangent[0];
        vertices[i0].tangent[1] += tangent[1];
        vertices[i0].tangent[2] += tangent[2];

        vertices[i1].tangent[0] += tangent[0];
        vertices[i1].tangent[1] += tangent[1];
        vertices[i1].tangent[2] += tangent[2];

        vertices[i2].tangent[0] += tangent[0];
        vertices[i2].tangent[1] += tangent[1];
        vertices[i2].tangent[2] += tangent[2];
    }

    // Gram-Schmidt 正交化并归一化所有切线
    for vertex in vertices.iter_mut() {
        let normal = vertex.normal;
        let tangent = vertex.tangent;

        // Gram-Schmidt: tangent = tangent - normal * dot(normal, tangent)
        let dot_nt = dot(normal, tangent);
        let orthogonal_tangent = [
            tangent[0] - normal[0] * dot_nt,
            tangent[1] - normal[1] * dot_nt,
            tangent[2] - normal[2] * dot_nt,
        ];

        // 归一化
        vertex.tangent = normalize(orthogonal_tangent);
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 辅助函数：计算两个3D向量的叉乘
///
/// 返回垂直于两个输入向量的向量，长度等于两向量张成的平行四边形面积。
#[inline]
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 辅助函数：计算两个3D向量的点乘
///
/// 返回两个向量的标量积。
#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 辅助函数：归一化3D向量
///
/// 返回与输入向量同方向的单位向量（长度为1）。
/// 如果输入向量长度为零，返回零向量。
#[inline]
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();

    if length < 1e-6 {
        [0.0, 0.0, 0.0]
    } else {
        let inv_length = 1.0 / length;
        [v[0] * inv_length, v[1] * inv_length, v[2] * inv_length]
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross() {
        // X 叉乘 Y = Z
        let x = [1.0, 0.0, 0.0];
        let y = [0.0, 1.0, 0.0];
        let z = cross(x, y);

        assert!((z[0] - 0.0).abs() < 1e-6);
        assert!((z[1] - 0.0).abs() < 1e-6);
        assert!((z[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let result = dot(a, b);

        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!((result - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let v = [3.0, 4.0, 0.0];
        let normalized = normalize(v);

        // 长度应该为 1
        let length = (normalized[0] * normalized[0] +
                     normalized[1] * normalized[1] +
                     normalized[2] * normalized[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-6);

        // 方向应该保持
        assert!((normalized[0] - 0.6).abs() < 1e-6);
        assert!((normalized[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero() {
        let v = [0.0, 0.0, 0.0];
        let normalized = normalize(v);

        assert_eq!(normalized, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_reconstruct_normals_simple_triangle() {
        // 创建一个简单的三角形在 XZ 平面上
        let mut vertices = vec![
            Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0], [0.0, 0.0, 0.0]),
            Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0], [0.0, 0.0, 0.0]),
            Vertex::new([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0], [0.0, 0.0, 0.0]),
        ];
        let indices = vec![0, 1, 2];

        reconstruct_normals(&mut vertices, &indices);

        // 所有顶点的法线应该指向 ±Y（垂直于XZ平面）
        // 方向取决于三角形绕序，这里接受任一方向
        for vertex in &vertices {
            assert!(vertex.normal[1].abs() > 0.9, "法线Y分量的绝对值应该接近1: {:?}", vertex.normal);
            assert!(vertex.normal[0].abs() < 0.1, "法线X分量应该接近0: {:?}", vertex.normal);
            assert!(vertex.normal[2].abs() < 0.1, "法线Z分量应该接近0: {:?}", vertex.normal);

            // 验证法线已归一化
            let length = (vertex.normal[0] * vertex.normal[0] +
                         vertex.normal[1] * vertex.normal[1] +
                         vertex.normal[2] * vertex.normal[2]).sqrt();
            assert!((length - 1.0).abs() < 0.01, "法线应该是单位向量: length = {}", length);
        }
    }

    #[test]
    fn test_compute_tangent_space_simple() {
        // 创建一个简单的三角形，带有法线和UV
        let mut vertices = vec![
            Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0], [0.0, 0.0, 0.0]),
            Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0], [0.0, 0.0, 0.0]),
            Vertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0], [0.0, 0.0, 0.0]),
        ];
        let indices = vec![0, 1, 2];

        compute_tangent_space(&mut vertices, &indices);

        // 验证切线已计算（不为零）
        for vertex in &vertices {
            let tangent_length = (vertex.tangent[0] * vertex.tangent[0] +
                                 vertex.tangent[1] * vertex.tangent[1] +
                                 vertex.tangent[2] * vertex.tangent[2]).sqrt();
            assert!(tangent_length > 0.5, "切线应该已归一化: {:?}", vertex.tangent);

            // 验证切线与法线正交（点乘应接近0）
            let dot_product = dot(vertex.normal, vertex.tangent);
            assert!(dot_product.abs() < 0.01, "切线应该与法线正交: dot = {}", dot_product);
        }
    }
}
