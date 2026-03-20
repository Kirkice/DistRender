use std::path::PathBuf;

use dist_render_simple::{Mat2, Quat, Vec2, Vec3, Vec3Swizzles};

use crate::misc::smoothstep;

use super::sanitize_ascii_label;

fn default_component_enabled() -> bool {
    true
}

fn default_component_emissive_multiplier() -> f32 {
    1.0
}

fn default_camera_primary() -> bool {
    true
}

pub const DEFAULT_CAMERA_VERTICAL_FOV: f32 = 62.0;
pub const DEFAULT_SUN_SIZE_MULTIPLIER: f32 = 1.0;
pub const DEFAULT_LOCAL_LIGHTS_THETA: f32 = 1.0;
pub const DEFAULT_LOCAL_LIGHTS_PHI: f32 = 1.0;
pub const DEFAULT_LOCAL_LIGHTS_COUNT: u32 = 0;
pub const DEFAULT_LOCAL_LIGHTS_DISTANCE: f32 = 1.5;
pub const DEFAULT_LOCAL_LIGHTS_MULTIPLIER: f32 = 10.0;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SunController {
    #[serde(skip)]
    latent: Option<Vec2>,
    towards_sun: Vec3,
}

impl PartialEq for SunController {
    fn eq(&self, other: &Self) -> bool {
        self.towards_sun == other.towards_sun
    }
}

impl Default for SunController {
    fn default() -> Self {
        Self {
            latent: None,
            towards_sun: Vec3::Y,
        }
    }
}

const SUN_CONTROLLER_SQUISH: f32 = 0.2;

impl SunController {
    pub fn towards_sun(&self) -> Vec3 {
        self.towards_sun
    }

    #[allow(dead_code)]
    pub fn set_towards_sun(&mut self, towards_sun: Vec3) {
        self.towards_sun = towards_sun;
        self.latent = None;
    }

    fn calculate_towards_sun(latent: Vec2) -> Vec3 {
        let mut xz = latent;
        let len = xz.length();

        let ysgn = if len > 1.0 {
            xz *= (2.0 - len) / len;
            -1.0
        } else {
            1.0
        };

        let y = SUN_CONTROLLER_SQUISH * ysgn * (1.0 - xz.length_squared());
        Vec3::new(xz.x, y, xz.y).normalize()
    }

    fn calculate_latent(towards_sun: Vec3) -> Vec2 {
        let t = SUN_CONTROLLER_SQUISH;
        let t2 = t * t;

        let y2 = towards_sun.y * towards_sun.y;
        let y4 = y2 * y2;

        let a = -y2 + 2.0 * t2 * (-1.0 + y2) + (y4 - 4.0 * t2 * y2 * (-1.0 + y2)).sqrt();
        let b = 2.0 * t2 * (-1.0 + y2);
        let xz_len = (a / b).sqrt();

        let xz_len = if xz_len.is_finite() { xz_len } else { 0.0 };

        let mut xz = towards_sun.xz() * (xz_len / towards_sun.xz().length().max(1e-10));

        if towards_sun.y < 0.0 {
            xz *= (2.0 - xz_len) / xz_len;
        }

        xz
    }

