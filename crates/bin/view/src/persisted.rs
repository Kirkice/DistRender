use dist_render_simple::{Quat, Vec3};

use crate::{
	runtime::component::{
		default_main_camera_transform, LocalLightsComponent, SceneElementTransform,
		SceneRenderSettings, SceneState, SunComponent, SunController,
		DEFAULT_CAMERA_VERTICAL_FOV,
	},
	runtime::Sequence,
};

pub trait ShouldResetPathTracer {
	fn should_reset_path_tracer(&self, _: &Self) -> bool {
		false
	}
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SunState {
	pub controller: SunController,
	pub size_multiplier: f32,
}

impl Default for SunState {
	fn default() -> Self {
		let sun = SunComponent::default();
		Self {
			controller: sun.controller,
			size_multiplier: sun.size_multiplier,
		}
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
		let local_lights = LocalLightsComponent::default();
		Self {
			theta: local_lights.theta,
			phi: local_lights.phi,
			count: local_lights.count,
			distance: local_lights.distance,
			multiplier: local_lights.multiplier,
		}
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
		let transform = default_main_camera_transform();
		Self {
			position: transform.position,
			rotation: transform.rotation(),
			vertical_fov: DEFAULT_CAMERA_VERTICAL_FOV,
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

impl From<&LightState> for SceneRenderSettings {
	fn from(value: &LightState) -> Self {
		Self {
			emissive_multiplier: value.emissive_multiplier,
			enable_emissive: value.enable_emissive,
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

impl ShouldResetPathTracer for SceneState {
	fn should_reset_path_tracer(&self, other: &Self) -> bool {
		self.game_objects != other.game_objects || self.ibl != other.ibl
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
