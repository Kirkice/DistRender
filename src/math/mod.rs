//! 统一的数学库模块
//!
//! 提供游戏和图形编程常用的数学类型和函数。
//! 基于 `nalgebra` 但提供了更友好的 API。
//!
//! # 模块组织
//!
//! - **基础类型**：Vector2/3/4, Matrix3/4, Quaternion, Color
//! - **常量**：PI, TAU, DEG_TO_RAD 等
//! - **工具函数**：clamp, lerp, smoothstep 等
//! - **矩阵辅助函数**：translation, rotation, projection 等
//! - **四元数辅助函数**：from_euler_angles, slerp 等
//! - **颜色空间转换**：linear_to_srgb, srgb_to_linear 等
//! - **几何处理**：法线重建、切线空间计算（见 geometry 子模块）
//!
//! # 设计理念
//!
//! 参考 C++ DistEngine 的数学库设计：
//! - 简洁的类型名称（Vector3, Matrix4 等）
//! - 常用的静态方法（Dot, Cross, Normalize 等）
//! - 与 DirectXMath 类似的 API 风格
//! - 零成本抽象，性能与手写代码相当

// 允许未使用的代码，因为这是一个工具库，不是所有函数都会立即使用
#![allow(dead_code)]

pub use nalgebra::{
    Matrix3 as Mat3, Matrix4 as Mat4, Point3, UnitQuaternion,
    Vector2 as Vec2, Vector3 as Vec3, Vector4 as Vec4,
};

// 类型别名，使用更简洁的名称
pub type Vector2 = Vec2<f32>;
pub type Vector3 = Vec3<f32>;
pub type Vector4 = Vec4<f32>;
pub type Matrix3 = Mat3<f32>;
pub type Matrix4 = Mat4<f32>;
pub type Quaternion = UnitQuaternion<f32>;

/// 颜色类型（RGBA，范围 0.0-1.0）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// 创建新的颜色
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// 创建 RGB 颜色（alpha = 1.0）
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// 从整数值创建颜色（0-255）
    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// 转换为 Vector4
    pub fn to_vec4(&self) -> Vector4 {
        Vector4::new(self.r, self.g, self.b, self.a)
    }

    /// 转换为 Vector3（忽略 alpha）
    pub fn to_vec3(&self) -> Vector3 {
        Vector3::new(self.r, self.g, self.b)
    }

    // 预定义颜色
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const YELLOW: Color = Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
}

/// 数学常量
pub mod constants {
    /// π
    pub const PI: f32 = std::f32::consts::PI;

    /// 2π
    pub const TAU: f32 = std::f32::consts::TAU;

    /// π/2
    pub const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

    /// π/4
    pub const QUARTER_PI: f32 = std::f32::consts::FRAC_PI_4;

    /// 角度转弧度的系数
    pub const DEG_TO_RAD: f32 = PI / 180.0;

    /// 弧度转角度的系数
    pub const RAD_TO_DEG: f32 = 180.0 / PI;

    /// 浮点数比较的 epsilon
    pub const EPSILON: f32 = 1e-6;
}

/// 数学工具函数
pub mod utils {
    use super::*;

    /// 限制值在范围内
    pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// 饱和函数（限制在 0.0-1.0）
    pub fn saturate(value: f32) -> f32 {
        clamp(value, 0.0, 1.0)
    }

    /// 线性插值
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// 向量线性插值
    pub fn lerp_vec3(a: &Vector3, b: &Vector3, t: f32) -> Vector3 {
        a + (b - a) * t
    }

    /// Smoothstep 插值
    pub fn smoothstep(a: f32, b: f32, t: f32) -> f32 {
        let t = saturate((t - a) / (b - a));
        t * t * (3.0 - 2.0 * t)
    }

    /// 角度转弧度
    pub fn deg_to_rad(degrees: f32) -> f32 {
        degrees * constants::DEG_TO_RAD
    }

    /// 弧度转角度
    pub fn rad_to_deg(radians: f32) -> f32 {
        radians * constants::RAD_TO_DEG
    }

    /// 检查两个浮点数是否近似相等
    pub fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }
}

/// 向量扩展 trait
///
/// 为 nalgebra 的向量类型添加额外的便捷方法
pub trait Vector3Ext {
    /// 计算向量长度
    fn length(&self) -> f32;

    /// 计算向量长度的平方
    fn length_squared(&self) -> f32;

    /// 归一化向量
    fn normalized(&self) -> Vector3;

    /// 点积
    fn dot_product(&self, other: &Vector3) -> f32;

    /// 叉积
    fn cross_product(&self, other: &Vector3) -> Vector3;

    /// 计算到另一个向量的距离
    fn distance_to(&self, other: &Vector3) -> f32;

    /// 向另一个向量方向插值
    fn lerp_to(&self, other: &Vector3, t: f32) -> Vector3;
}

impl Vector3Ext for Vector3 {
    fn length(&self) -> f32 {
        self.norm()
    }

