use super::super::*;

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

impl RuntimeState {
    pub(in crate::runtime) fn preload_game_object(
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

    pub(in crate::runtime) fn handle_file_drop_events(
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
                            if let Err(err) = self.load_scene(persisted, world_renderer, path) {
                                log::error!("Failed to load scene: {:#}", err);
                            }
                        }
                        "gltf" | "glb" => {
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