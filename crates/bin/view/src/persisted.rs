use std::path::PathBuf;

use dist_render_simple::{Affine3A, EulerRot, Mat2, Quat, Vec2, Vec3, Vec3Swizzles};

use crate::{misc::smoothstep, sequence::Sequence};

fn sanitize_ascii_label(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    let mut previous_was_separator = true;

    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch)
        } else if ch.is_ascii_whitespace() || matches!(ch, '-' | '_' | '.' | '(' | ')' | '[' | ']') {
            Some(' ')
        } else {
            None
        };

        match mapped {
            Some(' ') if !previous_was_separator => {
                sanitized.push(' ');
                previous_was_separator = true;
            }
            Some(' ') => {}
            Some(value) => {
                sanitized.push(value);
                previous_was_separator = false;
            }
            None => {}
        }
    }

    sanitized.trim().to_owned()
}

fn default_game_object_name_for_id(id: GameObjectId) -> String {
    format!("GameObject {}", id.0)
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SunState {
    pub controller: SunController,
    pub size_multiplier: f32,
}

impl Default for SunState {
    fn default() -> Self {
        Self {
            controller: SunController::default(),
            size_multiplier: 1.0,
        }
    }
}

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

        // Mathematica goes brrrrrrr
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

        // The controller has a singularity in the second outer ring.
        // This rotation will kick it out.
        let rotation_strength = smoothstep(1.2, 1.5, xz.length());
        let delta = *ref_frame * Vec3::new(-delta_x, 0.0, -delta_y);
        let move_align = delta.xz().perp_dot(xz_norm);

        // Working in projective geometry, add the new input
        xz += (delta * MOVE_SPEED).xz();

        let rm = Mat2::from_angle(move_align * rotation_strength);
        xz = rm * xz;

        {
            let len = xz.length();
            if len > 2.0 {
                // Second outer ring; reflect to the other side.
                xz *= -(4.0 - len) / len;
            }
        }

        self.latent = Some(xz);
        self.towards_sun = Self::calculate_towards_sun(xz);
    }
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalLightsState {
    pub theta: f32,
    pub phi: f32,
    pub count: u32,
    pub distance: f32,
    pub multiplier: f32,
}

impl Default for LocalLightsState {
    fn default() -> Self {
        Self {
            theta: 1.0,
            phi: 1.0,
            count: 0,
            distance: 1.5,
            multiplier: 10.0,
        }
    }
}

pub trait ShouldResetPathTracer {
    fn should_reset_path_tracer(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraState {
    pub position: Vec3,
    pub rotation: Quat,
    pub vertical_fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: Vec3::ONE,
            rotation: Quat::IDENTITY,
            vertical_fov: 62.0,
        }
    }
}

impl ShouldResetPathTracer for CameraState {
    fn should_reset_path_tracer(&self, other: &Self) -> bool {
        !self.position.abs_diff_eq(other.position, 1e-5)
            || !self.rotation.abs_diff_eq(other.rotation, 1e-5)
            || self.vertical_fov != other.vertical_fov
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct LightState {
    pub emissive_multiplier: f32,
    pub enable_emissive: bool,
    pub sun: SunState,
    pub local_lights: LocalLightsState,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            emissive_multiplier: 1.0,
            enable_emissive: true,
            sun: SunState::default(),
            local_lights: LocalLightsState::default(),
        }
    }
}

impl ShouldResetPathTracer for LightState {
    fn should_reset_path_tracer(&self, other: &Self) -> bool {
        self.emissive_multiplier != other.emissive_multiplier
            || self.enable_emissive != other.enable_emissive
            || self.sun != other.sun
            || self.local_lights != other.local_lights
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MovementState {
    pub camera_speed: f32,
    pub camera_smoothness: f32,
    pub sun_rotation_smoothness: f32,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            camera_speed: 2.5,
            camera_smoothness: 1.0,
            sun_rotation_smoothness: 0.0,
        }
    }
}

impl ShouldResetPathTracer for MovementState {}

