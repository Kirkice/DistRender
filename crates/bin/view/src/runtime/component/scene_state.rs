use std::path::PathBuf;

use dist_render_simple::Affine3A;

use super::{
    default_main_camera_transform, BuiltinGameObjectKind, CameraComponent, GameObject,
    GameObjectId, LocalLightsComponent, MeshRendererComponent, MeshSource, SceneComponent,
    SceneElementTransform, SceneRenderSettings, SunComponent, DEFAULT_CAMERA_VERTICAL_FOV,
};

fn default_next_game_object_id() -> u64 {
    1
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(from = "SceneStateSerde")]
pub struct SceneState {
    #[serde(default = "default_next_game_object_id")]
    pub next_game_object_id: u64,

    #[serde(default)]
    pub game_objects: Vec<GameObject>,

    #[serde(default)]
    pub render_settings: SceneRenderSettings,

    #[serde(default)]
    pub ibl: Option<PathBuf>,
}

impl Default for SceneState {
    fn default() -> Self {
        let mut scene = Self {
            next_game_object_id: default_next_game_object_id(),
            game_objects: Vec::new(),
            render_settings: SceneRenderSettings {
                emissive_multiplier: 1.0,
                enable_emissive: true,
            },
            ibl: None,
        };
        scene.ensure_editor_singletons();
        scene
    }
}

impl SceneState {
    pub fn first_component_index(
        &self,
        predicate: impl Fn(&SceneComponent) -> bool,
    ) -> Option<(usize, usize)> {
        for (game_object_index, game_object) in self.game_objects.iter().enumerate() {
            for (component_index, component) in game_object.components.iter().enumerate() {
                if predicate(component) {
                    return Some((game_object_index, component_index));
                }
            }
        }

        None
    }

