#![allow(clippy::single_match)]

use anyhow::Context;

use dolly::prelude::*;
use dist_render::{
    rg::GraphDebugHook,
    world_renderer::WorldRenderer,
};
use dist_render_simple::*;

use crate::{
    opt::Opt,
    persisted::{
        CameraState, GameObject, GameObjectId, MeshSource, SceneElementTransform, SceneState,
        ShouldResetPathTracer as _,
    },
    scene::SceneDesc,
    scene_runtime::{RuntimeScene, SyncSceneOptions},
    sequence::{CameraPlaybackSequence, MemOption, SequenceValue},
    PersistedState,
};

use crate::keymap::KeymapConfig;
use log::{info, warn};
use std::{fs::File, path::PathBuf};

pub const MAX_FPS_LIMIT: u32 = 256;

fn unique_game_object_name(scene: &SceneState, base_name: String) -> String {
    if !scene
        .game_objects
        .iter()
        .any(|game_object| game_object.name == base_name)
    {
        return base_name;
    }

    let mut suffix = 1;
    loop {
        let candidate = format!("{} {}", base_name, suffix);
        if !scene
            .game_objects
            .iter()
            .any(|game_object| game_object.name == candidate)
        {
            return candidate;
        }

        suffix += 1;
    }
}

pub struct RuntimeState {
    pub camera: CameraRig,
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub keymap_config: KeymapConfig,
    pub movement_map: KeyboardMap,

    pub show_gui: bool,
    pub sun_direction_interp: Vec3,
    pub left_click_edit_mode: LeftClickEditMode,

    pub max_fps: u32,
    pub locked_rg_debug_hook: Option<GraphDebugHook>,
    pub grab_cursor_pos: winit::dpi::PhysicalPosition<f64>,

    pub reset_path_tracer: bool,

    pub selected_game_object: Option<GameObjectId>,

    pub active_camera_key: Option<usize>,
    sequence_playback_state: SequencePlaybackState,
    pub sequence_playback_speed: f32,

    runtime_scene: RuntimeScene,
}

enum SequencePlaybackState {
    NotPlaying,
    Playing {
        t: f32,
        sequence: CameraPlaybackSequence,
    },
}

impl RuntimeState {
    fn scene_sync_options(persisted: &PersistedState) -> SyncSceneOptions {
        SyncSceneOptions {
            global_emissive_multiplier: persisted.scene.render_settings.emissive_multiplier,
            emissive_enabled: persisted.scene.render_settings.enable_emissive,
        }
    }

    fn active_camera_transform(scene: &SceneState) -> SceneElementTransform {
        scene
            .with_primary_camera(|transform, _| transform.clone())
            .unwrap_or_else(|| {
                let camera = CameraState::default();
                SceneElementTransform::from_position_rotation_scale(
                    camera.position,
                    camera.rotation,
                    Vec3::ONE,
                )
            })
    }

    fn active_camera_vertical_fov(scene: &SceneState) -> f32 {
        scene
            .with_primary_camera(|_, camera| camera.vertical_fov)
            .unwrap_or_else(|| CameraState::default().vertical_fov)
    }

    fn active_camera_position_and_rotation(scene: &SceneState) -> (Vec3, Quat) {
        let transform = Self::active_camera_transform(scene);
        (transform.position, transform.rotation())
    }

    fn current_sun_direction(scene: &SceneState) -> Vec3 {
        scene
            .with_sun(|_, sun| sun.controller.towards_sun())
            .unwrap_or(Vec3::Y)
    }

    fn current_sun_size_multiplier(scene: &SceneState) -> f32 {
        scene.with_sun(|_, sun| sun.size_multiplier).unwrap_or(1.0)
    }

    fn sync_camera_rig_from_scene(&mut self, scene: &SceneState) {
        let (position, rotation) = Self::active_camera_position_and_rotation(scene);
        self.camera.driver_mut::<Position>().position = position;
        self.camera
            .driver_mut::<YawPitch>()
            .set_rotation_quat(rotation);
        self.camera.update(1e10);
    }

