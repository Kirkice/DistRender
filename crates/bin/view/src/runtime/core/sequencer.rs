use super::super::*;

impl RuntimeState {
    pub fn is_sequence_playing(&self) -> bool {
        matches!(
            &self.sequence_playback_state,
            SequencePlaybackState::Playing { .. }
        )
    }

    pub fn stop_sequence(&mut self) {
        self.sequence_playback_state = SequencePlaybackState::NotPlaying;
    }

    pub fn play_sequence(&mut self, persisted: &mut PersistedState) {
        const PLAYBACK_WARMUP_DURATION: f32 = 0.5;

        let t = self
            .active_camera_key
            .and_then(|i| Some(persisted.sequence.get_item(i)?.t))
            .unwrap_or(-PLAYBACK_WARMUP_DURATION);

        self.sequence_playback_state = SequencePlaybackState::Playing {
            t,
            sequence: persisted.sequence.to_playback(),
        };
    }

    pub fn add_sequence_keyframe(&mut self, persisted: &mut PersistedState) {
        persisted.sequence.add_keyframe(
            self.active_camera_key,
            SequenceValue {
                camera_position: MemOption::new(self.camera.final_transform.position),
                camera_direction: MemOption::new(self.camera.final_transform.rotation * -Vec3::Z),
                towards_sun: MemOption::new(Self::current_sun_direction(&persisted.scene)),
            },
        );

        if let Some(idx) = &mut self.active_camera_key {
            *idx += 1;
        }
    }

    pub fn jump_to_sequence_key(&mut self, persisted: &mut PersistedState, idx: usize) {
        let exact_item = if let Some(item) = persisted.sequence.get_item(idx) {
            item.clone()
        } else {
            return;
        };

        if let Some(value) = persisted.sequence.to_playback().sample(exact_item.t) {
            self.camera.driver_mut::<Position>().position = exact_item
                .value
                .camera_position
                .unwrap_or(value.camera_position);
            self.camera
                .driver_mut::<YawPitch>()
                .set_rotation_quat(dolly::util::look_at::<dolly::handedness::RightHanded>(
                    exact_item
                        .value
                        .camera_direction
                        .unwrap_or(value.camera_direction),
                ));

            self.camera.update(1e10);
            self.write_camera_rig_to_scene(&mut persisted.scene);

            let _ = persisted.scene.with_sun_mut(|_, sun| {
                sun.controller
                    .set_towards_sun(exact_item.value.towards_sun.unwrap_or(value.towards_sun));
            });
        }

        self.active_camera_key = Some(idx);
        self.sequence_playback_state = SequencePlaybackState::NotPlaying;
    }

    pub fn replace_camera_sequence_key(&mut self, persisted: &mut PersistedState, idx: usize) {
        persisted.sequence.each_key(|i, item| {
            if idx != i {
                return;
            }

            item.value.camera_position = MemOption::new(self.camera.final_transform.position);
            item.value.camera_direction = MemOption::new(self.camera.final_transform.rotation * -Vec3::Z);
            item.value.towards_sun = MemOption::new(Self::current_sun_direction(&persisted.scene));
        })
    }

    pub fn delete_camera_sequence_key(&mut self, persisted: &mut PersistedState, idx: usize) {
        persisted.sequence.delete_key(idx);

        self.active_camera_key = None;
    }
}