    pub fn primary_camera_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::Camera(camera) if camera.enabled && camera.primary)
        })
        .or_else(|| {
            self.first_component_index(|component| {
                matches!(component, SceneComponent::Camera(camera) if camera.enabled)
            })
        })
        .or_else(|| self.first_component_index(|component| matches!(component, SceneComponent::Camera(_))))
    }

    pub fn sun_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::Sun(sun) if sun.enabled)
        })
        .or_else(|| self.first_component_index(|component| matches!(component, SceneComponent::Sun(_))))
    }

    pub fn local_lights_component_index(&self) -> Option<(usize, usize)> {
        self.first_component_index(|component| {
            matches!(component, SceneComponent::LocalLights(local_lights) if local_lights.enabled)
        })
        .or_else(|| {
            self.first_component_index(|component| matches!(component, SceneComponent::LocalLights(_)))
        })
    }

    pub fn with_primary_camera<R>(
        &self,
        f: impl FnOnce(&SceneElementTransform, &CameraComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.primary_camera_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_camera()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_primary_camera_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut CameraComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.primary_camera_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object.components.get_mut(component_index)?.as_camera_mut()?;
        Some(f(transform, component))
    }

    pub fn with_sun<R>(&self, f: impl FnOnce(&SceneElementTransform, &SunComponent) -> R) -> Option<R> {
        let (game_object_index, component_index) = self.sun_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_sun()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_sun_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut SunComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.sun_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object.components.get_mut(component_index)?.as_sun_mut()?;
        Some(f(transform, component))
    }

    pub fn with_local_lights<R>(
        &self,
        f: impl FnOnce(&SceneElementTransform, &LocalLightsComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.local_lights_component_index()?;
        let game_object = self.game_objects.get(game_object_index)?;
        let component = game_object.components.get(component_index)?.as_local_lights()?;
        Some(f(&game_object.transform, component))
    }

    pub fn with_local_lights_mut<R>(
        &mut self,
        f: impl FnOnce(&mut SceneElementTransform, &mut LocalLightsComponent) -> R,
    ) -> Option<R> {
        let (game_object_index, component_index) = self.local_lights_component_index()?;
        let game_object = self.game_objects.get_mut(game_object_index)?;
        let transform = &mut game_object.transform;
        let component = game_object
            .components
            .get_mut(component_index)?
            .as_local_lights_mut()?;
        Some(f(transform, component))
    }

    pub fn allocate_game_object_id(&mut self) -> GameObjectId {
        let id = GameObjectId(self.next_game_object_id.max(default_next_game_object_id()));
        self.next_game_object_id = id.0.saturating_add(1);
        id
    }

    pub fn create_game_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
    ) -> GameObjectId {
        let id = self.allocate_game_object_id();
        self.game_objects.push(GameObject::new(id, name, transform));
        id
    }

    pub fn create_mesh_object(
        &mut self,
        name: impl Into<String>,
        source: MeshSource,
        transform: SceneElementTransform,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        if let Some(game_object) = self.find_game_object_mut(id) {
            game_object
                .components
                .push(SceneComponent::MeshRenderer(MeshRendererComponent::new(source)));
        }
        id
    }

    pub fn add_component(&mut self, game_object_id: GameObjectId, component: SceneComponent) -> bool {
        if let Some(game_object) = self.find_game_object_mut(game_object_id) {
            game_object.components.push(component);
            true
        } else {
            false
        }
    }

    pub fn create_camera_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: CameraComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::Camera(component));
        id
    }

    pub fn create_sun_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: SunComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::Sun(component));
        id
    }

    pub fn create_local_lights_object(
        &mut self,
        name: impl Into<String>,
        transform: SceneElementTransform,
        component: LocalLightsComponent,
    ) -> GameObjectId {
        let id = self.create_game_object(name, transform);
        self.add_component(id, SceneComponent::LocalLights(component));
        id
    }

    pub fn find_game_object(&self, id: GameObjectId) -> Option<&GameObject> {
        self.game_objects.iter().find(|game_object| game_object.id == id)
    }

    pub fn find_game_object_mut(&mut self, id: GameObjectId) -> Option<&mut GameObject> {
        self.game_objects
            .iter_mut()
            .find(|game_object| game_object.id == id)
    }

    fn is_descendant_of_in(
        game_objects: &[GameObject],
        descendant: GameObjectId,
        ancestor: GameObjectId,
    ) -> bool {
        let mut current = game_objects
            .iter()
            .find(|game_object| game_object.id == descendant)
            .and_then(|game_object| game_object.parent);
        let mut visited = Vec::new();

        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }

            if visited.contains(&parent) {
                break;
            }

            visited.push(parent);
            current = game_objects
                .iter()
                .find(|game_object| game_object.id == parent)
                .and_then(|game_object| game_object.parent);
        }

        false
    }

    pub fn can_reparent_game_object(
        &self,
        child_id: GameObjectId,
        new_parent: Option<GameObjectId>,
    ) -> bool {
        let Some(child) = self.find_game_object(child_id) else {
            return false;
        };

        if child.parent == new_parent {
            return false;
        }

        if child.is_builtin() && new_parent.is_some() {
            return false;
        }

        let Some(new_parent) = new_parent else {
            return true;
        };

        if new_parent == child_id || self.find_game_object(new_parent).is_none() {
            return false;
        }

        !Self::is_descendant_of_in(&self.game_objects, new_parent, child_id)
    }

    pub fn reparent_game_object(
        &mut self,
        child_id: GameObjectId,
        new_parent: Option<GameObjectId>,
    ) -> bool {
        if !self.can_reparent_game_object(child_id, new_parent) {
            return false;
        }

        let mut subtree_ids = Vec::new();
        self.collect_subtree_ids(child_id, &mut subtree_ids);
        if subtree_ids.is_empty() {
            return false;
        }

        let mut subtree = Vec::new();
        let mut remaining = Vec::with_capacity(self.game_objects.len().saturating_sub(subtree_ids.len()));

        for game_object in std::mem::take(&mut self.game_objects) {
            if subtree_ids.contains(&game_object.id) {
                subtree.push(game_object);
            } else {
                remaining.push(game_object);
            }
        }

        let Some(child) = subtree.iter_mut().find(|game_object| game_object.id == child_id) else {
            self.game_objects = remaining;
            return false;
        };

        child.parent = new_parent;

        let insert_index = if let Some(new_parent_id) = new_parent {
            remaining
                .iter()
                .enumerate()
                .rev()
                .find(|(_, game_object)| {
                    game_object.id == new_parent_id
                        || Self::is_descendant_of_in(&remaining, game_object.id, new_parent_id)
                })
                .map(|(index, _)| index + 1)
                .unwrap_or(remaining.len())
        } else {
            remaining.len()
        };

        let mut tail = remaining.split_off(insert_index);
        remaining.extend(subtree);
        remaining.append(&mut tail);
        self.game_objects = remaining;
        true
    }

    pub fn refresh_editor_singletons(&mut self) {
        self.ensure_editor_singletons();
    }

    pub fn remove_game_object(&mut self, id: GameObjectId) -> bool {
        let mut subtree = Vec::new();
        self.collect_subtree_ids(id, &mut subtree);
        if subtree.is_empty() {
            return false;
        }

        self.game_objects
            .retain(|game_object| !subtree.contains(&game_object.id));
        self.ensure_editor_singletons();
        true
    }

    pub fn copy_editor_singletons_from(&mut self, other: &SceneState) {
        self.render_settings = other.render_settings.clone();

        if let Some((transform, camera)) = other.with_primary_camera(|transform, camera| {
            (transform.clone(), camera.clone())
        }) {
            let _ = self.with_primary_camera_mut(|dest_transform, dest_camera| {
                *dest_transform = transform;
                *dest_camera = camera;
            });
        }

        if let Some((transform, sun)) = other.with_sun(|transform, sun| (transform.clone(), sun.clone()))
        {
            let _ = self.with_sun_mut(|dest_transform, dest_sun| {
                *dest_transform = transform;
                *dest_sun = sun;
            });
        }

        if let Some((transform, local_lights)) =
            other.with_local_lights(|transform, local_lights| {
                (transform.clone(), local_lights.clone())
            })
        {
            let _ = self.with_local_lights_mut(|dest_transform, dest_local_lights| {
                *dest_transform = transform;
                *dest_local_lights = local_lights;
            });
        }
    }

    pub fn world_transform(&self, id: GameObjectId) -> Affine3A {
        let mut visited = Vec::new();
        self.world_transform_inner(id, &mut visited)
    }

    fn collect_subtree_ids(&self, id: GameObjectId, subtree: &mut Vec<GameObjectId>) {
        if subtree.contains(&id) || self.find_game_object(id).is_none() {
            return;
        }

        subtree.push(id);

        let child_ids: Vec<_> = self
            .game_objects
            .iter()
            .filter(|game_object| game_object.parent == Some(id))
            .map(|game_object| game_object.id)
            .collect();

        for child_id in child_ids {
            self.collect_subtree_ids(child_id, subtree);
        }
    }

    fn normalize(&mut self) {
        let mut seen_ids = Vec::new();
        let mut next_game_object_id = self.next_game_object_id.max(default_next_game_object_id());

        for game_object in &mut self.game_objects {
            if game_object.id.0 == 0 || seen_ids.contains(&game_object.id) {
                game_object.id = GameObjectId(next_game_object_id);
                next_game_object_id = next_game_object_id.saturating_add(1);
            }

            seen_ids.push(game_object.id);
            next_game_object_id = next_game_object_id.max(game_object.id.0.saturating_add(1));

            game_object.name = GameObject::sanitized_name(game_object.id, &game_object.name);
        }

        for game_object in &mut self.game_objects {
            if let Some(parent) = game_object.parent {
                if !seen_ids.contains(&parent) || parent == game_object.id || game_object.is_builtin() {
                    game_object.parent = None;
                }
            }
        }

        self.next_game_object_id = next_game_object_id.max(default_next_game_object_id());
        self.ensure_editor_singletons();
    }

    fn ensure_editor_singletons(&mut self) {
        if self.primary_camera_component_index().is_none() {
            self.create_camera_object(
                "Main Camera",
                default_main_camera_transform(),
                CameraComponent {
                    vertical_fov: DEFAULT_CAMERA_VERTICAL_FOV,
                    ..Default::default()
                },
            );
        }

        if self.sun_component_index().is_none() {
            self.create_sun_object("Sun", SceneElementTransform::IDENTITY, SunComponent::default());
        }

        if self.local_lights_component_index().is_none() {
            self.create_local_lights_object(
                "Local Lights",
                SceneElementTransform::IDENTITY,
                LocalLightsComponent::default(),
            );
        }

        for game_object in &mut self.game_objects {
            game_object.builtin = None;
        }

        if let Some((game_object_index, _)) = self.primary_camera_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::MainCamera);
                game_object.name = BuiltinGameObjectKind::MainCamera.display_name().to_owned();
            }
        }

        if let Some((game_object_index, _)) = self.sun_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::Sun);
                game_object.name = BuiltinGameObjectKind::Sun.display_name().to_owned();
            }
        }

        if let Some((game_object_index, _)) = self.local_lights_component_index() {
            if let Some(game_object) = self.game_objects.get_mut(game_object_index) {
                game_object.builtin = Some(BuiltinGameObjectKind::LocalLights);
                game_object.name = BuiltinGameObjectKind::LocalLights.display_name().to_owned();
            }
        }
    }

    fn world_transform_inner(&self, id: GameObjectId, visited: &mut Vec<GameObjectId>) -> Affine3A {
        let Some(game_object) = self.find_game_object(id) else {
            return Affine3A::IDENTITY;
        };

        let local_transform = game_object.transform.affine_transform();
        if visited.contains(&id) {
            return local_transform;
        }

        let parent_transform = if let Some(parent) = game_object.parent {
            visited.push(id);
            let transform = self.world_transform_inner(parent, visited);
            visited.pop();
            transform
        } else {
            Affine3A::IDENTITY
        };

        parent_transform * local_transform
    }
}

