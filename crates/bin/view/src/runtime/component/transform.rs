use dist_render_simple::{Affine3A, EulerRot, Quat, Vec3};

pub const DEFAULT_MAIN_CAMERA_POSITION: Vec3 = Vec3::ONE;
pub const DEFAULT_MAIN_CAMERA_ROTATION: Quat = Quat::IDENTITY;

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

pub fn default_main_camera_transform() -> SceneElementTransform {
    SceneElementTransform::from_position_rotation_scale(
        DEFAULT_MAIN_CAMERA_POSITION,
        DEFAULT_MAIN_CAMERA_ROTATION,
        Vec3::ONE,
    )
}