    pub fn view_space_rotate(&mut self, ref_frame: &Quat, delta_x: f32, delta_y: f32) {
        let mut xz = *self
            .latent
            .get_or_insert_with(|| Self::calculate_latent(self.towards_sun));

        const MOVE_SPEED: f32 = 0.2;

        let xz_norm = xz.normalize_or_zero();

        let rotation_strength = smoothstep(1.2, 1.5, xz.length());
        let delta = *ref_frame * Vec3::new(-delta_x, 0.0, -delta_y);
        let move_align = delta.xz().perp_dot(xz_norm);

        xz += (delta * MOVE_SPEED).xz();

        let rm = Mat2::from_angle(move_align * rotation_strength);
        xz = rm * xz;

        {
            let len = xz.length();
            if len > 2.0 {
                xz *= -(4.0 - len) / len;
            }
        }

        self.latent = Some(xz);
        self.towards_sun = Self::calculate_towards_sun(xz);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum MeshSource {
    File(PathBuf),
    Cache(PathBuf),
}

impl MeshSource {
    pub fn default_game_object_name(&self) -> String {
        let path = match self {
            MeshSource::File(path) | MeshSource::Cache(path) => path,
        };

        let mut name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("GameObject")
            .to_owned();

        if name.eq_ignore_ascii_case("scene") {
            if let Some(parent_name) = path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
            {
                name = parent_name.to_owned();
            }
        }

        let sanitized = sanitize_ascii_label(&name);
        if sanitized.is_empty() {
            "Imported Mesh".to_owned()
        } else {
            sanitized
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MeshRendererComponent {
    pub source: MeshSource,

    #[serde(default = "default_component_enabled")]
    pub enabled: bool,

    #[serde(default = "default_component_emissive_multiplier")]
    pub emissive_multiplier: f32,
}

impl MeshRendererComponent {
    pub fn new(source: MeshSource) -> Self {
        Self {
            source,
            enabled: true,
            emissive_multiplier: 1.0,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CameraComponent {
    #[serde(default = "default_component_enabled")]
    pub enabled: bool,

    #[serde(default = "default_camera_primary")]
    pub primary: bool,

    pub vertical_fov: f32,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            primary: true,
            vertical_fov: DEFAULT_CAMERA_VERTICAL_FOV,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SunComponent {
    #[serde(default = "default_component_enabled")]
    pub enabled: bool,

    pub controller: SunController,
    pub size_multiplier: f32,
}

impl Default for SunComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            controller: SunController::default(),
            size_multiplier: DEFAULT_SUN_SIZE_MULTIPLIER,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LocalLightsComponent {
    #[serde(default = "default_component_enabled")]
    pub enabled: bool,

    pub theta: f32,
    pub phi: f32,
    pub count: u32,
    pub distance: f32,
    pub multiplier: f32,
}

impl Default for LocalLightsComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            theta: DEFAULT_LOCAL_LIGHTS_THETA,
            phi: DEFAULT_LOCAL_LIGHTS_PHI,
            count: DEFAULT_LOCAL_LIGHTS_COUNT,
            distance: DEFAULT_LOCAL_LIGHTS_DISTANCE,
            multiplier: DEFAULT_LOCAL_LIGHTS_MULTIPLIER,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SceneRenderSettings {
    pub emissive_multiplier: f32,
    pub enable_emissive: bool,
}

impl Default for SceneRenderSettings {
    fn default() -> Self {
        Self {
            emissive_multiplier: 1.0,
            enable_emissive: true,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneComponent {
    MeshRenderer(MeshRendererComponent),
    Camera(CameraComponent),
    Sun(SunComponent),
    LocalLights(LocalLightsComponent),
}

impl SceneComponent {
    pub fn kind_name(&self) -> &'static str {
        match self {
            SceneComponent::MeshRenderer(_) => "Mesh Renderer",
            SceneComponent::Camera(_) => "Camera",
            SceneComponent::Sun(_) => "Sun",
            SceneComponent::LocalLights(_) => "Local Lights",
        }
    }

    pub fn as_mesh_renderer(&self) -> Option<&MeshRendererComponent> {
        match self {
            SceneComponent::MeshRenderer(mesh_renderer) => Some(mesh_renderer),
            _ => None,
        }
    }

    pub fn as_camera(&self) -> Option<&CameraComponent> {
        match self {
            SceneComponent::Camera(camera) => Some(camera),
            _ => None,
        }
    }

    pub fn as_camera_mut(&mut self) -> Option<&mut CameraComponent> {
        match self {
            SceneComponent::Camera(camera) => Some(camera),
            _ => None,
        }
    }

    pub fn as_sun(&self) -> Option<&SunComponent> {
        match self {
            SceneComponent::Sun(sun) => Some(sun),
            _ => None,
        }
    }

    pub fn as_sun_mut(&mut self) -> Option<&mut SunComponent> {
        match self {
            SceneComponent::Sun(sun) => Some(sun),
            _ => None,
        }
    }

    pub fn as_local_lights(&self) -> Option<&LocalLightsComponent> {
        match self {
            SceneComponent::LocalLights(local_lights) => Some(local_lights),
            _ => None,
        }
    }

    pub fn as_local_lights_mut(&mut self) -> Option<&mut LocalLightsComponent> {
        match self {
            SceneComponent::LocalLights(local_lights) => Some(local_lights),
            _ => None,
        }
    }
}