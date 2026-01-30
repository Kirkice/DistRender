//! Camera 组件
//!
//! 参考 DistEngine 的 Camera 类实现
//! 管理相机的视锥体和视图矩阵

use super::{Component, Transform};
use crate::math::{Vector3, Matrix4};
use std::f32::consts::PI;

/// Camera 组件
///
/// 管理相机的视图和投影，支持移动、旋转等操作
pub struct Camera {
    /// Transform 组件（继承）
    transform: Transform,

    /// 相机坐标系：右向量
    right: Vector3,

    /// 相机坐标系：上向量
    up: Vector3,

    /// 相机坐标系：前向量（Look）
    look: Vector3,

    /// 近裁剪面距离
    near_z: f32,

    /// 远裁剪面距离
    far_z: f32,

    /// 宽高比
    aspect: f32,

    /// 垂直视场角（弧度）
    fov_y: f32,

    /// 近平面高度
    near_window_height: f32,

    /// 远平面高度
    far_window_height: f32,

    /// 视图矩阵
    view_matrix: Matrix4,

    /// 投影矩阵
    proj_matrix: Matrix4,

    /// 视图矩阵是否需要更新
    view_dirty: bool,
}

impl Camera {
    /// 创建新的 Camera
    pub fn new(name: impl Into<String>) -> Self {
        let mut camera = Self {
            transform: Transform::new(name),
            right: Vector3::new(1.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            look: Vector3::new(0.0, 0.0, 1.0),
            near_z: 0.0,
            far_z: 0.0,
            aspect: 0.0,
            fov_y: 0.0,
            near_window_height: 0.0,
            far_window_height: 0.0,
            view_matrix: Matrix4::identity(),
            proj_matrix: Matrix4::identity(),
            view_dirty: true,
        };

        // 默认透视投影设置：FOV=45度，aspect=1.0，near=1.0，far=1000.0
        camera.set_lens(0.25 * PI, 1.0, 1.0, 1000.0);
        camera
    }

    /// 创建主相机
    pub fn main_camera() -> Self {
        Self::new("MainCamera")
    }

    // ========== 位置相关 ==========

    /// 获取相机位置
    pub fn position(&self) -> Vector3 {
        self.transform.position
    }

    /// 设置相机位置
    pub fn set_position(&mut self, position: Vector3) {
        self.transform.set_position(position);
        self.view_dirty = true;
    }

    /// 设置相机位置（分量形式）
    pub fn set_position_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.transform.set_position_xyz(x, y, z);
        self.view_dirty = true;
    }

    // ========== 相机坐标系向量 ==========

    /// 获取右向量
    pub fn right(&self) -> Vector3 {
        self.right
    }

    /// 获取上向量
    pub fn up(&self) -> Vector3 {
        self.up
    }

    /// 获取前向量（Look）
    pub fn look(&self) -> Vector3 {
        self.look
    }

    // ========== 视锥体属性 ==========

    /// 获取近裁剪面距离
    pub fn near_z(&self) -> f32 {
        self.near_z
    }

    /// 获取远裁剪面距离
    pub fn far_z(&self) -> f32 {
        self.far_z
    }

    /// 获取宽高比
    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    /// 获取垂直 FOV（弧度）
    pub fn fov_y(&self) -> f32 {
        self.fov_y
    }

    /// 获取垂直 FOV（度数）
    pub fn fov_y_degrees(&self) -> f32 {
        self.fov_y * 180.0 / PI
    }

    /// 获取水平 FOV（弧度）
    pub fn fov_x(&self) -> f32 {
        let half_width = 0.5 * self.near_window_width();
        2.0 * (half_width / self.near_z).atan()
    }

    /// 获取近平面宽度
    pub fn near_window_width(&self) -> f32 {
        self.aspect * self.near_window_height
    }

    /// 获取近平面高度
    pub fn near_window_height(&self) -> f32 {
        self.near_window_height
    }

    /// 获取远平面宽度
    pub fn far_window_width(&self) -> f32 {
        self.aspect * self.far_window_height
    }

    /// 获取远平面高度
    pub fn far_window_height(&self) -> f32 {
        self.far_window_height
    }

    // ========== 设置透视投影 ==========

    /// 设置透视投影参数
    ///
    /// # 参数
    /// - `fov_y`: 垂直视场角（弧度）
    /// - `aspect`: 宽高比
    /// - `near_z`: 近裁剪面距离
    /// - `far_z`: 远裁剪面距离
    pub fn set_lens(&mut self, fov_y: f32, aspect: f32, near_z: f32, far_z: f32) {
        self.fov_y = fov_y;
        self.aspect = aspect;
        self.near_z = near_z;
        self.far_z = far_z;

        self.near_window_height = 2.0 * self.near_z * (0.5 * self.fov_y).tan();
        self.far_window_height = 2.0 * self.far_z * (0.5 * self.fov_y).tan();

        // 创建透视投影矩阵
        self.proj_matrix = Matrix4::new_perspective(aspect, fov_y, near_z, far_z);
    }

