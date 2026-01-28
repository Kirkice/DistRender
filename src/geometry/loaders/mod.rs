/// 模型加载器模块
///
/// 提供统一的模型加载接口和各种格式的具体实现。
///
/// # 支持的格式
///
/// - **OBJ**: Wavefront OBJ 格式（使用 tobj crate）
/// - **FBX**: Autodesk FBX 格式（使用 russimp/Assimp）
///
/// # 使用示例
///
/// ```rust,no_run
/// use distrender::geometry::loaders::{MeshLoader, ObjLoader};
/// use std::path::Path;
///
/// let mesh = ObjLoader::load_from_file(Path::new("model.obj"))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
use crate::core::error::Result;
use crate::geometry::mesh::MeshData;
use std::path::Path;

pub mod obj_loader;
pub mod fbx_loader;

// 重新导出加载器
pub use obj_loader::ObjLoader;
pub use fbx_loader::FbxLoader;

/// 网格加载器 trait
///
/// 定义统一的加载接口，所有格式的加载器都实现此 trait。
/// 这种设计允许轻松添加新的文件格式支持。
///
/// # 实现要求
///
/// - 加载器应该是无状态的（使用静态方法）
/// - 返回 CPU 侧的 `MeshData`，不涉及 GPU 资源
/// - 正确处理错误情况并返回有意义的错误信息
///
/// # 示例实现
///
/// ```rust,ignore
/// use distrender::geometry::loaders::MeshLoader;
///
/// pub struct MyLoader;
///
/// impl MeshLoader for MyLoader {
///     fn load_from_file(path: &Path) -> Result<MeshData> {
///         // 实现加载逻辑
///         todo!()
///     }
///
///     fn load_from_memory(data: &[u8]) -> Result<MeshData> {
///         // 实现从内存加载
///         todo!()
///     }
///
///     fn supported_extensions() -> &'static [&'static str] {
///         &["myformat"]
///     }
/// }
/// ```
pub trait MeshLoader {
    /// 从文件路径加载网格
    ///
    /// # 参数
    ///
    /// - `path`: 模型文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(MeshData)`: 加载成功，返回网格数据
    /// - `Err(DistRenderError)`: 加载失败（文件不存在、解析错误等）
    ///
    /// # 错误
    ///
    /// - 文件不存在或无法读取
    /// - 文件格式错误或损坏
    /// - 数据验证失败
    fn load_from_file(path: &Path) -> Result<MeshData>;

    /// 从内存数据加载网格
    ///
    /// # 参数
    ///
    /// - `data`: 文件内容的字节数组
    ///
    /// # 返回
    ///
    /// - `Ok(MeshData)`: 加载成功
    /// - `Err(DistRenderError)`: 解析失败
    fn load_from_memory(data: &[u8]) -> Result<MeshData>;

    /// 获取支持的文件扩展名列表
    ///
    /// # 返回
    ///
    /// 支持的扩展名数组（小写，不含点号）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// assert_eq!(ObjLoader::supported_extensions(), &["obj"]);
    /// ```
    fn supported_extensions() -> &'static [&'static str];
}

/// 根据文件扩展名选择合适的加载器
///
/// # 参数
///
/// - `path`: 文件路径
///
/// # 返回
///
/// - `Ok(MeshData)`: 成功加载
/// - `Err(DistRenderError)`: 不支持的格式或加载失败
///
/// # 示例
///
/// ```rust,no_run
/// use distrender::geometry::loaders::load_mesh;
/// use std::path::Path;
///
/// let mesh = load_mesh(Path::new("model.obj"))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load_mesh(path: &Path) -> Result<MeshData> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| {
            crate::core::error::DistRenderError::MeshLoading(
                crate::core::error::MeshLoadError::UnsupportedFormat(
                    "无法确定文件扩展名".to_string(),
                ),
            )
        })?;

    match extension.as_str() {
        "obj" => ObjLoader::load_from_file(path),
        "fbx" => FbxLoader::load_from_file(path),
        _ => Err(crate::core::error::DistRenderError::MeshLoading(
            crate::core::error::MeshLoadError::UnsupportedFormat(format!(
                "不支持的文件格式: .{}",
                extension
            )),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let obj_exts = ObjLoader::supported_extensions();
        assert!(obj_exts.contains(&"obj"));

        let fbx_exts = FbxLoader::supported_extensions();
        assert!(fbx_exts.contains(&"fbx"));
    }
}
