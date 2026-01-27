/// 网格数据结构模块
///
/// 定义CPU侧的网格数据容器，用于存储从文件加载的原始几何数据。
/// 对应 DistEngine 的 MeshData 和 Subset 结构。

use super::vertex::Vertex;

/// 子网格描述符
///
/// 描述网格的一个子集，通常对应一个材质或一个独立的渲染批次。
/// 在多材质模型中，每个材质对应一个子网格。
///
/// # 示例
///
/// ```rust
/// use distrender::geometry::mesh::Subset;
///
/// // 描述一个包含100个顶点、50个三角形的子网格
/// let subset = Subset {
///     id: 0,
///     vertex_start: 0,
///     vertex_count: 100,
///     face_start: 0,
///     face_count: 50,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subset {
    /// 子网格ID（通常对应材质ID）
    pub id: u32,

    /// 起始顶点索引
    ///
    /// 在顶点数组中的起始位置。
    pub vertex_start: u32,

    /// 顶点数量
    ///
    /// 该子网格包含的顶点数。
    pub vertex_count: u32,

    /// 起始面索引
    ///
    /// 在索引数组中的起始位置（以三角形为单位）。
    pub face_start: u32,

    /// 面数量（三角形数量）
    ///
    /// 该子网格包含的三角形数。
    pub face_count: u32,
}

impl Subset {
    /// 创建一个新的子网格描述符
    #[inline]
    pub fn new(
        id: u32,
        vertex_start: u32,
        vertex_count: u32,
        face_start: u32,
        face_count: u32,
    ) -> Self {
        Self {
            id,
            vertex_start,
            vertex_count,
            face_start,
            face_count,
        }
    }

    /// 获取索引起始位置（以索引数量计，非三角形数）
    ///
    /// 由于每个三角形有3个索引，索引起始位置 = face_start * 3。
    #[inline]
    pub fn index_start(&self) -> u32 {
        self.face_start * 3
    }

    /// 获取索引数量（非三角形数）
    ///
    /// 由于每个三角形有3个索引，索引数量 = face_count * 3。
    #[inline]
    pub fn index_count(&self) -> u32 {
        self.face_count * 3
    }
}

/// CPU侧网格数据
///
/// 存储从文件加载的原始网格数据，包含顶点、索引和子网格信息。
/// 这是一个简单的数据持有者，不包含GPU资源。
///
/// # 架构说明
///
/// - **CPU侧**: `MeshData` 存储在内存中的原始数据
/// - **GPU侧**: 渲染器将 `MeshData` 上传到GPU缓冲区
///
/// # 示例
///
/// ```rust
/// use distrender::geometry::mesh::MeshData;
/// use distrender::geometry::vertex::Vertex;
///
/// let mesh = MeshData {
///     vertices: vec![
///         Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0], [1.0, 0.0, 0.0]),
///         Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0], [1.0, 0.0, 0.0]),
///         Vertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0], [1.0, 0.0, 0.0]),
///     ],
///     indices: vec![0, 1, 2],
///     subsets: vec![],
///     name: Some("Triangle".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct MeshData {
    /// 顶点数组
    ///
    /// 存储所有顶点的位置、法线、UV和切线数据。
    pub vertices: Vec<Vertex>,

    /// 索引数组
    ///
    /// 三角形顶点索引，每3个索引定义一个三角形。
    /// 使用32位索引以支持超过65535个顶点的模型。
    pub indices: Vec<u32>,

    /// 子网格列表
    ///
    /// 用于多材质模型，每个子网格对应一个材质。
    /// 如果模型只有一个材质，此列表可以为空。
    pub subsets: Vec<Subset>,

    /// 网格名称（可选）
    ///
    /// 从文件中读取的网格名称，用于调试和识别。
    pub name: Option<String>,
}

