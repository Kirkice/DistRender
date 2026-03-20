use super::super::{component::{default_main_camera_transform, DEFAULT_CAMERA_VERTICAL_FOV}, *};

impl RuntimeState {
    fn active_camera_transform(scene: &SceneState) -> SceneElementTransform {
        scene
            .with_primary_camera(|transform, _| transform.clone())
            .unwrap_or_else(default_main_camera_transform)
    }

    pub(crate) fn active_camera_vertical_fov(scene: &SceneState) -> f32 {
        scene
            .with_primary_camera(|_, camera| camera.vertical_fov)
            .unwrap_or(DEFAULT_CAMERA_VERTICAL_FOV)
    }

    pub(crate) fn active_camera_position_and_rotation(scene: &SceneState) -> (Vec3, Quat) {
        let transform = Self::active_camera_transform(scene);
        (transform.position, transform.rotation())
    }

    pub(crate) fn current_sun_direction(scene: &SceneState) -> Vec3 {
        scene
            .with_sun(|_, sun| sun.controller.towards_sun())
            .unwrap_or(Vec3::Y)
    }

    fn current_sun_size_multiplier(scene: &SceneState) -> f32 {
        scene.with_sun(|_, sun| sun.size_multiplier).unwrap_or(1.0)
    }

    pub(crate) fn sync_camera_rig_from_scene(&mut self, scene: &SceneState) {
        let (position, rotation) = Self::active_camera_position_and_rotation(scene);
        self.camera.driver_mut::<Position>().position = position;
        self.camera
            .driver_mut::<YawPitch>()
            .set_rotation_quat(rotation);
        self.camera.update(1e10);
    }

    pub(crate) fn write_camera_rig_to_scene(&self, scene: &mut SceneState) {
        let position = self.camera.final_transform.position;
        let rotation = self.camera.final_transform.rotation;

        let _ = scene.with_primary_camera_mut(|transform, _| {
            transform.position = position;
            transform.set_rotation(rotation);
        });
    }