fn default_contrast() -> f32 {
    1.0
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ExposureState {
    pub ev_shift: f32,
    #[serde(default)]
    pub use_dynamic_adaptation: bool,
    #[serde(default)]
    pub dynamic_adaptation_speed: f32,
    #[serde(default)]
    pub dynamic_adaptation_low_clip: f32,
    #[serde(default)]
    pub dynamic_adaptation_high_clip: f32,
    #[serde(default = "default_contrast")]
    pub contrast: f32,
}

impl Default for ExposureState {
    fn default() -> Self {
        Self {
            ev_shift: 0.0,
            use_dynamic_adaptation: false,
            dynamic_adaptation_speed: 0.0,
            dynamic_adaptation_low_clip: 0.0,
            dynamic_adaptation_high_clip: 0.0,
            contrast: default_contrast(),
        }
    }
}

impl ShouldResetPathTracer for ExposureState {}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SceneElementTransform {
    pub position: Vec3,
    pub rotation_euler_degrees: Vec3,
    pub scale: Vec3,
}

impl SceneElementTransform {
    pub const IDENTITY: SceneElementTransform = SceneElementTransform {
        position: Vec3::ZERO,
        rotation_euler_degrees: Vec3::ZERO,
        scale: Vec3::ONE,
    };

    pub fn from_position_rotation_scale(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);

        Self {
            position,
            rotation_euler_degrees: Vec3::new(
                pitch.to_degrees(),
                yaw.to_degrees(),
                roll.to_degrees(),
            ),
            scale,
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            self.rotation_euler_degrees.y.to_radians(),
            self.rotation_euler_degrees.x.to_radians(),
            self.rotation_euler_degrees.z.to_radians(),
        )
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
        self.rotation_euler_degrees = Vec3::new(
            pitch.to_degrees(),
            yaw.to_degrees(),
            roll.to_degrees(),
        );
    }

    pub fn affine_transform(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation(), self.position)
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct GameObjectId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinGameObjectKind {
    MainCamera,
    Sun,
    LocalLights,
}

impl BuiltinGameObjectKind {
    pub fn display_name(self) -> &'static str {
        match self {
            BuiltinGameObjectKind::MainCamera => "Main Camera",
            BuiltinGameObjectKind::Sun => "Sun",
            BuiltinGameObjectKind::LocalLights => "Local Lights",
        }
    }

    pub fn matches_component(self, component: &SceneComponent) -> bool {
        matches!(
            (self, component),
            (BuiltinGameObjectKind::MainCamera, SceneComponent::Camera(_))
                | (BuiltinGameObjectKind::Sun, SceneComponent::Sun(_))
                | (BuiltinGameObjectKind::LocalLights, SceneComponent::LocalLights(_))
        )
    }
}

fn default_game_object_enabled() -> bool {
    true
}

fn default_component_enabled() -> bool {
    true
}

fn default_component_emissive_multiplier() -> f32 {
    1.0
}

fn default_next_game_object_id() -> u64 {
    1
}