impl MeshData {
    /// 创建一个空的网格数据
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            subsets: Vec::new(),
            name: None,
        }
    }

    /// 创建一个指定名称的空网格数据
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            subsets: Vec::new(),
            name: Some(name.into()),
        }
    }

    /// 创建一个带容量预分配的网格数据
    ///
    /// # 参数
    ///
    /// - `vertex_capacity`: 预分配的顶点数量
    /// - `index_capacity`: 预分配的索引数量
    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
            subsets: Vec::new(),
            name: None,
        }
    }

    /// 获取顶点数量
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// 获取索引数量
    #[inline]
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// 获取三角形数量
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// 验证网格数据的有效性
    ///
    /// 检查：
    /// - 索引数量是3的倍数（每个三角形3个顶点）
    /// - 所有索引都在有效范围内
    /// - 子网格描述符的范围有效
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 数据有效
    /// - `Err(String)`: 数据无效，返回错误描述
    pub fn validate(&self) -> Result<(), String> {
        // 检查索引数量
        if self.indices.len() % 3 != 0 {
            return Err(format!(
                "索引数量必须是3的倍数，当前为: {}",
                self.indices.len()
            ));
        }

        // 检查索引范围
        let vertex_count = self.vertices.len() as u32;
        for (i, &index) in self.indices.iter().enumerate() {
            if index >= vertex_count {
                return Err(format!(
                    "索引 {} 处的值 {} 超出顶点范围 (0-{})",
                    i,
                    index,
                    vertex_count - 1
                ));
            }
        }

        // 检查子网格描述符
        for (i, subset) in self.subsets.iter().enumerate() {
            if subset.vertex_start + subset.vertex_count > vertex_count {
                return Err(format!(
                    "子网格 {} 的顶点范围超出边界: start={}, count={}, total={}",
                    i, subset.vertex_start, subset.vertex_count, vertex_count
                ));
            }

            let triangle_count = self.triangle_count() as u32;
            if subset.face_start + subset.face_count > triangle_count {
                return Err(format!(
                    "子网格 {} 的面范围超出边界: start={}, count={}, total={}",
                    i, subset.face_start, subset.face_count, triangle_count
                ));
            }
        }

        Ok(())
    }

    /// 清空所有数据
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.subsets.clear();
        self.name = None;
    }
}

impl Default for MeshData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subset_creation() {
        let subset = Subset::new(0, 0, 100, 0, 50);

        assert_eq!(subset.id, 0);
        assert_eq!(subset.vertex_start, 0);
        assert_eq!(subset.vertex_count, 100);
        assert_eq!(subset.face_start, 0);
        assert_eq!(subset.face_count, 50);
    }

    #[test]
    fn test_subset_index_helpers() {
        let subset = Subset::new(0, 0, 100, 10, 20);

        assert_eq!(subset.index_start(), 30); // 10 * 3
        assert_eq!(subset.index_count(), 60); // 20 * 3
    }

    #[test]
    fn test_mesh_data_creation() {
        let mesh = MeshData::new();

        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.index_count(), 0);
        assert_eq!(mesh.triangle_count(), 0);
        assert!(mesh.name.is_none());
    }

    #[test]
    fn test_mesh_data_with_name() {
        let mesh = MeshData::with_name("TestMesh");

        assert_eq!(mesh.name, Some("TestMesh".to_string()));
    }

    #[test]
    fn test_mesh_data_with_capacity() {
        let mesh = MeshData::with_capacity(100, 300);

        assert_eq!(mesh.vertices.capacity(), 100);
        assert_eq!(mesh.indices.capacity(), 300);
    }

    #[test]
    fn test_mesh_data_counts() {
        let mut mesh = MeshData::new();
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.indices.extend_from_slice(&[0, 1, 2]);

        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.index_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_mesh_data_validation_valid() {
        let mut mesh = MeshData::new();
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.indices.extend_from_slice(&[0, 1, 2]);

        assert!(mesh.validate().is_ok());
    }

    #[test]
    fn test_mesh_data_validation_invalid_index_count() {
        let mut mesh = MeshData::new();
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.indices.extend_from_slice(&[0, 1]); // 不是3的倍数

        assert!(mesh.validate().is_err());
    }

    #[test]
    fn test_mesh_data_validation_invalid_index_range() {
        let mut mesh = MeshData::new();
        mesh.vertices.push(Vertex::default());
        mesh.vertices.push(Vertex::default());
        mesh.indices.extend_from_slice(&[0, 1, 5]); // 索引5超出范围

        let result = mesh.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超出顶点范围"));
    }

    #[test]
    fn test_mesh_data_clear() {
        let mut mesh = MeshData::with_name("Test");
        mesh.vertices.push(Vertex::default());
        mesh.indices.push(0);

        mesh.clear();

        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.index_count(), 0);
        assert!(mesh.name.is_none());
    }
}