    fn length_squared(&self) -> f32 {
        self.norm_squared()
    }

    fn normalized(&self) -> Vector3 {
        self.normalize()
    }

    fn dot_product(&self, other: &Vector3) -> f32 {
        self.dot(other)
    }

    fn cross_product(&self, other: &Vector3) -> Vector3 {
        self.cross(other)
    }

    fn distance_to(&self, other: &Vector3) -> f32 {
        (self - other).norm()
    }

    fn lerp_to(&self, other: &Vector3, t: f32) -> Vector3 {
        utils::lerp_vec3(self, other, t)
    }
}

/// 矩阵辅助函数
pub mod matrix {
    use super::*;

    /// 创建平移矩阵
    pub fn translation(x: f32, y: f32, z: f32) -> Matrix4 {
        Matrix4::new_translation(&Vector3::new(x, y, z))
    }

    /// 创建缩放矩阵
    pub fn scaling(x: f32, y: f32, z: f32) -> Matrix4 {
        Matrix4::new_nonuniform_scaling(&Vector3::new(x, y, z))
    }

    /// 创建绕 X 轴旋转的矩阵
    pub fn rotation_x(angle: f32) -> Matrix4 {
        Matrix4::from_axis_angle(&Vector3::x_axis(), angle)
    }

    /// 创建绕 Y 轴旋转的矩阵
    pub fn rotation_y(angle: f32) -> Matrix4 {
        Matrix4::from_axis_angle(&Vector3::y_axis(), angle)
    }

    /// 创建绕 Z 轴旋转的矩阵
    pub fn rotation_z(angle: f32) -> Matrix4 {
        Matrix4::from_axis_angle(&Vector3::z_axis(), angle)
    }

    /// 从四元数创建旋转矩阵
    pub fn from_quaternion(quat: &Quaternion) -> Matrix4 {
        quat.to_homogeneous()
    }

    /// 创建透视投影矩阵
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Matrix4 {
        Matrix4::new_perspective(aspect, fov_y, near, far)
    }

    /// 创建正交投影矩阵
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Matrix4 {
        Matrix4::new_orthographic(left, right, bottom, top, near, far)
    }

    /// 创建 Look-At 视图矩阵
    pub fn look_at(eye: &Vector3, target: &Vector3, up: &Vector3) -> Matrix4 {
        Matrix4::look_at_rh(&Point3::from(*eye), &Point3::from(*target), up)
    }
}

/// 四元数辅助函数
pub mod quaternion {
    use super::*;

    /// 从欧拉角创建四元数（YXZ 顺序）
    pub fn from_euler_angles(yaw: f32, pitch: f32, roll: f32) -> Quaternion {
        UnitQuaternion::from_euler_angles(roll, pitch, yaw)
    }

    /// 从轴角创建四元数
    pub fn from_axis_angle(axis: &Vector3, angle: f32) -> Quaternion {
        UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(*axis), angle)
    }

    /// 在两个四元数之间球面线性插值
    pub fn slerp(q1: &Quaternion, q2: &Quaternion, t: f32) -> Quaternion {
        q1.slerp(q2, t)
    }
}

/// 颜色空间转换
pub mod color_space {
    use super::*;

    /// Linear 转 sRGB
    pub fn linear_to_srgb(color: Vector3) -> Vector3 {
        let linear_to_srgb_component = |c: f32| -> f32 {
            if c <= 0.0031308 {
                c * 12.92
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        };

        Vector3::new(
            linear_to_srgb_component(color.x),
            linear_to_srgb_component(color.y),
            linear_to_srgb_component(color.z),
        )
    }

    /// sRGB 转 Linear
    pub fn srgb_to_linear(color: Vector3) -> Vector3 {
        let srgb_to_linear_component = |c: f32| -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        Vector3::new(
            srgb_to_linear_component(color.x),
            srgb_to_linear_component(color.y),
            srgb_to_linear_component(color.z),
        )
    }

    /// 计算亮度
    pub fn luminance(color: &Vector3) -> f32 {
        color.dot(&Vector3::new(0.299, 0.587, 0.114))
    }
}

// 几何处理模块（网格法线、切线等）
pub mod geometry;

// 注意：由于 Rust 的孤儿规则，我们不能为 nalgebra 的 Vector 类型实现 bytemuck traits
// 顶点结构使用原始数组，但提供了 from_vectors() 便利方法来使用 Vector 类型

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_operations() {
        let v1 = Vector3::new(1.0, 0.0, 0.0);
        let v2 = Vector3::new(0.0, 1.0, 0.0);

        let cross = v1.cross_product(&v2);
        assert!((cross.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_color_creation() {
        let color = Color::rgb(1.0, 0.5, 0.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_matrix_translation() {
        let mat = matrix::translation(1.0, 2.0, 3.0);
        let point = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let result = mat * point;

        assert!((result.x - 1.0).abs() < 1e-6);
        assert!((result.y - 2.0).abs() < 1e-6);
        assert!((result.z - 3.0).abs() < 1e-6);
    }
}