    pub(in crate::runtime) fn update_camera(&mut self, persisted: &mut PersistedState, ctx: &FrameContext) {
        self.sync_camera_rig_from_scene(&persisted.scene);

        let viewport_input_active = !self.show_gui
            || self.viewport_keyboard_focused
            || self.viewport_hovered
            || self.viewport_pointer_captured;
        let viewport_speed_adjust_active = !self.show_gui
            || self.viewport_hovered
            || self.viewport_pointer_captured;
        let camera_rotating = (self.mouse.buttons_held & (1 << 2)) != 0 && viewport_input_active;

        let smooth = self.camera.driver_mut::<Smooth>();
        if ctx.world_renderer.render_mode == RenderMode::Reference {
            smooth.position_smoothness = 0.0;
            smooth.rotation_smoothness = 0.0;
        } else {
            smooth.position_smoothness = persisted.movement.camera_smoothness;
            smooth.rotation_smoothness = if camera_rotating {
                0.0
            } else {
                persisted.movement.camera_smoothness
            };
        }

        if (self.mouse.buttons_pressed & (1 << 2)) != 0 && viewport_input_active {
            let _ = ctx.window.set_cursor_grab(true);
            self.grab_cursor_pos = self.mouse.physical_position;
            ctx.window.set_cursor_visible(false);
        }

        if (self.mouse.buttons_released & (1 << 2)) != 0 {
            let _ = ctx.window.set_cursor_grab(false);
            ctx.window.set_cursor_visible(true);
        }

        let move_vec = if viewport_input_active {
            let input = self.movement_map.map(&self.keyboard, ctx.dt_filtered);
            self.camera.final_transform.rotation
                * Vec3::new(input["move_right"], input["move_up"], -input["move_fwd"])
                    .clamp_length_max(1.0)
                * 4.0f32.powf(input["boost"])
        } else {
            Vec3::ZERO
        };

        if self.mouse.wheel_delta.abs() > f32::EPSILON && viewport_speed_adjust_active {
            const CAMERA_SPEED_SCROLL_STEP: f32 = 1.2;

            let camera_speed = persisted.movement.camera_speed.max(MIN_CAMERA_SPEED);
            persisted.movement.camera_speed = (camera_speed
                * CAMERA_SPEED_SCROLL_STEP.powf(self.mouse.wheel_delta))
            .clamp(MIN_CAMERA_SPEED, MAX_CAMERA_SPEED);
        }

        if camera_rotating {
            let _ = ctx
                .window
                .set_cursor_position(winit::dpi::PhysicalPosition::new(
                    self.grab_cursor_pos.x,
                    self.grab_cursor_pos.y,
                ));

            let sensitivity = 0.1;
            self.camera.driver_mut::<YawPitch>().rotate_yaw_pitch(
                -sensitivity * self.mouse.delta.x,
                -sensitivity * self.mouse.delta.y,
            );
        }

        self.camera
            .driver_mut::<Position>()
            .translate(move_vec * ctx.dt_filtered * persisted.movement.camera_speed);

        if let SequencePlaybackState::Playing { t, sequence } = &mut self.sequence_playback_state {
            let smooth = self.camera.driver_mut::<Smooth>();
            if *t <= 0.0 {
                smooth.position_smoothness = 0.0;
                smooth.rotation_smoothness = 0.0;
            } else {
                smooth.position_smoothness = persisted.movement.camera_smoothness;
                smooth.rotation_smoothness = persisted.movement.camera_smoothness;
            }

            if let Some(value) = sequence.sample(t.max(0.0)) {
                self.camera.driver_mut::<Position>().position = value.camera_position;
                self.camera
                    .driver_mut::<YawPitch>()
                    .set_rotation_quat(dolly::util::look_at::<dolly::handedness::RightHanded>(
                        value.camera_direction,
                    ));
                let _ = persisted.scene.with_sun_mut(|_, sun| {
                    sun.controller.set_towards_sun(value.towards_sun);
                });

                *t += ctx.dt_filtered * self.sequence_playback_speed;
            } else {
                self.sequence_playback_state = SequencePlaybackState::NotPlaying;
            }
        }

        self.camera.update(ctx.dt_filtered);
        self.write_camera_rig_to_scene(&mut persisted.scene);

        if self
            .keyboard
            .was_just_pressed(self.keymap_config.misc.print_camera_transform)
        {
            println!(
                "position: {}, look_at: {}",
                self.camera.final_transform.position,
                self.camera.final_transform.position + self.camera.final_transform.rotation * -Vec3::Z,
            );
        }
    }

    pub(in crate::runtime) fn update_sun(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        let viewport_input_active = !self.show_gui
            || self.viewport_keyboard_focused
            || self.viewport_hovered
            || self.viewport_pointer_captured;

        if self.mouse.buttons_held & 1 != 0 && viewport_input_active && !self.show_gui {
            let delta_x =
                (self.mouse.delta.x / ctx.render_extent[0] as f32) * std::f32::consts::TAU;
            let delta_y = (self.mouse.delta.y / ctx.render_extent[1] as f32) * std::f32::consts::PI;

            match self.left_click_edit_mode {
                LeftClickEditMode::MoveSun => {
                    let ref_frame = Quat::from_xyzw(
                        0.0,
                        self.camera.final_transform.rotation.y,
                        0.0,
                        self.camera.final_transform.rotation.w,
                    )
                    .normalize();

                    let _ = persisted.scene.with_sun_mut(|_, sun| {
                        sun.controller.view_space_rotate(&ref_frame, delta_x, delta_y);
                    });
                }
            }
        }

        let sun_direction = Self::current_sun_direction(&persisted.scene);
        if (sun_direction.dot(self.sun_direction_interp) - 1.0).abs() > 1e-5 {
            self.reset_path_tracer = true;
        }

        let sun_interp_t = if ctx.world_renderer.render_mode == RenderMode::Reference {
            1.0
        } else {
            (-1.0 * persisted.movement.sun_rotation_smoothness).exp2()
        };

        self.sun_direction_interp =
            Vec3::lerp(self.sun_direction_interp, sun_direction, sun_interp_t).normalize();

        ctx.world_renderer.sun_size_multiplier = Self::current_sun_size_multiplier(&persisted.scene);
    }
}