fn default_camera_primary() -> bool {
    true
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
            vertical_fov: CameraState::default().vertical_fov,
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
        let sun = SunState::default();
        Self {
            enabled: true,
            controller: sun.controller,
            size_multiplier: sun.size_multiplier,
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
        let local_lights = LocalLightsState::default();
        Self {
            enabled: true,
            theta: local_lights.theta,
            phi: local_lights.phi,
            count: local_lights.count,
            distance: local_lights.distance,
            multiplier: local_lights.multiplier,
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

impl From<&LightState> for SceneRenderSettings {
    fn from(value: &LightState) -> Self {
        Self {
            emissive_multiplier: value.emissive_multiplier,
            enable_emissive: value.enable_emissive,
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

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GameObject {
    pub id: GameObjectId,
    pub name: String,

    #[serde(default = "default_game_object_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub builtin: Option<BuiltinGameObjectKind>,

    #[serde(default)]
    pub parent: Option<GameObjectId>,

    #[serde(default)]
    pub transform: SceneElementTransform,

    #[serde(default)]
    pub components: Vec<SceneComponent>,
}

impl GameObject {
    pub fn new(
        id: GameObjectId,
        name: impl Into<String>,
        transform: SceneElementTransform,
    ) -> Self {
        let name = Self::sanitized_name(id, &name.into());
        Self {
            id,
            name,
            enabled: true,
            builtin: None,
            parent: None,
            transform,
            components: Vec::new(),
        }
    }

    pub fn sanitized_name(id: GameObjectId, name: &str) -> String {
        let sanitized = sanitize_ascii_label(name);
        if sanitized.is_empty() {
            default_game_object_name_for_id(id)
        } else {
            sanitized
        }
    }

    pub fn is_builtin(&self) -> bool {
        self.builtin.is_some()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(from = "SceneStateSerde")]
pub struct SceneState {
    #[serde(default = "default_next_game_object_id")]
    pub next_game_object_id: u64,

    #[serde(default)]
    pub game_objects: Vec<GameObject>,

    #[serde(default)]
    pub render_settings: SceneRenderSettings,

    #[serde(default)]
    pub ibl: Option<PathBuf>,
}

impl Default for SceneState {
    fn default() -> Self {
        let mut scene = Self {
            next_game_object_id: default_next_game_object_id(),
            game_objects: Vec::new(),
            render_settings: SceneRenderSettings {
                emissive_multiplier: 1.0,
                enable_emissive: true,
            },
            ibl: None,
        };
        scene.ensure_editor_singletons();
        scene
    }
}

impl SceneState {
    pub fn first_component_index(
        &self,
        predicate: impl Fn(&SceneComponent) -> bool,
    ) -> Option<(usize, usize)> {
        for (game_object_index, game_object) in self.game_objects.iter().enumerate() {
            for (component_index, component) in game_object.components.iter().enumerate() {
                if predicate(component) {
                    return Some((game_object_index, component_index));
                }
            }
        }

        None
    }

    pub fn primary_camera_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::Camera(camera) if camera.enabled && camera.primary)
        })
        .or_else(|| {
            self.first_component_index(|component| {
                matches!(component, SceneComponent::Camera(camera) if camera.enabled)
            })
        })
        .or_else(|| self.first_component_index(|component| matches!(component, SceneComponent::Camera(_))))
    }

    pub fn sun_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::Sun(sun) if sun.enabled)
        })
        .or_else(|| self.first_component_index(|component| matches!(component, SceneComponent::Sun(_))))
    }

    pub fn local_lights_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::LocalLights(local_lights) if local_lights.enabled)
        })
        .or_else(|| {
            self.first_component_index(|component| matches!(component, SceneComponent::LocalLights(_)))
        })
    }

    pub fn with_primary_camera<R>(
        &self,
        f: impl FnOnce(&SceneElementTransform, &CameraComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.primary_camera_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_camera()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_primary_camera_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut CameraComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.primary_camera_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object.components.get_mut(component_index)?.as_camera_mut()?;
        Some(f(transform, component))
    }

    pub fn with_sun<R>(&self, f: impl FnOnce(&SceneElementTransform, &SunComponent) -> R) -> Option<R> {
        let (game_object_index, component_index) = self.sun_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_sun()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_sun_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut SunComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.sun_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object.components.get_mut(component_index)?.as_sun_mut()?;
        Some(f(transform, component))
    }

    pub fn with_local_lights<R>(
        &self,
        f: impl FnOnce(&SceneElementTransform, &LocalLightsComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.local_lights_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_local_lights()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_local_lights_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut LocalLightsComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.local_lights_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object
            .components
            .get_mut(component_index)?
            .as_local_lights_mut()?;
        Some(f(transform, component))
    }

    pub fn allocate_game_object_id(&mut self) -> GameObjectId {
        let id = GameObjectId(self.next_game_object_id.max(default_next_game_object_id()));
        self.next_game_object_id = id.0.saturating_add(1);
        id
    }

    pub fn create_game_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
    ) -> GameObjectId {
        let id = self.allocate_game_object_id();
        self.game_objects.push(GameObject::new(id, name, transform));
        id
    }

    pub fn create_mesh_object(
        &mut self,
        name: impl Into<String>,
        source: MeshSource,
        transform: SceneElementTransform,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        if let Some(game_object) = self.find_game_object_mut(id) {
            game_object
                .components
                .push(SceneComponent::MeshRenderer(MeshRendererComponent::new(source)));
        }
        id
    }

    pub fn add_component(&mut self, game_object_id: GameObjectId, component: SceneComponent) -> bool {
        if let Some(game_object) = self.find_game_object_mut(game_object_id) {
            game_object.components.push(component);
            true
        } else {
            false
        }
    }

    pub fn create_camera_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: CameraComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::Camera(component));
        id
    }

    pub fn create_sun_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: SunComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::Sun(component));
        id
    }

    pub fn create_local_lights_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: LocalLightsComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::LocalLights(component));
        id
    }

    pub fn find_game_object(&self, id: GameObjectId) -> Option<&GameObject> {
        self.game_objects.iter().find(|game_object| game_object.id == id)
    }

    pub fn find_game_object_mut(&mut self, id: GameObjectId) -> Option<&mut GameObject> {
        self.game_objects
            .iter_mut()
            .find(|game_object| game_object.id == id)
    }

    fn is_descendant_of_in(
        game_objects: &[GameObject],
        descendant: GameObjectId,
        ancestor: GameObjectId,
    ) -> bool {
        let mut current = game_objects
            .iter()
            .find(|game_object| game_object.id == descendant)
            .and_then(|game_object| game_object.parent);
        let mut visited = Vec::new();

        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }

            if visited.contains(&parent) {
                break;
            }

            visited.push(parent);
            current = game_objects
                .iter()
                .find(|game_object| game_object.id == parent)
                .and_then(|game_object| game_object.parent);
        }

        false
    }

    pub fn can_reparent_game_object(
        &self,
        child_id: GameObjectId,
        new_parent: Option<GameObjectId>,
    ) -> bool {
        let Some(child) = self.find_game_object(child_id) else {
            return false;
        };

        if child.parent == new_parent {
            return false;
        }

        if child.is_builtin() && new_parent.is_some() {
            return false;
        }

        let Some(new_parent) = new_parent else {
            return true;
        };

        if new_parent == child_id || self.find_game_object(new_parent).is_none() {
            return false;
        }

        !Self::is_descendant_of_in(&self.game_objects, new_parent, child_id)
    }

    pub fn reparent_game_object(
        &mut self,
        child_id: GameObjectId,
        new_parent: Option<GameObjectId>,
    ) -> bool {
        if !self.can_reparent_game_object(child_id, new_parent) {
            return false;
        }

        let mut subtree_ids = Vec::new();
        self.collect_subtree_ids(child_id, &mut subtree_ids);
        if subtree_ids.is_empty() {
            return false;
        }

        let mut subtree = Vec::new();
        let mut remaining = Vec::with_capacity(self.game_objects.len().saturating_sub(subtree_ids.len()));

        for game_object in std::mem::take(&mut self.game_objects) {
            if subtree_ids.contains(&game_object.id) {
                subtree.push(game_object);
            } else {
                remaining.push(game_object);
            }
        }

        let Some(child) = subtree
            .iter_mut()
            .find(|game_object| game_object.id == child_id)
        else {
            self.game_objects = remaining;
            return false;
        };

        child.parent = new_parent;

        let insert_index = if let Some(new_parent_id) = new_parent {
            remaining
                .iter()
                .enumerate()
                .rev()
                .find(|(_, game_object)| {
                    game_object.id == new_parent_id
                        || Self::is_descendant_of_in(&remaining, game_object.id, new_parent_id)
                })
                .map(|(index, _)| index + 1)
                .unwrap_or(remaining.len())
        } else {
            remaining.len()
        };

        let mut tail = remaining.split_off(insert_index);
        remaining.extend(subtree);
        remaining.append(&mut tail);
        self.game_objects = remaining;
        true
    }

    pub fn refresh_editor_singletons(&mut self) {
        self.ensure_editor_singletons();
    }

    pub fn remove_game_object(&mut self, id: GameObjectId) -> bool {
        let mut subtree = Vec::new();
        self.collect_subtree_ids(id, &mut subtree);
        if subtree.is_empty() {
            return false;
        }

        self.game_objects
            .retain(|game_object| !subtree.contains(&game_object.id));
        self.ensure_editor_singletons();
        true
    }

    pub fn copy_editor_singletons_from(&mut self, other: &SceneState) {
        self.render_settings = other.render_settings.clone();

        if let Some((transform, camera)) = other.with_primary_camera(|transform, camera| {
            (transform.clone(), camera.clone())
        }) {
            let _ = self.with_primary_camera_mut(|dest_transform, dest_camera| {
                *dest_transform = transform;
                *dest_camera = camera;
            });
        }

        if let Some((transform, sun)) = other.with_sun(|transform, sun| (transform.clone(), sun.clone()))
        {
            let _ = self.with_sun_mut(|dest_transform, dest_sun| {
                *dest_transform = transform;
                *dest_sun = sun;
            });
        }

        if let Some((transform, local_lights)) =
            other.with_local_lights(|transform, local_lights| {
                (transform.clone(), local_lights.clone())
            })
        {
            let _ = self.with_local_lights_mut(|dest_transform, dest_local_lights| {
                *dest_transform = transform;
                *dest_local_lights = local_lights;
            });
        }
    }

    pub fn world_transform(&self, id: GameObjectId) -> Affine3A {
        let mut visited = Vec::new();
        self.world_transform_inner(id, &mut visited)
    }

    fn collect_subtree_ids(&self, id: GameObjectId, subtree: &mut Vec<GameObjectId>) {
        if subtree.contains(&id) || self.find_game_object(id).is_none() {
            return;
        }

        subtree.push(id);

        let child_ids: Vec<_> = self
            .game_objects
            .iter()
            .filter(|game_object| game_object.parent == Some(id))
            .map(|game_object| game_object.id)
            .collect();

        for child_id in child_ids {
            self.collect_subtree_ids(child_id, subtree);
        }
    }

    fn normalize(&mut self) {
        let mut seen_ids = Vec::new();
        let mut next_game_object_id = self.next_game_object_id.max(default_next_game_object_id());

        for game_object in &mut self.game_objects {
            if game_object.id.0 == 0 || seen_ids.contains(&game_object.id) {
                game_object.id = GameObjectId(next_game_object_id);
                next_game_object_id = next_game_object_id.saturating_add(1);
            }

            seen_ids.push(game_object.id);
            next_game_object_id = next_game_object_id.max(game_object.id.0.saturating_add(1));

            game_object.name = GameObject::sanitized_name(game_object.id, &game_object.name);
        }

        for game_object in &mut self.game_objects {
            if let Some(parent) = game_object.parent {
                if !seen_ids.contains(&parent)
                    || parent == game_object.id
                    || game_object.is_builtin()
                {
                    game_object.parent = None;
                }
            }
        }

        self.next_game_object_id = next_game_object_id.max(default_next_game_object_id());
        self.ensure_editor_singletons();
    }

    fn ensure_editor_singletons(&mut self) {
        if self.primary_camera_component_index().is_none() {
            let camera = CameraState::default();
            self.create_camera_object(
                "Main Camera",
                SceneElementTransform::from_position_rotation_scale(
                    camera.position,
                    camera.rotation,
                    Vec3::ONE,
                ),
                CameraComponent {
                    vertical_fov: camera.vertical_fov,
                    ..Default::default()
                },
            );
        }

        if self.sun_component_index().is_none() {
            self.create_sun_object(
                "Sun",
                SceneElementTransform::IDENTITY,
                SunComponent::default(),
            );
        }

        if self.local_lights_component_index().is_none() {
            self.create_local_lights_object(
                "Local Lights",
                SceneElementTransform::IDENTITY,
                LocalLightsComponent::default(),
            );
        }

        for game_object in &mut self.game_objects {
            game_object.builtin = None;
        }

        if let Some((game_object_index, _)) = self.primary_camera_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::MainCamera);
                game_object.name = BuiltinGameObjectKind::MainCamera.display_name().to_owned();
            }
        }

        if let Some((game_object_index, _)) = self.sun_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::Sun);
                game_object.name = BuiltinGameObjectKind::Sun.display_name().to_owned();
            }
        }

        if let Some((game_object_index, _)) = self.local_lights_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::LocalLights);
                game_object.name = BuiltinGameObjectKind::LocalLights.display_name().to_owned();
            }
        }
    }

    fn world_transform_inner(&self, id: GameObjectId, visited: &mut Vec<GameObjectId>) -> Affine3A {
        let Some(game_object) = self.find_game_object(id) else {
            return Affine3A::IDENTITY;
        };

        let local_transform = game_object.transform.affine_transform();
        if visited.contains(&id) {
            return local_transform;
        }

        let parent_transform = if let Some(parent) = game_object.parent {
            visited.push(id);
            let transform = self.world_transform_inner(parent, visited);
            visited.pop();
            transform
        } else {
            Affine3A::IDENTITY
        };

        parent_transform * local_transform
    }
}

