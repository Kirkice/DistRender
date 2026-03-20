use super::super::*;

impl RuntimeState {
    pub(in crate::runtime) fn update_lights(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.keyboard.was_just_pressed(
            self.keymap_config
                .rendering
                .switch_to_reference_path_tracing,
        ) {
            match ctx.world_renderer.render_mode {
                RenderMode::Standard => {
                    ctx.world_renderer.render_mode = RenderMode::Reference;
                }
                RenderMode::Reference => {
                    ctx.world_renderer.render_mode = RenderMode::Standard;
                }
            };
        }

        if self
            .keyboard
            .was_just_pressed(self.keymap_config.rendering.light_enable_emissive)
        {
            persisted.scene.render_settings.enable_emissive =
                !persisted.scene.render_settings.enable_emissive;
        }
    }

    pub(in crate::runtime) fn update_objects(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if let Err(err) = self.runtime_scene.sync(
            &persisted.scene,
            ctx.world_renderer,
            Self::scene_sync_options(persisted),
        ) {
            log::error!("Failed to sync scene bindings: {:#}", err);
        }
    }
}