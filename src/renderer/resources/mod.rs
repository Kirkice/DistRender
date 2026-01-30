//! 渲染资源管理模块
//!
//! 包含与渲染资源相关的所有类型和功能：
//! - 顶点数据结构
//! - 资源池管理
//! - 描述符分配器

pub mod vertex;
pub mod resource;
pub mod descriptor;

// 重新导出常用类型
pub use vertex::{MyVertex, GeometryVertex};
pub use resource::FrameResourcePool;
pub use descriptor::DescriptorAllocator;
