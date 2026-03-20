use super::super::*;

impl RuntimeState {
    pub fn frame(
        &mut self,
        mut ctx: FrameContext,
        persisted: &mut PersistedState,
    ) -> WorldFrameDesc {
        if self.max_fps != MAX_FPS_LIMIT {
            std::thread::sleep(std::time::Duration::from_micros(
                1_000_000 / self.max_fps as u64,
            ));
        }

        self.keyboard.update(ctx.events);
        self.mouse.update(ctx.events);
        self.handle_file_drop_events(persisted, ctx.world_renderer, ctx.events);

        let orig_persisted_state = persisted.clone();
        let orig_render_overrides = ctx.world_renderer.render_overrides;

        self.do_gui(persisted, &mut ctx);
        self.update_lights(persisted, &mut ctx);
        self.update_objects(persisted, &mut ctx);
        self.update_sun(persisted, &mut ctx);
        self.update_camera(persisted, &ctx);

        if self
            .keyboard
            .was_just_pressed(self.keymap_config.sequencer.add_keyframe)
            || (self.mouse.buttons_pressed & (1 << 1)) != 0
        {
            self.add_sequence_keyframe(persisted);
        }

        if self
            .keyboard
            .was_just_pressed(self.keymap_config.sequencer.play)
        {
            match self.sequence_playback_state {
                SequencePlaybackState::NotPlaying => {
                    self.play_sequence(persisted);
                }
                SequencePlaybackState::Playing { .. } => {
                    self.stop_sequence();
                }
            };
        }

        ctx.world_renderer.ev_shift = persisted.exposure.ev_shift;
        ctx.world_renderer.contrast = persisted.exposure.contrast;
        ctx.world_renderer.dynamic_exposure.enabled = persisted.exposure.use_dynamic_adaptation;
        ctx.world_renderer.dynamic_exposure.speed_log2 =
            persisted.exposure.dynamic_adaptation_speed;
        ctx.world_renderer.dynamic_exposure.histogram_clipping.low =
            persisted.exposure.dynamic_adaptation_low_clip;
        ctx.world_renderer.dynamic_exposure.histogram_clipping.high =
            persisted.exposure.dynamic_adaptation_high_clip;

        if persisted.should_reset_path_tracer(&orig_persisted_state)
            || ctx.world_renderer.render_overrides != orig_render_overrides
        {
            self.reset_path_tracer = true;
        }

        if (self.reset_path_tracer
            || self
                .keyboard
                .was_just_pressed(self.keymap_config.rendering.reset_path_tracer))
            && ctx.world_renderer.render_mode == RenderMode::Reference
        {
            ctx.world_renderer.reset_reference_accumulation = true;
            self.reset_path_tracer = false;
        }

        let lens = CameraLens {
            aspect_ratio: ctx.aspect_ratio(),
            vertical_fov: Self::active_camera_vertical_fov(&persisted.scene),
            ..Default::default()
        };

        WorldFrameDesc {
            camera_matrices: self
                .camera
                .final_transform
                .into_position_rotation()
                .through(&lens),
            render_extent: ctx.render_extent,
            sun_direction: self.sun_direction_interp,
        }
    }
}