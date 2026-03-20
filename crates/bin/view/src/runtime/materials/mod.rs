use super::*;

#[derive(Clone, Copy)]
pub struct SyncSceneOptions {
    pub global_emissive_multiplier: f32,
    pub emissive_enabled: bool,
}

impl Default for SyncSceneOptions {
    fn default() -> Self {
        Self {
            global_emissive_multiplier: 1.0,
            emissive_enabled: true,
        }
    }
}

impl RuntimeState {
    pub(crate) fn scene_sync_options(persisted: &PersistedState) -> SyncSceneOptions {
        SyncSceneOptions {
            global_emissive_multiplier: persisted.scene.render_settings.emissive_multiplier,
            emissive_enabled: persisted.scene.render_settings.enable_emissive,
        }
    }
}