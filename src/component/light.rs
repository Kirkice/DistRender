//! 光照组件模块
//!
//! 参考 DistEngine 的 Light.h、PointLight.h、SpotLight.h 实现
//! 包含基础光源接口和各种光源类型

use crate::component::Component;
use crate::core::math::Vector3;

/// 光源颜色（RGB）
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    /// 创建新颜色
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// 白色
    pub const fn white() -> Self {
        Self { r: 1.0, g: 1.0, b: 1.0 }
    }

    /// 黑色
    pub const fn black() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0 }
    }

    /// 红色
    pub const fn red() -> Self {
        Self { r: 1.0, g: 0.0, b: 0.0 }
    }

    /// 绿色
    pub const fn green() -> Self {
        Self { r: 0.0, g: 1.0, b: 0.0 }
    }

    /// 蓝色
    pub const fn blue() -> Self {
        Self { r: 0.0, g: 0.0, b: 1.0 }
    }

    /// 转换为数组
    pub fn to_array(&self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    /// 带强度的颜色（颜色 * 强度）
    pub fn with_intensity(&self, intensity: f32) -> [f32; 3] {
        [self.r * intensity, self.g * intensity, self.b * intensity]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::white()
    }
}

/// 光源类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// 方向光
    Directional,
    /// 点光源
    Point,
    /// 聚光灯
    Spot,
}

/// 光源基础 trait
///
/// 所有光源类型的通用接口
pub trait Light: Component {
    /// 获取光源类型
    fn light_type(&self) -> LightType;

    /// 获取光照强度
    fn intensity(&self) -> f32;

    /// 设置光照强度
    fn set_intensity(&mut self, intensity: f32);

    /// 获取光照颜色
    fn color(&self) -> &Color;

    /// 设置光照颜色
    fn set_color(&mut self, color: Color);

    /// 获取光源位置（如果适用）
    fn position(&self) -> Option<Vector3> {
        None
    }

    /// 获取光源方向（如果适用）
    fn direction(&self) -> Option<Vector3> {
        None
    }

    /// 获取光源范围（如果适用）
    fn range(&self) -> Option<f32> {
        None
    }
}

/// 方向光（平行光）
///
/// 模拟太阳光等远距离光源，所有光线平行
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    name: String,
    /// 光照强度
    pub intensity: f32,
    /// 光照颜色
    pub color: Color,
    /// 光照方向（归一化向量）
    pub direction: Vector3,
}

impl DirectionalLight {
    /// 创建新的方向光
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color: Color::white(),
            direction: Vector3::new(0.0, -1.0, 0.0), // 默认向下
        }
    }

    /// 创建带颜色的方向光
    pub fn with_color(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color,
            direction: Vector3::new(0.0, -1.0, 0.0),
        }
    }

    /// 创建完全自定义的方向光
    pub fn with_params(
        name: impl Into<String>,
        color: Color,
        intensity: f32,
        direction: Vector3,
    ) -> Self {
        Self {
            name: name.into(),
            intensity,
            color,
            direction: direction.normalize(),
        }
    }

    /// 设置光照方向
    pub fn set_direction(&mut self, direction: Vector3) {
        self.direction = direction.normalize();
    }
}

impl Component for DirectionalLight {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Light for DirectionalLight {
    fn light_type(&self) -> LightType {
        LightType::Directional
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    fn color(&self) -> &Color {
        &self.color
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn direction(&self) -> Option<Vector3> {
        Some(self.direction)
    }
}

/// 点光源
///
/// 从一个点向所有方向发射光线，光照强度随距离衰减
#[derive(Debug, Clone)]
pub struct PointLight {
    name: String,
    /// 光照强度
    pub intensity: f32,
    /// 光照颜色
    pub color: Color,
    /// 光源位置
    pub position: Vector3,
    /// 光照范围（影响距离）
    pub range: f32,
}

impl PointLight {
    /// 创建新的点光源
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color: Color::white(),
            position: Vector3::zeros(),
            range: 10.0,
        }
    }

    /// 创建带颜色的点光源
    pub fn with_color(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color,
            position: Vector3::zeros(),
            range: 10.0,
        }
    }

    /// 创建带颜色和强度的点光源
    pub fn with_color_intensity(
        name: impl Into<String>,
        color: Color,
        intensity: f32,
    ) -> Self {
        Self {
            name: name.into(),
            intensity,
            color,
            position: Vector3::zeros(),
            range: 10.0,
        }
    }

    /// 创建完全自定义的点光源
    pub fn with_params(
        name: impl Into<String>,
        color: Color,
        intensity: f32,
        range: f32,
    ) -> Self {
        Self {
            name: name.into(),
            intensity,
            color,
            position: Vector3::zeros(),
            range,
        }
    }

    /// 设置光源位置
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// 设置光照范围
    pub fn set_range(&mut self, range: f32) {
        self.range = range.max(0.0);
    }
}

