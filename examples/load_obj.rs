/// OBJ 模型加载示例
///
/// 演示如何使用 DistRender 的 geometry 模块加载 OBJ 文件。
///
/// 运行方式：
/// ```
/// cargo run --example load_obj
/// ```

use dist_render::geometry::loaders::{MeshLoader, ObjLoader};
use std::path::Path;

fn main() {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== DistRender OBJ 加载器示例 ===\n");

    // 加载测试三角形
    let obj_path = Path::new("assets/sphere.obj");

    println!("正在加载: {}", obj_path.display());

    match ObjLoader::load_from_file(obj_path) {
        Ok(mesh_data) => {
            println!("\n✓ 加载成功！\n");

            println!("网格信息:");
            println!("  名称: {}", mesh_data.name.as_ref().unwrap_or(&"未命名".to_string()));
            println!("  顶点数: {}", mesh_data.vertex_count());
            println!("  索引数: {}", mesh_data.index_count());
            println!("  三角形数: {}", mesh_data.triangle_count());
            println!("  子网格数: {}", mesh_data.subsets.len());

            // 显示前几个顶点的数据
            println!("\n顶点数据（前 {} 个）:", mesh_data.vertex_count().min(3));
            for (i, vertex) in mesh_data.vertices.iter().take(3).enumerate() {
                println!("  顶点 {}:", i);
                println!("    位置: [{:.3}, {:.3}, {:.3}]",
                    vertex.position[0], vertex.position[1], vertex.position[2]);
                println!("    法线: [{:.3}, {:.3}, {:.3}]",
                    vertex.normal[0], vertex.normal[1], vertex.normal[2]);
                println!("    UV: [{:.3}, {:.3}]",
                    vertex.texcoord[0], vertex.texcoord[1]);
                println!("    切线: [{:.3}, {:.3}, {:.3}]",
                    vertex.tangent[0], vertex.tangent[1], vertex.tangent[2]);
            }

            // 显示索引数据
            println!("\n索引数据（前 {} 个）:", mesh_data.index_count().min(9));
            for (i, &index) in mesh_data.indices.iter().take(9).enumerate() {
                if i % 3 == 0 {
                    print!("  三角形 {}: [", i / 3);
                }
                print!("{}", index);
                if i % 3 == 2 {
                    println!("]");
                } else {
                    print!(", ");
                }
            }

            // 显示子网格信息
            if !mesh_data.subsets.is_empty() {
                println!("\n子网格信息:");
                for (i, subset) in mesh_data.subsets.iter().enumerate() {
                    println!("  子网格 {}:", i);
                    println!("    ID: {}", subset.id);
                    println!("    顶点范围: {} - {}",
                        subset.vertex_start,
                        subset.vertex_start + subset.vertex_count - 1);
                    println!("    面数: {}", subset.face_count);
                }
            }

            // 验证数据
            match mesh_data.validate() {
                Ok(()) => println!("\n✓ 数据验证通过"),
                Err(e) => println!("\n✗ 数据验证失败: {}", e),
            }
        }
        Err(e) => {
            eprintln!("\n✗ 加载失败: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n=== 示例完成 ===");
}