impl ShouldResetPathTracer for SceneState {
    fn should_reset_path_tracer(&self, other: &Self) -> bool {
        self.game_objects != other.game_objects || self.ibl != other.ibl
    }
}

#[derive(serde::Deserialize)]
struct SceneStateSerde {
    #[serde(default = "default_next_game_object_id")]
    next_game_object_id: u64,

    #[serde(default)]
    game_objects: Vec<GameObject>,

    #[serde(default)]
    render_settings: SceneRenderSettings,

    #[serde(default)]
    ibl: Option<PathBuf>,

    #[serde(default)]
    elements: Vec<LegacySceneElement>,
}

#[derive(serde::Deserialize)]
struct LegacySceneElement {
    pub source: MeshSource,

    #[serde(default)]
    pub transform: SceneElementTransform,
}

impl From<SceneStateSerde> for SceneState {
    fn from(value: SceneStateSerde) -> Self {
        let mut scene = if value.game_objects.is_empty() && !value.elements.is_empty() {
            let mut scene = SceneState {
                next_game_object_id: value.next_game_object_id,
                game_objects: Vec::new(),
                render_settings: value.render_settings,
                ibl: value.ibl,
            };

            for element in value.elements {
                let name = element.source.default_game_object_name();
                scene.create_mesh_object(name, element.source, element.transform);
            }

            scene
        } else {
            SceneState {
                next_game_object_id: value.next_game_object_id,
                game_objects: value.game_objects,
                render_settings: value.render_settings,
                ibl: value.ibl,
            }
        };

        scene.normalize();
        scene
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(from = "PersistedStateSerde")]
pub struct PersistedState {
    pub exposure: ExposureState,
    pub movement: MovementState,
    pub sequence: Sequence,
    #[serde(default)]
    pub scene: SceneState,
}

impl ShouldResetPathTracer for PersistedState {
    fn should_reset_path_tracer(&self, other: &Self) -> bool {
        self.exposure.should_reset_path_tracer(&other.exposure)
            || self.movement.should_reset_path_tracer(&other.movement)
            || self.scene.should_reset_path_tracer(&other.scene)
    }
}

#[derive(serde::Deserialize)]
struct PersistedStateSerde {
    #[serde(default)]
    camera: Option<CameraState>,

