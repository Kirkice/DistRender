use std::{
    fs::File,
    path::{Path, PathBuf},
};

use dist_render_simple::*;

use crate::{
    opt::Opt,
    persisted::PersistedState,
    runtime::{component::{MeshSource, SceneElementTransform, SceneState}, RuntimeState},
};

const APP_STATE_CONFIG_FILE_PATH: &str = "renderer_state.ron";
const LEGACY_APP_STATE_CONFIG_FILE_PATH: &str = "view_state.ron";

struct AppState {
    persisted: PersistedState,
    runtime: RuntimeState,
    dist_render: SimpleMainLoop,
}

impl AppState {
    fn new(mut persisted: PersistedState, opt: &Opt) -> anyhow::Result<Self> {
        let mut dist_render = SimpleMainLoop::builder()
            .resolution([opt.width, opt.height])
            .vsync(!opt.no_vsync)
            .graphics_debugging(opt.graphics_debugging)
            .physical_device_index(opt.physical_device_index)
            .temporal_upsampling(opt.temporal_upsampling)
            .default_log_level(log::LevelFilter::Info)
            .fullscreen(opt.fullscreen.then_some(FullscreenMode::Exclusive))
            .build(
                WindowBuilder::new()
                    .with_title("dist render")
                    .with_resizable(true)
                    .with_decorations(!opt.no_window_decorations),
            )?;

        let runtime = RuntimeState::new(&mut persisted, &mut dist_render.world_renderer, opt);

        Ok(Self {
            persisted,
            runtime,
            dist_render,
        })
    }

    fn load_scene(&mut self, scene_path: &Path) -> anyhow::Result<()> {
        self.runtime.load_scene(
            &mut self.persisted,
            &mut self.dist_render.world_renderer,
            scene_path,
        )
    }

    fn add_standalone_mesh(&mut self, path: PathBuf, mesh_scale: f32) -> anyhow::Result<()> {
        self.runtime.add_mesh_instance(
            &mut self.persisted,
            &mut self.dist_render.world_renderer,
            MeshSource::File(path),
            SceneElementTransform {
                position: Vec3::ZERO,
                rotation_euler_degrees: Vec3::ZERO,
                scale: Vec3::splat(mesh_scale),
            },
        )
    }

    fn run(self) -> anyhow::Result<PersistedState> {
        let Self {
            mut persisted,
            mut runtime,
            dist_render,
        } = self;

        dist_render.run(|ctx| runtime.frame(ctx, &mut persisted))?;

        Ok(persisted)
    }
}

pub fn run(opt: Opt) -> anyhow::Result<()> {
    set_vfs_mount_point("/meshes", "assets/meshes");

    let mut persisted: PersistedState = File::open(APP_STATE_CONFIG_FILE_PATH)
        .or_else(|_| File::open(LEGACY_APP_STATE_CONFIG_FILE_PATH))
        .map_err(|err| anyhow::anyhow!(err))
        .and_then(|file| Ok(ron::de::from_reader(file)?))
        .unwrap_or_default();

    if opt.scene.is_some() || opt.mesh.is_some() {
        persisted.scene = SceneState::default();
    }

    let mut state = AppState::new(persisted, &opt)?;

    if let Some(scene) = opt.scene.as_ref() {
        state.load_scene(scene)?;
    } else if let Some(mesh) = opt.mesh.as_ref() {
        state.add_standalone_mesh(mesh.clone(), opt.mesh_scale)?;
    } else {
        state.load_scene(&PathBuf::from("assets/scenes/pica.ron"))?;
    }

    let state = state.run()?;

    ron::ser::to_writer_pretty(
        File::create(APP_STATE_CONFIG_FILE_PATH)?,
        &state,
        Default::default(),
    )?;

    Ok(())
}