#[derive(serde::Deserialize)]
struct SceneStateSerde {
    #[serde(default = "default_next_game_object_id")]
    next_game_object_id: u64,

    #[serde(default)]
    game_objects: Vec<GameObject>,

    #[serde(default)]
    render_settings: SceneRenderSettings,

    #[serde(default)]
    ibl: Option<PathBuf>,

    #[serde(default)]
    elements: Vec<LegacySceneElement>,
}

#[derive(serde::Deserialize)]
struct LegacySceneElement {
    pub source: MeshSource,

    #[serde(default)]
    pub transform: SceneElementTransform,
}

impl From<SceneStateSerde> for SceneState {
    fn from(value: SceneStateSerde) -> Self {
        let mut scene = if value.game_objects.is_empty() && !value.elements.is_empty() {
            let mut scene = SceneState {
                next_game_object_id: value.next_game_object_id,
                game_objects: Vec::new(),
                render_settings: value.render_settings,
                ibl: value.ibl,
            };

            for element in value.elements {
                let name = element.source.default_game_object_name();
                scene.create_mesh_object(name, element.source, element.transform);
            }

            scene
        } else {
            SceneState {
                next_game_object_id: value.next_game_object_id,
                game_objects: value.game_objects,
                render_settings: value.render_settings,
                ibl: value.ibl,
            }
        };

        scene.normalize();
        scene
    }
}