    #[serde(default)]
    light: Option<LightState>,

    #[serde(default)]
    exposure: ExposureState,

    #[serde(default)]
    movement: MovementState,

    #[serde(default)]
    sequence: Sequence,

    #[serde(default)]
    scene: SceneState,
}

impl From<PersistedStateSerde> for PersistedState {
    fn from(value: PersistedStateSerde) -> Self {
        let mut scene = value.scene;

        if let Some(camera) = value.camera {
            let _ = scene.with_primary_camera_mut(|transform, camera_component| {
                *transform = SceneElementTransform::from_position_rotation_scale(
                    camera.position,
                    camera.rotation,
                    Vec3::ONE,
                );
                camera_component.vertical_fov = camera.vertical_fov;
            });
        }

        if let Some(light) = value.light {
            scene.render_settings = SceneRenderSettings::from(&light);

            let _ = scene.with_sun_mut(|_, sun_component| {
                sun_component.controller = light.sun.controller.clone();
                sun_component.size_multiplier = light.sun.size_multiplier;
            });

            let _ = scene.with_local_lights_mut(|_, local_lights_component| {
                local_lights_component.theta = light.local_lights.theta;
                local_lights_component.phi = light.local_lights.phi;
                local_lights_component.count = light.local_lights.count;
                local_lights_component.distance = light.local_lights.distance;
                local_lights_component.multiplier = light.local_lights.multiplier;
            });
        }

        PersistedState {
            exposure: value.exposure,
            movement: value.movement,
            sequence: value.sequence,
            scene,
        }
    }
}
