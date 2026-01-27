//! 场景配置模块
//!
//! 定义场景配置，包括相机、模型等元素的变换和参数。

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use crate::core::error::{Result, DistRenderError, ConfigError};
use crate::core::math::{Vector3, Matrix4};

/// 3D 变换数据
///
/// 包含位置、旋转和缩放信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    /// 位置 (x, y, z)
    #[serde(default = "default_position")]
    pub position: [f32; 3],

    /// 旋转（欧拉角，度数）(pitch, yaw, roll)
    #[serde(default = "default_rotation")]
    pub rotation: [f32; 3],

    /// 缩放 (x, y, z)
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
}

fn default_position() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}

fn default_rotation() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform {
    /// 创建模型矩阵
    ///
    /// 将 Transform 转换为 4x4 模型矩阵。
    /// 变换顺序：缩放 -> 旋转 -> 平移
    pub fn to_matrix(&self) -> Matrix4 {
        use std::f32::consts::PI;

        // 转换角度到弧度
        let pitch = self.rotation[0] * PI / 180.0;
        let yaw = self.rotation[1] * PI / 180.0;
        let roll = self.rotation[2] * PI / 180.0;

        // 平移矩阵
        let translation = Matrix4::new_translation(&Vector3::new(
            self.position[0],
            self.position[1],
            self.position[2],
        ));

        // 旋转矩阵（欧拉角）
        let rotation_x = Matrix4::from_axis_angle(&Vector3::x_axis(), pitch);
        let rotation_y = Matrix4::from_axis_angle(&Vector3::y_axis(), yaw);
        let rotation_z = Matrix4::from_axis_angle(&Vector3::z_axis(), roll);
        let rotation = rotation_z * rotation_y * rotation_x;

        // 缩放矩阵
        let scale = Matrix4::new_nonuniform_scaling(&Vector3::new(
            self.scale[0],
            self.scale[1],
            self.scale[2],
        ));

        // 组合：T * R * S
        translation * rotation * scale
    }
}

/// 相机配置
///
/// 定义相机的位置、朝向和投影参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// 相机变换
    pub transform: Transform,

    /// 视野角度（Field of View，度数）
    #[serde(default = "default_fov")]
    pub fov: f32,

    /// 近裁剪面距离
    #[serde(default = "default_near_clip")]
    pub near_clip: f32,

    /// 远裁剪面距离
    #[serde(default = "default_far_clip")]
    pub far_clip: f32,
}

fn default_fov() -> f32 {
    60.0
}

fn default_near_clip() -> f32 {
    0.1
}

fn default_far_clip() -> f32 {
    100.0
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            transform: Transform {
                position: [0.0, 0.0, 3.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            fov: 60.0,
            near_clip: 0.1,
            far_clip: 100.0,
        }
    }
}

impl CameraConfig {
    /// 创建视图矩阵
    ///
    /// 从相机的位置和旋转创建视图矩阵。
    pub fn view_matrix(&self) -> Matrix4 {
        use std::f32::consts::PI;

        // 相机位置
        let eye = Vector3::new(
            self.transform.position[0],
            self.transform.position[1],
            self.transform.position[2],
        );

        // 计算相机朝向（基于旋转）
        let pitch = self.transform.rotation[0] * PI / 180.0;
        let yaw = self.transform.rotation[1] * PI / 180.0;

        // 前向向量（默认朝向 -Z 方向）
        let forward = Vector3::new(
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),  // 负号使默认朝向 -Z
        );

        let target = eye + forward;
        let up = Vector3::new(0.0, 1.0, 0.0);

        Matrix4::look_at_rh(&eye.into(), &target.into(), &up)
    }

    /// 创建透视投影矩阵
    ///
    /// 使用 FOV、宽高比和裁剪面创建投影矩阵。
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Matrix4 {
        use std::f32::consts::PI;
        let fov_rad = self.fov * PI / 180.0;
        Matrix4::new_perspective(aspect_ratio, fov_rad, self.near_clip, self.far_clip)
    }
}

/// 模型配置
///
/// 定义模型的文件路径和变换。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型文件路径
    pub path: String,

    /// 模型变换
    #[serde(default)]
    pub transform: Transform,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            path: "assets/models/sphere.obj".to_string(),
            transform: Transform::default(),
        }
    }
}

/// 场景配置
///
/// 包含场景中的所有元素配置，包括相机和模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneConfig {
    /// 相机配置
    #[serde(default)]
    pub camera: CameraConfig,

    /// 模型配置
    #[serde(default)]
    pub model: ModelConfig,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            camera: CameraConfig::default(),
            model: ModelConfig::default(),
        }
    }
}

impl SceneConfig {
    /// 从文件加载场景配置
    ///
    /// # 参数
    ///
    /// - `path`: 配置文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(SceneConfig)`: 加载成功
    /// - `Err(DistRenderError)`: 加载失败
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .map_err(|e| DistRenderError::Config(ConfigError::FileNotFound(format!(
                "Failed to read scene config file '{}': {}",
                path.display(),
                e
            ))))?;

        toml::from_str(&contents)
            .map_err(|e| DistRenderError::Config(ConfigError::ParseError(format!(
                "Failed to parse scene config: {}",
                e
            ))))
    }

    /// 从文件加载，如果文件不存在则返回默认配置
    ///
    /// # 参数
    ///
    /// - `path`: 配置文件路径
    ///
    /// # 返回
    ///
    /// 场景配置（从文件加载或默认配置）
    pub fn from_file_or_default<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            match Self::from_file(path) {
                Ok(config) => {
                    tracing::info!("Loaded scene config from: {}", path.display());
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to load scene config: {}, using defaults", e);
                    Self::default()
                }
            }
        } else {
            tracing::info!("Scene config not found, using defaults");
            Self::default()
        }
    }

    /// 保存配置到文件
    ///
    /// # 参数
    ///
    /// - `path`: 配置文件路径
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 保存成功
    /// - `Err(DistRenderError)`: 保存失败
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let contents = toml::to_string_pretty(self)
            .map_err(|e| DistRenderError::Config(ConfigError::ParseError(format!(
                "Failed to serialize scene config: {}",
                e
            ))))?;

        fs::write(path, contents)
            .map_err(|e| DistRenderError::Config(ConfigError::FileNotFound(format!(
                "Failed to write scene config to '{}': {}",
                path.display(),
                e
            ))))?;

        tracing::info!("Saved scene config to: {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_transform() {
        let transform = Transform::default();
        assert_eq!(transform.position, [0.0, 0.0, 0.0]);
        assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_transform_to_matrix() {
        let transform = Transform {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        };
        let matrix = transform.to_matrix();

        // 检查平移部分
        assert!((matrix[(0, 3)] - 1.0).abs() < 0.001);
        assert!((matrix[(1, 3)] - 2.0).abs() < 0.001);
        assert!((matrix[(2, 3)] - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_default_camera() {
        let camera = CameraConfig::default();
        assert_eq!(camera.fov, 60.0);
        assert_eq!(camera.near_clip, 0.1);
        assert_eq!(camera.far_clip, 100.0);
    }

    #[test]
    fn test_default_scene() {
        let scene = SceneConfig::default();
        assert_eq!(scene.camera.fov, 60.0);
        assert_eq!(scene.model.path, "assets/models/sphere.obj");
    }
}