    fn write_camera_rig_to_scene(&self, scene: &mut SceneState) {
        let position = self.camera.final_transform.position;
        let rotation = self.camera.final_transform.rotation;

        let _ = scene.with_primary_camera_mut(|transform, _| {
            transform.position = position;
            transform.set_rotation(rotation);
        });
    }

    fn preload_game_object(
        &mut self,
        world_renderer: &mut WorldRenderer,
        game_object: &GameObject,
    ) -> anyhow::Result<()> {
        for component in &game_object.components {
            if let Some(mesh_renderer) = component.as_mesh_renderer() {
                self.runtime_scene
                    .load_mesh(world_renderer, &mesh_renderer.source)
                    .with_context(|| {
                        format!(
                            "Preloading MeshRenderer for GameObject {} ({})",
                            game_object.name, game_object.id.0
                        )
                    })?;
            }
        }

        Ok(())
    }

    pub fn new(
        persisted: &mut PersistedState,
        world_renderer: &mut WorldRenderer,
        opt: &Opt,
    ) -> Self {
        let (camera_position, camera_rotation) =
            Self::active_camera_position_and_rotation(&persisted.scene);
        let camera: CameraRig = CameraRig::builder()
            .with(Position::new(camera_position))
            .with(YawPitch::new().rotation_quat(camera_rotation))
            .with(Smooth::default())
            .build();

        // Mitsuba match
        /*let mut camera = camera::FirstPersonCamera::new(Vec3::new(-2.0, 4.0, 8.0));
        camera.fov = 35.0 * 9.0 / 16.0;
        camera.look_at(Vec3::new(0.0, 0.75, 0.0));*/

        let mouse: MouseState = Default::default();
        let keyboard: KeyboardState = Default::default();

        let keymap_config = KeymapConfig::load(&opt.keymap).unwrap_or_else(|err| {
            warn!("Failed to load keymap: {}", err);
            info!("Using default keymap");
            KeymapConfig::default()
        });

        let sun_direction_interp = Self::current_sun_direction(&persisted.scene);

        let mut res = Self {
            camera,
            mouse,
            keyboard,
            keymap_config: keymap_config.clone(),
            movement_map: keymap_config.movement.into(),

            show_gui: true,
            sun_direction_interp,
            left_click_edit_mode: LeftClickEditMode::MoveSun,

            max_fps: MAX_FPS_LIMIT,
            locked_rg_debug_hook: None,
            grab_cursor_pos: Default::default(),

            reset_path_tracer: false,
            selected_game_object: None,

            active_camera_key: None,
            sequence_playback_state: SequencePlaybackState::NotPlaying,
            sequence_playback_speed: 1.0,

            runtime_scene: RuntimeScene::default(),
        };

        // Preload persisted GameObjects so invalid mesh references do not crash startup.
        let mut valid_game_objects = Vec::with_capacity(persisted.scene.game_objects.len());
        for game_object in std::mem::take(&mut persisted.scene.game_objects) {
            match res.preload_game_object(world_renderer, &game_object) {
                Ok(()) => valid_game_objects.push(game_object),
                Err(err) => {
                    log::error!(
                        "Failed to preload GameObject {} ({}): {:#}",
                        game_object.name,
                        game_object.id.0,
                        err
                    );
                }
            }
        }
        persisted.scene.game_objects = valid_game_objects;

        if let Err(err) = res.runtime_scene.rebuild(
            &persisted.scene,
            world_renderer,
            Self::scene_sync_options(persisted),
        ) {
            log::error!("Failed to initialize scene bindings: {:#}", err);
        }

        // Load the IBL too
        if let Some(ibl) = persisted.scene.ibl.as_ref() {
            if world_renderer.ibl.load_image(ibl).is_err() {
                persisted.scene.ibl = None;
            }
        }

        res
    }