    /// 设置宽高比
    ///
    /// # 参数
    /// - `aspect`: 宽高比
    pub fn set_aspect(&mut self, aspect: f32) {
        if (self.aspect - aspect).abs() > f32::EPSILON {
            self.aspect = aspect;
            // 重新计算投影矩阵
            self.proj_matrix = Matrix4::new_perspective(self.aspect, self.fov_y, self.near_z, self.far_z);
        }
    }

    // ========== LookAt ==========

    /// 设置相机朝向目标点
    ///
    /// # 参数
    /// - `position`: 相机位置
    /// - `target`: 目标位置
    /// - `world_up`: 世界上向量（通常是 (0, 1, 0)）
    pub fn look_at(&mut self, position: Vector3, target: Vector3, world_up: Vector3) {
        // 计算 Look 向量（从位置指向目标）
        let look = (target - position).normalize();

        // 计算 Right 向量（世界上向量 × Look）
        let right = world_up.cross(&look).normalize();

        // 计算 Up 向量（Look × Right）
        let up = look.cross(&right);

        self.transform.set_position(position);
        self.look = look;
        self.right = right;
        self.up = up;

        self.view_dirty = true;
    }

    // ========== 获取矩阵 ==========

    /// 获取视图矩阵
    pub fn view_matrix(&mut self) -> Matrix4 {
        if self.view_dirty {
            self.update_view_matrix();
        }
        self.view_matrix
    }

    /// 获取投影矩阵
    pub fn proj_matrix(&self) -> Matrix4 {
        self.proj_matrix
    }

    // ========== 相机移动 ==========

    /// 左右平移（Strafe）
    ///
    /// # 参数
    /// - `distance`: 移动距离（正值向右，负值向左）
    pub fn strafe(&mut self, distance: f32) {
        self.transform.position += self.right * distance;
        self.view_dirty = true;
    }

    /// 前后移动（Walk）
    ///
    /// # 参数
    /// - `distance`: 移动距离（正值向前，负值向后）
    pub fn walk(&mut self, distance: f32) {
        self.transform.position += self.look * distance;
        self.view_dirty = true;
    }

    // ========== 相机旋转 ==========

    /// 俯仰旋转（Pitch）
    ///
    /// # 参数
    /// - `angle`: 旋转角度（弧度）
    pub fn pitch(&mut self, angle: f32) {
        // 绕 Right 轴旋转 Up 和 Look 向量
        use nalgebra::Unit;
        let axis = Unit::new_normalize(self.right);
        let rotation = Matrix4::from_axis_angle(&axis, angle);

        self.up = rotation.transform_vector(&self.up).normalize();
        self.look = rotation.transform_vector(&self.look).normalize();

        self.view_dirty = true;
    }

    /// 绕 Y 轴旋转（Yaw）
    ///
    /// # 参数
    /// - `angle`: 旋转角度（弧度）
    pub fn rotate_y(&mut self, angle: f32) {
        // 绕世界 Y 轴旋转所有基向量
        let rotation = Matrix4::from_axis_angle(&Vector3::y_axis(), angle);

        self.right = rotation.transform_vector(&self.right).normalize();
        self.up = rotation.transform_vector(&self.up).normalize();
        self.look = rotation.transform_vector(&self.look).normalize();

        self.view_dirty = true;
    }

    // ========== 更新视图矩阵 ==========

    /// 更新视图矩阵
    ///
    /// 在修改相机位置或方向后需要调用此方法来更新视图矩阵
    pub fn update_view_matrix(&mut self) {
        if !self.view_dirty {
            return;
        }

        // 保持相机坐标轴正交归一化
        let look = self.look.normalize();
        let up = look.cross(&self.right).normalize();
        let right = up.cross(&look);

        // 构建视图矩阵
        let position = self.transform.position;

        let x = -position.dot(&right);
        let y = -position.dot(&up);
        let z = -position.dot(&look);

        // 更新坐标轴
        self.right = right;
        self.up = up;
        self.look = look;

        // 手动构建视图矩阵（行主序）
        #[rustfmt::skip]
        let view = Matrix4::new(
            right.x, right.y, right.z, x,
            up.x,    up.y,    up.z,    y,
            look.x,  look.y,  look.z,  z,
            0.0,     0.0,     0.0,     1.0,
        );

        self.view_matrix = view;
        self.view_dirty = false;
    }

    /// 获取 Transform 组件的可变引用
    pub fn transform_mut(&mut self) -> &mut Transform {
        self.view_dirty = true;
        &mut self.transform
    }

    /// 获取 Transform 组件的引用
    pub fn transform(&self) -> &Transform {
        &self.transform
    }
}

impl Component for Camera {
    fn name(&self) -> &str {
        self.transform.name()
    }

    fn tick(&mut self, delta_time: f32) {
        self.transform.tick(delta_time);

        // 如果需要，更新视图矩阵
        if self.view_dirty {
            self.update_view_matrix();
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::main_camera()
    }
}
