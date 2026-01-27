/// 几何体加载和处理模块
///
/// 提供3D模型加载功能，支持多种文件格式（OBJ、FBX等）。
/// 包含顶点定义、网格数据结构以及几何处理工具。
///
/// # 模块结构
///
/// - `vertex`: 顶点数据结构定义
/// - `mesh`: 网格数据和子网格结构
/// - `math_utils`: 几何数学工具（法线计算、切线空间等）
/// - `loaders`: 各种格式的模型加载器
///
/// # 架构设计
///
/// ```text
/// 文件 (OBJ/FBX)
///     ↓
/// Loader (ObjLoader/FbxLoader)
///     ↓
/// MeshData (CPU侧数据)
///     ↓
/// Renderer (上传到GPU)
/// ```
///
/// # 使用示例
///
/// ```rust,no_run
/// use distrender::geometry::loaders::{MeshLoader, ObjLoader};
/// use std::path::Path;
///
/// // 加载OBJ模型
/// let mesh_data = ObjLoader::load_from_file(Path::new("model.obj"))?;
///
/// println!("顶点数: {}", mesh_data.vertex_count());
/// println!("三角形数: {}", mesh_data.triangle_count());
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```

pub mod vertex;
pub mod mesh;
pub mod math_utils;
pub mod loaders;

// 重新导出常用类型
pub use vertex::Vertex;
pub use mesh::{MeshData, Subset};