impl Component for PointLight {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Light for PointLight {
    fn light_type(&self) -> LightType {
        LightType::Point
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    fn color(&self) -> &Color {
        &self.color
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn position(&self) -> Option<Vector3> {
        Some(self.position)
    }

    fn range(&self) -> Option<f32> {
        Some(self.range)
    }
}

/// 聚光灯
///
/// 从一个点沿特定方向发射锥形光线
#[derive(Debug, Clone)]
pub struct SpotLight {
    name: String,
    /// 光照强度
    pub intensity: f32,
    /// 光照颜色
    pub color: Color,
    /// 光源位置
    pub position: Vector3,
    /// 光照方向（归一化向量）
    pub direction: Vector3,
    /// 光照范围（影响距离）
    pub range: f32,
    /// 聚光角度（弧度，表示锥形的半角）
    pub spot_angle: f32,
}

impl SpotLight {
    /// 创建新的聚光灯
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color: Color::white(),
            position: Vector3::zeros(),
            direction: Vector3::new(0.0, -1.0, 0.0),
            range: 10.0,
            spot_angle: 45.0_f32.to_radians(), // 默认 45 度
        }
    }

    /// 创建带颜色的聚光灯
    pub fn with_color(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color,
            position: Vector3::zeros(),
            direction: Vector3::new(0.0, -1.0, 0.0),
            range: 10.0,
            spot_angle: 45.0_f32.to_radians(),
        }
    }

    /// 创建带颜色和范围的聚光灯
    pub fn with_color_range(
        name: impl Into<String>,
        color: Color,
        range: f32,
    ) -> Self {
        Self {
            name: name.into(),
            intensity: 1.0,
            color,
            position: Vector3::zeros(),
            direction: Vector3::new(0.0, -1.0, 0.0),
            range,
            spot_angle: 45.0_f32.to_radians(),
        }
    }

    /// 创建带颜色、范围和强度的聚光灯
    pub fn with_color_range_intensity(
        name: impl Into<String>,
        color: Color,
        range: f32,
        intensity: f32,
    ) -> Self {
        Self {
            name: name.into(),
            intensity,
            color,
            position: Vector3::zeros(),
            direction: Vector3::new(0.0, -1.0, 0.0),
            range,
            spot_angle: 45.0_f32.to_radians(),
        }
    }

    /// 创建完全自定义的聚光灯
    pub fn with_params(
        name: impl Into<String>,
        color: Color,
        range: f32,
        intensity: f32,
        spot_angle: f32,
    ) -> Self {
        Self {
            name: name.into(),
            intensity,
            color,
            position: Vector3::zeros(),
            direction: Vector3::new(0.0, -1.0, 0.0),
            range,
            spot_angle: spot_angle.to_radians(),
        }
    }

    /// 设置光源位置
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// 设置光照方向
    pub fn set_direction(&mut self, direction: Vector3) {
        self.direction = direction.normalize();
    }

    /// 设置光照范围
    pub fn set_range(&mut self, range: f32) {
        self.range = range.max(0.0);
    }

    /// 设置聚光角度（度数）
    pub fn set_spot_angle_degrees(&mut self, degrees: f32) {
        self.spot_angle = degrees.to_radians();
    }

    /// 设置聚光角度（弧度）
    pub fn set_spot_angle_radians(&mut self, radians: f32) {
        self.spot_angle = radians;
    }

    /// 获取聚光角度（度数）
    pub fn spot_angle_degrees(&self) -> f32 {
        self.spot_angle.to_degrees()
    }

    /// 获取聚光角度（弧度）
    pub fn spot_angle_radians(&self) -> f32 {
        self.spot_angle
    }
}

impl Component for SpotLight {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Light for SpotLight {
    fn light_type(&self) -> LightType {
        LightType::Spot
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    fn color(&self) -> &Color {
        &self.color
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn position(&self) -> Option<Vector3> {
        Some(self.position)
    }

    fn direction(&self) -> Option<Vector3> {
        Some(self.direction)
    }

    fn range(&self) -> Option<f32> {
        Some(self.range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let white = Color::white();
        assert_eq!(white.r, 1.0);
        assert_eq!(white.g, 1.0);
        assert_eq!(white.b, 1.0);

        let custom = Color::new(0.5, 0.7, 0.9);
        assert_eq!(custom.r, 0.5);
        assert_eq!(custom.g, 0.7);
        assert_eq!(custom.b, 0.9);
    }

    #[test]
    fn test_directional_light() {
        let mut light = DirectionalLight::new("MainLight");
        assert_eq!(light.name(), "MainLight");
        assert_eq!(light.light_type(), LightType::Directional);
        assert_eq!(light.intensity(), 1.0);

        light.set_intensity(2.0);
        assert_eq!(light.intensity(), 2.0);

        light.set_direction(Vector3::new(1.0, -1.0, 0.0));
        assert!(light.direction.norm() > 0.0);
    }

    #[test]
    fn test_point_light() {
        let mut light = PointLight::new("PointLight1");
        assert_eq!(light.light_type(), LightType::Point);
        assert_eq!(light.range, 10.0);

        light.set_range(20.0);
        assert_eq!(light.range, 20.0);

        light.set_position(Vector3::new(5.0, 10.0, 5.0));
        assert_eq!(light.position().unwrap(), Vector3::new(5.0, 10.0, 5.0));
    }

    #[test]
    fn test_spot_light() {
        let mut light = SpotLight::new("SpotLight1");
        assert_eq!(light.light_type(), LightType::Spot);

        light.set_spot_angle_degrees(30.0);
        assert!((light.spot_angle_degrees() - 30.0).abs() < 0.001);

        light.set_direction(Vector3::new(0.0, 0.0, 1.0));
        assert!(light.direction.norm() > 0.0);
    }
}