    pub fn load_scene(
        &mut self,
        persisted: &mut PersistedState,
        world_renderer: &mut WorldRenderer,
        scene_path: impl Into<PathBuf>,
    ) -> anyhow::Result<()> {
        let scene_path = scene_path.into();
        let scene_desc: SceneDesc = ron::de::from_reader(
            File::open(&scene_path)
                .with_context(|| format!("Opening scene file {:?}", scene_path))?,
        )?;

        let mut scene = SceneState::default();
        scene.copy_editor_singletons_from(&persisted.scene);
        scene.ibl = persisted.scene.ibl.clone();

        for instance in scene_desc.instances {
            let mesh_path = canonical_path_from_vfs(&instance.mesh)
                .with_context(|| format!("Mesh path: {:?}", instance.mesh))
                .expect("valid mesh path");

            let source = MeshSource::File(mesh_path.clone());

            self.runtime_scene
                .load_mesh(world_renderer, &source)
                .with_context(|| format!("Mesh path: {:?}", instance.mesh))
                .expect("valid mesh");

            let transform = SceneElementTransform {
                position: instance.position.into(),
                rotation_euler_degrees: instance.rotation.into(),
                scale: instance.scale.into(),
            };

            let object_name = unique_game_object_name(&scene, source.default_game_object_name());
            scene.create_mesh_object(object_name, source, transform);
        }

        self.runtime_scene.rebuild(
            &scene,
            world_renderer,
            Self::scene_sync_options(persisted),
        )?;
        persisted.scene = scene;
        self.selected_game_object = None;
        self.sync_camera_rig_from_scene(&persisted.scene);
        self.sun_direction_interp = Self::current_sun_direction(&persisted.scene);

        Ok(())
    }

