use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use super::super::{materials::SyncSceneOptions, *};

#[derive(Default)]
pub struct RuntimeScene {
    known_meshes: HashMap<PathBuf, MeshHandle>,
    render_instances: HashMap<(GameObjectId, usize), InstanceHandle>,
}

impl RuntimeScene {
    pub fn clear(&mut self, world_renderer: &mut WorldRenderer) {
        for instance in self.render_instances.drain().map(|(_, instance)| instance) {
            world_renderer.remove_instance(instance);
        }
    }

    pub fn rebuild(
        &mut self,
        scene: &SceneState,
        world_renderer: &mut WorldRenderer,
        options: SyncSceneOptions,
    ) -> anyhow::Result<()> {
        self.clear(world_renderer);
        self.sync(scene, world_renderer, options)
    }

    pub fn sync(
        &mut self,
        scene: &SceneState,
        world_renderer: &mut WorldRenderer,
        options: SyncSceneOptions,
    ) -> anyhow::Result<()> {
        let mut desired_bindings = HashSet::new();

        for game_object in &scene.game_objects {
            if !game_object.enabled {
                continue;
            }

            let world_transform = scene.world_transform(game_object.id);

            for (component_index, component) in game_object.components.iter().enumerate() {
                let Some(mesh_renderer) = component.as_mesh_renderer() else {
                    continue;
                };

                if !mesh_renderer.enabled {
                    continue;
                }

                let binding_key = (game_object.id, component_index);
                desired_bindings.insert(binding_key);

                let instance = if let Some(instance) = self.render_instances.get(&binding_key) {
                    *instance
                } else {
                    let mesh = self
                        .load_mesh(world_renderer, &mesh_renderer.source)
                        .with_context(|| {
                            format!(
                                "Loading MeshRenderer for GameObject {} ({})",
                                game_object.name, game_object.id.0
                            )
                        })?;

                    let instance = world_renderer.add_instance(mesh, world_transform);
                    self.render_instances.insert(binding_key, instance);
                    instance
                };

                world_renderer.set_instance_transform(instance, world_transform);
                let dynamic_parameters = world_renderer.get_instance_dynamic_parameters_mut(instance);
                dynamic_parameters.base_color_tint = mesh_renderer.base_color_tint;
                dynamic_parameters.roughness_multiplier = mesh_renderer.roughness_multiplier;
                dynamic_parameters.metalness_multiplier = mesh_renderer.metalness_multiplier;
                dynamic_parameters.emissive_multiplier = if options.emissive_enabled {
                    options.global_emissive_multiplier * mesh_renderer.emissive_multiplier
                } else {
                    0.0
                };
            }
        }

        let stale_bindings: Vec<_> = self
            .render_instances
            .keys()
            .copied()
            .filter(|binding_key| !desired_bindings.contains(binding_key))
            .collect();

        for binding_key in stale_bindings {
            if let Some(instance) = self.render_instances.remove(&binding_key) {
                world_renderer.remove_instance(instance);
            }
        }

        Ok(())
    }

    pub fn load_mesh(
        &mut self,
        world_renderer: &mut WorldRenderer,
        source: &MeshSource,
    ) -> anyhow::Result<MeshHandle> {
        log::info!("Loading a mesh from {:?}", source);

        let path = match source {
            MeshSource::File(path) => {
                fn calculate_hash(path: &PathBuf) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    path.hash(&mut hasher);
                    hasher.finish()
                }

                let path_hash = match path.canonicalize() {
                    Ok(canonical) => calculate_hash(&canonical),
                    Err(_) => calculate_hash(path),
                };

                let cached_mesh_name = format!("{:8.8x}", path_hash);
                let cached_mesh_path = PathBuf::from(format!("/cache/{}.mesh", cached_mesh_name));

                if !canonical_path_from_vfs(&cached_mesh_path).map_or(false, |path| path.exists()) {
                    dist_render_asset_pipe::process_mesh_asset(
                        dist_render_asset_pipe::MeshAssetProcessParams {
                            path: path.clone(),
                            output_name: cached_mesh_name,
                            scale: 1.0,
                        },
                    )?;
                }

                cached_mesh_path
            }
            MeshSource::Cache(path) => path.clone(),
        };

        Ok(*self.known_meshes.entry(path.clone()).or_insert_with(|| {
            world_renderer
                .add_baked_mesh(path, AddMeshOptions::new())
                .unwrap()
        }))
    }
}