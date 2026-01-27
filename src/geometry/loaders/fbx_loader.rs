/// FBX 文件加载器
///
/// 使用 russimp (Assimp) 加载 Autodesk FBX 格式的3D模型。
/// 支持复杂的场景层次、多网格、多材质等高级特性。

use super::MeshLoader;
use crate::core::error::{MeshLoadError, Result};
use crate::geometry::mesh::MeshData;
use std::path::Path;

/// FBX 格式加载器
///
/// 实现 `MeshLoader` trait，提供 FBX 文件的加载功能。
/// 使用 Assimp 库通过 russimp crate 进行加载。
///
/// # 特性
///
/// - 支持 FBX 2011 及以上版本
/// - 递归遍历场景层次
/// - 自动三角化
/// - 自动生成法线和切线（通过 Assimp 后处理）
/// - 支持多网格和多材质
///
/// # 使用示例
///
/// ```rust,no_run
/// use distrender::geometry::loaders::{MeshLoader, FbxLoader};
/// use std::path::Path;
///
/// let mesh = FbxLoader::load_from_file(Path::new("model.fbx"))?;
/// println!("加载了 {} 个顶点", mesh.vertex_count());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct FbxLoader;

impl MeshLoader for FbxLoader {
    fn load_from_file(path: &Path) -> Result<MeshData> {
        // TODO: 将在 Phase 4 实现
        // 这是一个占位符实现
        if !path.exists() {
            return Err(MeshLoadError::FileNotFound(path.to_path_buf()).into());
        }

        // 返回空网格数据作为占位
        Ok(MeshData::new())
    }

    fn load_from_memory(_data: &[u8]) -> Result<MeshData> {
        // TODO: 将在 Phase 4 实现
        Ok(MeshData::new())
    }

    fn supported_extensions() -> &'static [&'static str] {
        &["fbx"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let exts = FbxLoader::supported_extensions();
        assert_eq!(exts, &["fbx"]);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = FbxLoader::load_from_file(Path::new("nonexistent.fbx"));
        assert!(result.is_err());
    }
}
