#![allow(clippy::single_match)]

pub(crate) mod component;
mod core;
mod geometry;
mod materials;
mod render;
mod shaders;

use anyhow::Context;
use dolly::prelude::*;
use dist_render::{
    rg::GraphDebugHook,
    world_renderer::{AddMeshOptions, InstanceHandle, MeshHandle, WorldRenderer},
};
use dist_render_simple::*;
use log::{info, warn};
use std::{fs::File, path::PathBuf};

use self::{
    component::{GameObject, GameObjectId, MeshSource, SceneElementTransform, SceneState},
    geometry::RuntimeScene,
};

use crate::{
    keymap::KeymapConfig,
    opt::Opt,
    persisted::ShouldResetPathTracer as _,
    scene::SceneDesc,
    sequence::{CameraPlaybackSequence, MemOption, SequenceValue},
    PersistedState,
};

pub const MAX_FPS_LIMIT: u32 = 256;
pub const MIN_CAMERA_SPEED: f32 = 0.025;
pub const MAX_CAMERA_SPEED: f32 = 64.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewportGizmoAxis {
    X,
    Y,
    Z,
}

impl ViewportGizmoAxis {
    pub fn world_vector(self) -> Vec3 {
        match self {
            ViewportGizmoAxis::X => Vec3::X,
            ViewportGizmoAxis::Y => Vec3::Y,
            ViewportGizmoAxis::Z => Vec3::Z,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ViewportGizmoDragState {
    pub game_object_id: GameObjectId,
    pub axis: ViewportGizmoAxis,
    pub pointer_origin: Vec2,
    pub screen_direction: Vec2,
    pub start_world_position: Vec3,
    pub pixels_per_unit: f32,
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

    pub viewport_hovered: bool,
    pub viewport_pointer_captured: bool,
    pub viewport_keyboard_focused: bool,
    pub viewport_click_origin: Option<Vec2>,
    pub viewport_gizmo_drag: Option<ViewportGizmoDragState>,

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

            viewport_hovered: false,
            viewport_pointer_captured: false,
            viewport_keyboard_focused: false,
            viewport_click_origin: None,
            viewport_gizmo_drag: None,

            runtime_scene: RuntimeScene::default(),
        };

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

        if let Some(ibl) = persisted.scene.ibl.as_ref() {
            if world_renderer.ibl.load_image(ibl).is_err() {
                persisted.scene.ibl = None;
            }
        }

        res
    }
}

#[derive(PartialEq, Eq)]
pub enum LeftClickEditMode {
    MoveSun,
}
