//! Transform 组件
//!
//! 参考 DistEngine 的 Transform 类实现
//! 管理游戏对象的位置、旋转和缩放

use super::Component;
use crate::core::math::{Vector3, Matrix4, Quaternion};

/// Transform 组件
///
/// 管理游戏对象的空间变换（位置、旋转、缩放）
pub struct Transform {
    /// 组件名称
    name: String,

    /// 位置
    pub position: Vector3,

    /// 欧拉角（度数）
    pub euler_angle: Vector3,

    /// 缩放
    pub scale: Vector3,

    /// 前方向量
    pub forward: Vector3,

    /// 四元数（内部使用）
    quaternion: Quaternion,

    /// 世界矩阵缓存
    world_matrix: Matrix4,

    /// 世界矩阵是否需要更新
    world_dirty: bool,
}

impl Transform {
    /// 创建新的 Transform 组件
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            position: Vector3::zeros(),
            euler_angle: Vector3::zeros(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            quaternion: Quaternion::identity(),
            world_matrix: Matrix4::identity(),
            world_dirty: true,
        }
    }

    /// 创建带位置的 Transform
    pub fn with_position(name: impl Into<String>, position: Vector3) -> Self {
        let mut transform = Self::new(name);
        transform.position = position;
        transform
    }

    /// 创建带位置和旋转的 Transform
    pub fn with_position_rotation(
        name: impl Into<String>,
        position: Vector3,
        euler_angle: Vector3,
    ) -> Self {
        let mut transform = Self::new(name);
        transform.position = position;
        transform.euler_angle = euler_angle;
        transform.world_dirty = true;
        transform
    }

    /// 设置位置
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
        self.world_dirty = true;
    }

    /// 设置位置（分量形式）
    pub fn set_position_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.position = Vector3::new(x, y, z);
        self.world_dirty = true;
    }

    /// 设置欧拉角（度数）
    pub fn set_euler_angle(&mut self, euler: Vector3) {
        self.euler_angle = euler;
        self.world_dirty = true;
    }

    /// 设置欧拉角（分量形式，度数）
    pub fn set_euler_angle_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.euler_angle = Vector3::new(x, y, z);
        self.world_dirty = true;
    }

    /// 设置缩放
    pub fn set_scale(&mut self, scale: Vector3) {
        self.scale = scale;
        self.world_dirty = true;
    }

    /// 设置缩放（分量形式）
    pub fn set_scale_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.scale = Vector3::new(x, y, z);
        self.world_dirty = true;
    }

    /// 添加位置偏移
    pub fn add_position(&mut self, offset: Vector3) {
        self.position += offset;
        self.world_dirty = true;
    }

    /// 添加旋转偏移（度数）
    pub fn add_euler_angle(&mut self, offset: Vector3) {
        self.euler_angle += offset;
        self.world_dirty = true;
    }

    /// 添加缩放偏移
    pub fn add_scale(&mut self, offset: Vector3) {
        self.scale += offset;
        self.world_dirty = true;
    }

    /// 获取四元数
    pub fn quaternion(&self) -> Quaternion {
        self.quaternion
    }

    /// 获取世界矩阵
    pub fn world_matrix(&mut self) -> Matrix4 {
        if self.world_dirty {
            self.update_world_matrix();
        }
        self.world_matrix
    }

    /// 更新世界矩阵
    fn update_world_matrix(&mut self) {
        use std::f32::consts::PI;

        // 转换欧拉角为弧度
        let pitch = self.euler_angle.x * PI / 180.0;
        let yaw = self.euler_angle.y * PI / 180.0;
        let roll = self.euler_angle.z * PI / 180.0;

        // 创建变换矩阵
        let translation = Matrix4::new_translation(&self.position);

        // 旋转矩阵（欧拉角）
        let rotation_x = Matrix4::from_axis_angle(&Vector3::x_axis(), pitch);
        let rotation_y = Matrix4::from_axis_angle(&Vector3::y_axis(), yaw);
        let rotation_z = Matrix4::from_axis_angle(&Vector3::z_axis(), roll);
        let rotation = rotation_z * rotation_y * rotation_x;

        // 缩放矩阵
        let scale = Matrix4::new_nonuniform_scaling(&self.scale);

        // 组合：T * R * S
        self.world_matrix = translation * rotation * scale;

        // 更新四元数
        self.quaternion = Quaternion::from_euler_angles(roll, pitch, yaw);

        // 更新前向向量
        self.forward = rotation.transform_vector(&Vector3::new(0.0, 0.0, -1.0)).normalize();

        self.world_dirty = false;
    }
}

impl Component for Transform {
    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, _delta_time: f32) {
        // 可以在这里添加每帧更新逻辑
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new("Transform")
    }
}
