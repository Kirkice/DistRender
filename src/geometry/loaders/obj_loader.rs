/// OBJ 文件加载器
///
/// 使用 tobj crate 加载 Wavefront OBJ 格式的3D模型。
/// 支持顶点位置、法线、纹理坐标的加载，并可自动重建缺失的法线和切线。
use super::MeshLoader;
use crate::core::error::{MeshLoadError, Result};
use crate::geometry::mesh::{MeshData, Subset};
use crate::geometry::vertex::Vertex;
use crate::geometry::math_utils::{reconstruct_normals, compute_tangent_space, smooth_normals_by_position};
use std::path::Path;

/// OBJ 格式加载器
///
/// 实现 `MeshLoader` trait，提供 OBJ 文件的加载功能。
///
/// # 特性
///
/// - 使用 tobj crate 解析 OBJ 文件
/// - 自动三角化（如果需要）
/// - UV 坐标翻转（V轴：1.0 - v）
/// - 自动重建缺失的法线
/// - 自动计算切线空间
///
/// # 使用示例
///
/// ```rust,no_run
/// use distrender::geometry::loaders::{MeshLoader, ObjLoader};
/// use std::path::Path;
///
/// let mesh = ObjLoader::load_from_file(Path::new("model.obj"))?;
/// println!("加载了 {} 个顶点", mesh.vertex_count());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ObjLoader;

impl MeshLoader for ObjLoader {
    fn load_from_file(path: &Path) -> Result<MeshData> {
        // 检查文件是否存在
        if !path.exists() {
            return Err(MeshLoadError::FileNotFound(path.to_path_buf()).into());
        }

        // 使用 tobj 加载 OBJ 文件
        let load_options = tobj::LoadOptions {
            triangulate: true,    // 自动三角化
            single_index: true,   // 使用单一索引（简化处理）
            ..Default::default()
        };

        let (models, _materials) = tobj::load_obj(path, &load_options)
            .map_err(|e| MeshLoadError::ParseError(format!("tobj 解析失败: {}", e)))?;

        // 检查是否有模型数据
        if models.is_empty() {
            return Err(MeshLoadError::ValidationError("OBJ 文件不包含任何模型".to_string()).into());
        }

        // 创建 MeshData
        let mut mesh_data = MeshData::with_name(
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unnamed")
        );

        let mut has_normals = false;
        let mut has_texcoords = false;

        // 遍历所有模型（OBJ 可能包含多个对象）
        for (mesh_idx, model) in models.iter().enumerate() {
            let mesh = &model.mesh;

            let vertex_start = mesh_data.vertices.len() as u32;
            let face_start = mesh_data.triangle_count() as u32;

            // 检查数据完整性
            let positions = &mesh.positions;
            let normals = &mesh.normals;
            let texcoords = &mesh.texcoords;

            if positions.len() % 3 != 0 {
                return Err(MeshLoadError::InvalidGeometry(
                    format!("顶点位置数据不完整: {} 个浮点数", positions.len())
                ).into());
            }

            let vertex_count = positions.len() / 3;

            // 更新标志
            if !normals.is_empty() {
                has_normals = true;
            }
            if !texcoords.is_empty() {
                has_texcoords = true;
            }

            // 提取顶点数据
            for i in 0..vertex_count {
                let position = [
                    positions[i * 3],
                    positions[i * 3 + 1],
                    positions[i * 3 + 2],
                ];

                // 提取法线（如果有）
                let normal = if !normals.is_empty() && normals.len() >= (i + 1) * 3 {
                    [
                        normals[i * 3],
                        normals[i * 3 + 1],
                        normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 0.0, 0.0]
                };

                // 提取UV坐标（如果有），并翻转V轴
                let texcoord = if !texcoords.is_empty() && texcoords.len() >= (i + 1) * 2 {
                    [
                        texcoords[i * 2],
                        1.0 - texcoords[i * 2 + 1],  // 翻转V坐标
                    ]
                } else {
                    [0.0, 0.0]
                };

                // 切线将在后处理中计算
                let tangent = [0.0, 0.0, 0.0];

                mesh_data.vertices.push(Vertex {
                    position,
                    normal,
                    texcoord,
                    tangent,
                });
            }

            // 提取索引
            let face_count = mesh.indices.len() / 3;
            for &index in &mesh.indices {
                mesh_data.indices.push(vertex_start + index);
            }

            // 创建子网格
            let subset = Subset::new(
                mesh_idx as u32,
                vertex_start,
                vertex_count as u32,
                face_start,
                face_count as u32,
            );
            mesh_data.subsets.push(subset);
        }

        // 后处理：重建法线（如果缺失）
        if !has_normals {
            tracing::info!("OBJ 文件缺少法线数据，正在重建...");
            reconstruct_normals(&mut mesh_data.vertices, &mesh_data.indices);
        }

        // 后处理：seam 平滑处理（按 position 聚类法线）
        // OBJ 常在 UV seam 处拆顶点，导致同位置不同法线，从而出现“切边”。
        // 这里用一个小的 epsilon 将同一位置附近的顶点归为一组并平均法线。
        smooth_normals_by_position(&mut mesh_data.vertices, 1e-5);

        // 后处理：计算切线空间（如果有UV坐标）
        if has_texcoords {
            tracing::info!("计算切线空间...");
            compute_tangent_space(&mut mesh_data.vertices, &mesh_data.indices);
        } else {
            tracing::warn!("OBJ 文件缺少UV坐标，跳过切线空间计算");
        }

        // 验证数据
        mesh_data.validate()
            .map_err(|e| MeshLoadError::ValidationError(e))?;

        tracing::info!(
            "成功加载 OBJ 文件: {} 个顶点, {} 个三角形, {} 个子网格",
            mesh_data.vertex_count(),
            mesh_data.triangle_count(),
            mesh_data.subsets.len()
        );

        Ok(mesh_data)
    }

    fn load_from_memory(_data: &[u8]) -> Result<MeshData> {
        // tobj 需要从文件系统读取，暂不支持内存加载
        // 可以考虑使用临时文件实现
        Err(MeshLoadError::UnsupportedFormat(
            "OBJ 加载器暂不支持从内存加载".to_string()
        ).into())
    }

    fn supported_extensions() -> &'static [&'static str] {
        &["obj"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let exts = ObjLoader::supported_extensions();
        assert_eq!(exts, &["obj"]);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = ObjLoader::load_from_file(Path::new("nonexistent.obj"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_memory_unsupported() {
        let result = ObjLoader::load_from_memory(&[]);
        assert!(result.is_err());
    }
}
