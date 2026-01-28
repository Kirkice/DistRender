//! 组件系统模块
//!
//! 参考 DistEngine 的 Component 架构实现的组件系统。
//! 提供 GameObject、Transform、Camera、Light 等游戏对象组件。

mod component;
mod transform;
mod camera;
mod game_object;
mod light;

pub use component::Component;
pub use transform::Transform;
pub use camera::Camera;
pub use game_object::GameObject;
pub use light::{Light, LightType, Color, DirectionalLight, PointLight, SpotLight};