    fn update_camera(&mut self, persisted: &mut PersistedState, ctx: &FrameContext) {
        self.sync_camera_rig_from_scene(&persisted.scene);

        let smooth = self.camera.driver_mut::<Smooth>();
        if ctx.world_renderer.render_mode == RenderMode::Reference {
            smooth.position_smoothness = 0.0;
            smooth.rotation_smoothness = 0.0;
        } else {
            smooth.position_smoothness = persisted.movement.camera_smoothness;
            smooth.rotation_smoothness = persisted.movement.camera_smoothness;
        }

        // When starting camera rotation, hide the mouse cursor, and capture it to the window.
        if (self.mouse.buttons_pressed & (1 << 2)) != 0 {
            let _ = ctx.window.set_cursor_grab(true);
            self.grab_cursor_pos = self.mouse.physical_position;
            ctx.window.set_cursor_visible(false);
        }

        // When ending camera rotation, release the cursor.
        if (self.mouse.buttons_released & (1 << 2)) != 0 {
            let _ = ctx.window.set_cursor_grab(false);
            ctx.window.set_cursor_visible(true);
        }

        let input = self.movement_map.map(&self.keyboard, ctx.dt_filtered);
        let move_vec = self.camera.final_transform.rotation
            * Vec3::new(input["move_right"], input["move_up"], -input["move_fwd"])
                .clamp_length_max(1.0)
            * 4.0f32.powf(input["boost"]);

        if (self.mouse.buttons_held & (1 << 2)) != 0 {
            // While we're rotating, the cursor should not move, so that upon revealing it,
            // it will be where we started the rotation motion at.
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

    fn update_sun(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.mouse.buttons_held & 1 != 0 {
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
                } /*LeftClickEditMode::MoveLocalLights => {
                      persisted.light.lights.theta += theta_delta;
                      persisted.light.lights.phi += phi_delta;
                  }*/
            }
        }

        //state.sun.phi += dt;
        //state.sun.phi %= std::f32::consts::TAU;

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

    fn update_lights(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.keyboard.was_just_pressed(
            self.keymap_config
                .rendering
                .switch_to_reference_path_tracing,
        ) {
            match ctx.world_renderer.render_mode {
                RenderMode::Standard => {
                    //camera.convergence_sensitivity = 1.0;
                    ctx.world_renderer.render_mode = RenderMode::Reference;
                }
                RenderMode::Reference => {
                    //camera.convergence_sensitivity = 0.0;
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

        /*if self.keyboard.is_down(VirtualKeyCode::Z) {
            persisted.light.local_lights.distance /= 0.99;
        }
        if self.keyboard.is_down(VirtualKeyCode::X) {
            persisted.light.local_lights.distance *= 0.99;
        }*/

        /*#[allow(clippy::comparison_chain)]
        if light_instances.len() > state.lights.count as usize {
            for extra_light in light_instances.drain(state.lights.count as usize..) {
                ctx.world_renderer.remove_instance(extra_light);
            }
        } else if light_instances.len() < state.lights.count as usize {
            light_instances.extend(
                (0..(state.lights.count as usize - light_instances.len())).map(|_| {
                    ctx.world_renderer
                        .add_instance(light_mesh, Vec3::ZERO, Quat::IDENTITY)
                }),
            );
        }

        for (i, inst) in light_instances.iter().enumerate() {
            let ring_rot = Quat::from_rotation_y(
                (i as f32) / light_instances.len() as f32 * std::f32::consts::TAU,
            );

            let rot =
                Quat::from_euler(EulerRot::YXZ, -state.lights.theta, -state.lights.phi, 0.0)
                    * ring_rot;
            ctx.world_renderer.set_instance_transform(
                *inst,
                rot * (Vec3::Z * state.lights.distance) + Vec3::new(0.1, 1.2, 0.0),
                rot,
            );

            ctx.world_renderer
                .get_instance_dynamic_parameters_mut(*inst)
                .emissive_multiplier = state.lights.multiplier;
        }*/
    }

    fn update_objects(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if let Err(err) = self.runtime_scene.sync(
            &persisted.scene,
            ctx.world_renderer,
            Self::scene_sync_options(persisted),
        ) {
            log::error!("Failed to sync scene bindings: {:#}", err);
        }
    }

    pub fn frame(
        &mut self,
        mut ctx: FrameContext,
        persisted: &mut PersistedState,
    ) -> WorldFrameDesc {
        // Limit framerate. Not particularly precise.
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

        // Reset accumulation of the path tracer whenever the camera moves
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
        // Allow some time at the start of the playback before the camera starts moving
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

    pub(crate) fn add_mesh_instance(
        &mut self,
        persisted: &mut PersistedState,
        world_renderer: &mut WorldRenderer,
        source: MeshSource,
        transform: SceneElementTransform,
    ) -> anyhow::Result<()> {
        self.runtime_scene.load_mesh(world_renderer, &source)?;

        let object_name = unique_game_object_name(&persisted.scene, source.default_game_object_name());
        let game_object_id = persisted
            .scene
            .create_mesh_object(object_name, source, transform);
        self.selected_game_object = Some(game_object_id);

        self.runtime_scene.sync(
            &persisted.scene,
            world_renderer,
            Self::scene_sync_options(persisted),
        )?;

        Ok(())
    }

    fn handle_file_drop_events(
        &mut self,
        persisted: &mut PersistedState,
        world_renderer: &mut WorldRenderer,
        events: &[winit::event::Event<()>],
    ) {
        for event in events {
            match event {
                winit::event::Event::WindowEvent {
                    window_id: _,
                    event: WindowEvent::DroppedFile(path),
                } => {
                    let extension = path
                        .extension()
                        .map_or("".to_string(), |ext| ext.to_string_lossy().into_owned());

                    match extension.as_str() {
                        "hdr" | "exr" => {
                            // IBL
                            match world_renderer.ibl.load_image(path) {
                                Ok(_) => {
                                    persisted.scene.ibl = Some(path.clone());
                                }
                                Err(err) => {
                                    log::error!("{:#}", err);
                                }
                            }
                        }
                        "ron" => {
                            // Scene
                            if let Err(err) = self.load_scene(persisted, world_renderer, path) {
                                log::error!("Failed to load scene: {:#}", err);
                            }
                        }
                        "gltf" | "glb" => {
                            // Mesh
                            if let Err(err) = self.add_mesh_instance(
                                persisted,
                                world_renderer,
                                MeshSource::File(path.clone()),
                                SceneElementTransform::IDENTITY,
                            ) {
                                log::error!("{:#}", err);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum LeftClickEditMode {
    MoveSun,
    //MoveLocalLights,
}
