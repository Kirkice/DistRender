use dist_render::RenderOverrideFlags;
use dist_render_simple::*;
use egui::{self, Color32};

use crate::{
    persisted::{
        CameraComponent, GameObject, GameObjectId, LocalLightsComponent, SceneComponent,
        SceneElementTransform, SceneState, SunComponent,
    },
    runtime::{RuntimeState, ViewportGizmoAxis, ViewportGizmoDragState, MAX_FPS_LIMIT},
    PersistedState,
};

struct ProjectedViewportObject {
    game_object_id: GameObjectId,
    world_position: Vec3,
    screen_position: egui::Pos2,
    depth: f32,
}

struct ProjectedGizmoAxis {
    axis: ViewportGizmoAxis,
    screen_origin: egui::Pos2,
    screen_end: egui::Pos2,
    screen_direction: Vec2,
    pixels_per_unit: f32,
}

impl RuntimeState {
    fn quantize_viewport_extent(size_in_pixels: f32) -> u32 {
        const VIEWPORT_GRANULARITY: u32 = 32;

        let size_in_pixels = size_in_pixels.round().max(VIEWPORT_GRANULARITY as f32) as u32;
        ((size_in_pixels + VIEWPORT_GRANULARITY - 1) / VIEWPORT_GRANULARITY) * VIEWPORT_GRANULARITY
    }

    fn chrome_fill() -> Color32 {
        Color32::from_rgb(14, 18, 26)
    }

    fn panel_fill() -> Color32 {
        Color32::from_rgb(16, 22, 31)
    }

    fn viewport_fill() -> Color32 {
        Color32::from_rgb(6, 9, 14)
    }

    fn warn_color() -> Color32 {
        Color32::from_rgb(255, 210, 96)
    }

    fn muted_color() -> Color32 {
        Color32::from_rgb(146, 158, 174)
    }

    fn validate_selected_game_object(&mut self, scene: &SceneState) {
        if let Some(selected_game_object) = self.selected_game_object {
            if scene.find_game_object(selected_game_object).is_none() {
                self.selected_game_object = None;
            }
        }

        if let Some(drag_state) = self.viewport_gizmo_drag {
            if scene.find_game_object(drag_state.game_object_id).is_none() {
                self.viewport_gizmo_drag = None;
            }
        }
    }

    fn viewport_camera_matrices(&self, persisted: &PersistedState, ctx: &FrameContext) -> (Mat4, Mat4, Vec3) {
        let lens = CameraLens {
            aspect_ratio: ctx.aspect_ratio(),
            vertical_fov: Self::active_camera_vertical_fov(&persisted.scene),
            ..Default::default()
        };
        let camera_matrices = self.camera.final_transform.into_position_rotation().through(&lens);

        (
            camera_matrices.view_to_clip * camera_matrices.world_to_view,
            camera_matrices.world_to_view,
            camera_matrices.eye_position(),
        )
    }

    fn project_world_point(
        viewport_rect: egui::Rect,
        world_to_clip: Mat4,
        world_to_view: Mat4,
        world_position: Vec3,
    ) -> Option<(egui::Pos2, f32)> {
        let view_position = world_to_view * world_position.extend(1.0);
        let depth = -view_position.z;
        if depth <= 0.0 {
            return None;
        }

        let clip_position = world_to_clip * world_position.extend(1.0);
        if clip_position.w <= 0.0 {
            return None;
        }

        let ndc = clip_position.truncate() / clip_position.w;
        let x = viewport_rect.left() + (ndc.x * 0.5 + 0.5) * viewport_rect.width();
        let y = viewport_rect.top() + (1.0 - (ndc.y * 0.5 + 0.5)) * viewport_rect.height();
        Some((egui::pos2(x, y), depth))
    }

    fn project_scene_objects(
        scene: &SceneState,
        viewport_rect: egui::Rect,
        world_to_clip: Mat4,
        world_to_view: Mat4,
    ) -> Vec<ProjectedViewportObject> {
        scene
            .game_objects
            .iter()
            .filter_map(|game_object| {
                let world_position = scene.world_transform(game_object.id).transform_point3(Vec3::ZERO);
                let (screen_position, depth) = Self::project_world_point(
                    viewport_rect,
                    world_to_clip,
                    world_to_view,
                    world_position,
                )?;

                Some(ProjectedViewportObject {
                    game_object_id: game_object.id,
                    world_position,
                    screen_position,
                    depth,
                })
            })
            .collect()
    }

    fn gizmo_axis_projections(
        viewport_rect: egui::Rect,
        world_to_clip: Mat4,
        world_to_view: Mat4,
        eye_position: Vec3,
        world_position: Vec3,
    ) -> Vec<ProjectedGizmoAxis> {
        let Some((screen_origin, _)) = Self::project_world_point(
            viewport_rect,
            world_to_clip,
            world_to_view,
            world_position,
        ) else {
            return Vec::new();
        };

        let handle_length_world = eye_position.distance(world_position).mul_add(0.12, 0.0).clamp(0.4, 4.0);

        [ViewportGizmoAxis::X, ViewportGizmoAxis::Y, ViewportGizmoAxis::Z]
            .into_iter()
            .filter_map(|axis| {
                let axis_world = axis.world_vector();
                let axis_end_world = world_position + axis_world * handle_length_world;
                let (screen_end, _) = Self::project_world_point(
                    viewport_rect,
                    world_to_clip,
                    world_to_view,
                    axis_end_world,
                )?;

                let delta = Vec2::new(screen_end.x - screen_origin.x, screen_end.y - screen_origin.y);
                let screen_length = delta.length();
                if screen_length < 6.0 {
                    return None;
                }

                Some(ProjectedGizmoAxis {
                    axis,
                    screen_origin,
                    screen_end,
                    screen_direction: delta / screen_length,
                    pixels_per_unit: screen_length / handle_length_world.max(1e-4),
                })
            })
            .collect()
    }

    fn screen_distance_to_segment(point: egui::Pos2, start: egui::Pos2, end: egui::Pos2) -> f32 {
        let point = Vec2::new(point.x, point.y);
        let start = Vec2::new(start.x, start.y);
        let end = Vec2::new(end.x, end.y);
        let segment = end - start;
        let segment_length_sq = segment.length_squared();
        if segment_length_sq <= 1e-5 {
            return (point - start).length();
        }

        let t = ((point - start).dot(segment) / segment_length_sq).clamp(0.0, 1.0);
        let closest = start + segment * t;
        (point - closest).length()
    }

    fn pick_game_object_at(
        projected_objects: &[ProjectedViewportObject],
        pointer_position: egui::Pos2,
    ) -> Option<GameObjectId> {
        const PICK_RADIUS: f32 = 14.0;

        projected_objects
            .iter()
            .filter_map(|projected| {
                let distance = projected.screen_position.distance(pointer_position);
                if distance <= PICK_RADIUS {
                    Some((projected.game_object_id, distance, projected.depth))
                } else {
                    None
                }
            })
            .min_by(|lhs, rhs| {
                lhs.1
                    .partial_cmp(&rhs.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| lhs.2.partial_cmp(&rhs.2).unwrap_or(std::cmp::Ordering::Equal))
            })
            .map(|(game_object_id, _, _)| game_object_id)
    }

    fn set_game_object_world_position(
        scene: &mut SceneState,
        game_object_id: GameObjectId,
        world_position: Vec3,
    ) {
        let parent = scene.find_game_object(game_object_id).and_then(|game_object| game_object.parent);

        let local_position = if let Some(parent_id) = parent {
            scene
                .world_transform(parent_id)
                .inverse()
                .transform_point3(world_position)
        } else {
            world_position
        };

        if let Some(game_object) = scene.find_game_object_mut(game_object_id) {
            game_object.transform.position = local_position;
        }
    }

    fn axis_color(axis: ViewportGizmoAxis) -> Color32 {
        match axis {
            ViewportGizmoAxis::X => Color32::from_rgb(235, 87, 87),
            ViewportGizmoAxis::Y => Color32::from_rgb(111, 207, 107),
            ViewportGizmoAxis::Z => Color32::from_rgb(86, 156, 255),
        }
    }

    fn game_object_has_scene_role_component(game_object: &GameObject) -> bool {
        game_object.components.iter().any(|component| {
            matches!(
                component,
                SceneComponent::Camera(_) | SceneComponent::Sun(_) | SceneComponent::LocalLights(_)
            )
        })
    }

    fn game_object_component_summary(game_object: &GameObject) -> String {
        game_object
            .components
            .iter()
            .map(|component| component.kind_name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn hierarchy_label(game_object: &GameObject, depth: usize, has_children: bool) -> String {
        let mut label = String::new();
        label.push_str(&"    ".repeat(depth));
        label.push_str(if has_children { "> " } else { "- " });

        if !game_object.enabled {
            label.push_str("[off] ");
        }

        label.push_str(&game_object.name);

        let component_summary = Self::game_object_component_summary(game_object);
        if !component_summary.is_empty() {
            label.push_str(" [");
            label.push_str(&component_summary);
            label.push(']');
        }

        label
    }

    fn draw_hierarchy_node(
        &mut self,
        ui: &mut egui::Ui,
        scene: &SceneState,
        game_object_id: GameObjectId,
        depth: usize,
    ) {
        let Some((label, child_ids, is_builtin)) = scene
            .find_game_object(game_object_id)
            .map(|game_object| {
                let child_ids: Vec<_> = scene
                    .game_objects
                    .iter()
                    .filter(|child| child.parent == Some(game_object_id))
                    .map(|child| child.id)
                    .collect();

                (
                    Self::hierarchy_label(game_object, depth, !child_ids.is_empty()),
                    child_ids,
                    game_object.is_builtin(),
                )
            })
        else {
            return;
        };

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 14.0);

            let selected = self.selected_game_object == Some(game_object_id);
            let response = ui.selectable_label(selected, label);
            if response.clicked() {
                self.selected_game_object = Some(game_object_id);
            }

            if is_builtin {
                ui.colored_label(Self::muted_color(), "builtin");
            }
        });

        for child_id in child_ids {
            self.draw_hierarchy_node(ui, scene, child_id, depth + 1);
        }
    }

    fn draw_hierarchy_panel(&mut self, ui: &mut egui::Ui, persisted: &mut PersistedState) {
        ui.heading("Hierarchy");
        ui.colored_label(
            Self::muted_color(),
            format!("{} game objects", persisted.scene.game_objects.len()),
        );
        ui.label("Reparenting lives in the Inspector Parent selector.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_game_object.is_none(), "Scene")
                .clicked()
            {
                self.selected_game_object = None;
            }

            if ui.button("Create Empty").clicked() {
                let name = format!("GameObject {}", persisted.scene.next_game_object_id);
                let game_object_id =
                    persisted.scene.create_game_object(name, SceneElementTransform::IDENTITY);
                self.selected_game_object = Some(game_object_id);
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let root_ids: Vec<_> = persisted
                .scene
                .game_objects
                .iter()
                .filter(|game_object| game_object.parent.is_none())
                .map(|game_object| game_object.id)
                .collect();

            for root_id in root_ids {
                self.draw_hierarchy_node(ui, &persisted.scene, root_id, 0);
            }
        });
    }

    fn drag_f32(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut f32,
        speed: f64,
        min: f32,
        max: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(
                egui::DragValue::new(value)
                    .speed(speed)
                    .clamp_range(min..=max),
            );
        });
    }

    fn drag_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, min: u32, max: u32) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).speed(1.0).clamp_range(min..=max));
        });
    }

    fn drag_triplet(
        ui: &mut egui::Ui,
        label: &str,
        values: [&mut f32; 3],
        speed: f64,
        min: f32,
        max: f32,
    ) {
        ui.label(label);
        ui.horizontal(|ui| {
            for (axis, value) in ["x", "y", "z"].into_iter().zip(values.into_iter()) {
                ui.add(
                    egui::DragValue::new(value)
                        .prefix(format!("{} ", axis))
                        .speed(speed)
                        .clamp_range(min..=max),
                );
            }
        });
    }

    fn draw_transform_inspector(ui: &mut egui::Ui, transform: &mut SceneElementTransform) {
        Self::drag_triplet(
            ui,
            "Position",
            [
                &mut transform.position.x,
                &mut transform.position.y,
                &mut transform.position.z,
            ],
            0.01,
            -10_000.0,
            10_000.0,
        );
        Self::drag_triplet(
            ui,
            "Rotation",
            [
                &mut transform.rotation_euler_degrees.x,
                &mut transform.rotation_euler_degrees.y,
                &mut transform.rotation_euler_degrees.z,
            ],
            0.1,
            -720.0,
            720.0,
        );
        Self::drag_triplet(
            ui,
            "Scale",
            [
                &mut transform.scale.x,
                &mut transform.scale.y,
                &mut transform.scale.z,
            ],
            0.05,
            0.001,
            1000.0,
        );
    }

    fn draw_component_inspector(
        &mut self,
        ui: &mut egui::Ui,
        component: &mut SceneComponent,
    ) {
        match component {
            SceneComponent::MeshRenderer(mesh_renderer) => {
                ui.checkbox(&mut mesh_renderer.enabled, "Enabled");
                ui.label(format!("Source: {:?}", mesh_renderer.source));
                Self::drag_f32(
                    ui,
                    "Emissive",
                    &mut mesh_renderer.emissive_multiplier,
                    0.05,
                    0.0,
                    10.0,
                );
            }
            SceneComponent::Camera(camera) => {
                ui.checkbox(&mut camera.enabled, "Enabled");
                ui.checkbox(&mut camera.primary, "Primary");
                Self::drag_f32(ui, "Vertical FOV", &mut camera.vertical_fov, 0.25, 1.0, 120.0);
            }
            SceneComponent::Sun(sun) => {
                ui.checkbox(&mut sun.enabled, "Enabled");
                Self::drag_f32(ui, "Sun size", &mut sun.size_multiplier, 0.02, 0.0, 10.0);
                ui.label(format!(
                    "Direction: {:.3}, {:.3}, {:.3}",
                    sun.controller.towards_sun().x,
                    sun.controller.towards_sun().y,
                    sun.controller.towards_sun().z,
                ));
            }
            SceneComponent::LocalLights(local_lights) => {
                ui.checkbox(&mut local_lights.enabled, "Enabled");
                Self::drag_u32(ui, "Count", &mut local_lights.count, 0, 64);
                Self::drag_f32(ui, "Theta", &mut local_lights.theta, 0.01, -10.0, 10.0);
                Self::drag_f32(ui, "Phi", &mut local_lights.phi, 0.01, -10.0, 10.0);
                Self::drag_f32(ui, "Distance", &mut local_lights.distance, 0.05, 0.0, 100.0);
                Self::drag_f32(ui, "Multiplier", &mut local_lights.multiplier, 0.5, 0.0, 1000.0);
            }
        }
    }

    fn draw_scene_inspector(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        ui.heading("Scene");
        ui.colored_label(
            Self::muted_color(),
            format!("{} game objects", persisted.scene.game_objects.len()),
        );

        egui::CollapsingHeader::new("Environment")
            .default_open(true)
            .show(ui, |ui| {
            if let Some(ibl) = persisted.scene.ibl.as_ref() {
                ui.label(format!("IBL: {:?}", ibl));
                if ui.button("Unload IBL").clicked() {
                    ctx.world_renderer.ibl.unload_image();
                    persisted.scene.ibl = None;
                }
            } else {
                ui.label("Drag a sphere-mapped .hdr/.exr to load as IBL.");
            }
        });

        egui::CollapsingHeader::new("Exposure")
            .default_open(true)
            .show(ui, |ui| {
            Self::drag_f32(ui, "EV shift", &mut persisted.exposure.ev_shift, 0.01, -8.0, 12.0);
            ui.checkbox(
                &mut persisted.exposure.use_dynamic_adaptation,
                "Use dynamic exposure",
            );
            Self::drag_f32(
                ui,
                "Adaptation speed",
                &mut persisted.exposure.dynamic_adaptation_speed,
                0.01,
                -4.0,
                4.0,
            );
            Self::drag_f32(
                ui,
                "Luminance histogram low clip",
                &mut persisted.exposure.dynamic_adaptation_low_clip,
                0.001,
                0.0,
                1.0,
            );
            persisted.exposure.dynamic_adaptation_low_clip = persisted
                .exposure
                .dynamic_adaptation_low_clip
                .clamp(0.0, 1.0);

            Self::drag_f32(
                ui,
                "Luminance histogram high clip",
                &mut persisted.exposure.dynamic_adaptation_high_clip,
                0.001,
                0.0,
                1.0,
            );
            persisted.exposure.dynamic_adaptation_high_clip = persisted
                .exposure
                .dynamic_adaptation_high_clip
                .clamp(0.0, 1.0);

            Self::drag_f32(ui, "Contrast", &mut persisted.exposure.contrast, 0.001, 1.0, 1.5);
        });

        egui::CollapsingHeader::new("Lighting")
            .default_open(true)
            .show(ui, |ui| {
            Self::drag_f32(
                ui,
                "Emissive multiplier",
                &mut persisted.scene.render_settings.emissive_multiplier,
                0.1,
                0.0,
                10.0,
            );
            ui.checkbox(
                &mut persisted.scene.render_settings.enable_emissive,
                "Enable emissive",
            );
            ui.colored_label(
                Self::muted_color(),
                "Select Main Camera, Sun, or Local Lights in Scene Graph for per-object controls.",
            );
        });

        egui::CollapsingHeader::new("Editor")
            .default_open(false)
            .show(ui, |ui| {
            Self::drag_f32(ui, "Camera speed", &mut persisted.movement.camera_speed, 0.025, 0.0, 10.0);
            Self::drag_f32(
                ui,
                "Camera smoothness",
                &mut persisted.movement.camera_smoothness,
                0.1,
                0.0,
                20.0,
            );
            Self::drag_f32(
                ui,
                "Sun rotation smoothness",
                &mut persisted.movement.sun_rotation_smoothness,
                0.1,
                0.0,
                20.0,
            );
        });

        ui.colored_label(
            Self::muted_color(),
            "Select a GameObject in Scene Graph to edit its Transform and Components.",
        );
    }

    fn draw_game_object_inspector(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        game_object_id: GameObjectId,
    ) {
        let mut delete_selected_game_object = false;
        let parent_label = persisted
            .scene
            .find_game_object(game_object_id)
            .and_then(|game_object| game_object.parent)
            .map(|parent_id| {
                persisted
                    .scene
                    .find_game_object(parent_id)
                    .map(|parent| format!("{} ({})", parent.name, parent.id.0))
                    .unwrap_or_else(|| format!("{}", parent_id.0))
            })
            .unwrap_or_else(|| "<root>".to_owned());
        let parent_candidates: Vec<_> = persisted
            .scene
            .game_objects
            .iter()
            .filter(|candidate| {
                candidate.id != game_object_id
                    && persisted
                        .scene
                        .can_reparent_game_object(game_object_id, Some(candidate.id))
            })
            .map(|candidate| (candidate.id, candidate.name.clone()))
            .collect();
        let mut component_to_add = None;
        let mut component_to_remove = None;
        let mut component_to_remove_name = None;
        let mut refresh_scene_singletons = false;
        let mut pending_reparent = None;

        if let Some(game_object) = persisted.scene.find_game_object_mut(game_object_id) {
            let mut name = game_object.name.clone();
            ui.label("Name");
            if ui.text_edit_singleline(&mut name).changed() {
                game_object.name = GameObject::sanitized_name(game_object.id, name.trim());
            }

            ui.label(format!("ID: {}", game_object.id.0));
            ui.checkbox(&mut game_object.enabled, "Enabled");

            let mut selected_parent = game_object.parent;
            egui::ComboBox::from_label("Parent")
                .selected_text(parent_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(selected_parent.is_none(), "Scene Root")
                        .clicked()
                    {
                        selected_parent = None;
                    }

                    for (candidate_id, candidate_name) in &parent_candidates {
                        if ui
                            .selectable_label(selected_parent == Some(*candidate_id), candidate_name)
                            .clicked()
                        {
                            selected_parent = Some(*candidate_id);
                        }
                    }
                });
            if selected_parent != game_object.parent {
                pending_reparent = Some(selected_parent);
            }

            if let Some(builtin) = game_object.builtin {
                ui.colored_label(
                    Self::muted_color(),
                    format!("Built-in role: {}", builtin.display_name()),
                );
            } else if ui.button("Delete GameObject").clicked() {
                delete_selected_game_object = true;
            }

            egui::CollapsingHeader::new("Transform")
                .default_open(true)
                .show(ui, |ui| {
                Self::draw_transform_inspector(ui, &mut game_object.transform);
            });

            if game_object.components.is_empty() {
                ui.colored_label(Self::muted_color(), "No Components");
            }

            egui::CollapsingHeader::new("Add Component")
                .default_open(true)
                .show(ui, |ui| {
                if Self::game_object_has_scene_role_component(game_object) {
                    ui.colored_label(
                        Self::muted_color(),
                        "This GameObject already owns a Camera, Sun, or Local Lights component.",
                    );
                } else {
                    if ui.button("Add Camera").clicked() {
                        component_to_add = Some(SceneComponent::Camera(CameraComponent::default()));
                    }

                    if ui.button("Add Sun").clicked() {
                        component_to_add = Some(SceneComponent::Sun(SunComponent::default()));
                    }

                    if ui.button("Add Local Lights").clicked() {
                        component_to_add =
                            Some(SceneComponent::LocalLights(LocalLightsComponent::default()));
                    }
                }

                ui.colored_label(
                    Self::muted_color(),
                    "MeshRenderer creation still comes from importing or dragging mesh assets.",
                );
            });

            for (component_index, component) in game_object.components.iter_mut().enumerate() {
                egui::CollapsingHeader::new(component.kind_name())
                    .id_source((game_object.id.0, component_index))
                    .default_open(true)
                    .show(ui, |ui| {
                    let is_builtin_component = game_object
                        .builtin
                        .map(|builtin| builtin.matches_component(component))
                        .unwrap_or(false);

                    if is_builtin_component {
                        ui.colored_label(Self::muted_color(), "Protected scene singleton component");
                    } else if ui.small_button("Remove").clicked() {
                        component_to_remove = Some(component_index);
                        component_to_remove_name = Some(component.kind_name().to_owned());
                    }

                    self.draw_component_inspector(ui, component);

                    if matches!(
                        component,
                        SceneComponent::Camera(_) | SceneComponent::Sun(_) | SceneComponent::LocalLights(_)
                    ) {
                        refresh_scene_singletons = true;
                    }
                });
            }
        } else {
            self.selected_game_object = None;
        }

        if let Some(new_parent) = pending_reparent {
            let _ = persisted.scene.reparent_game_object(game_object_id, new_parent);
        }

        if let Some(component) = component_to_add {
            let _ = persisted.scene.add_component(game_object_id, component);
            refresh_scene_singletons = true;
        }

        if let Some(component_index) = component_to_remove {
            if let Some(game_object) = persisted.scene.find_game_object_mut(game_object_id) {
                if component_index < game_object.components.len() {
                    game_object.components.remove(component_index);
                    refresh_scene_singletons = true;
                    if let Some(component_name) = component_to_remove_name.take() {
                        ui.colored_label(
                            Self::warn_color(),
                            format!("Removed {} component.", component_name),
                        );
                    }
                }
            }
        }

        if delete_selected_game_object {
            persisted.scene.remove_game_object(game_object_id);
            self.selected_game_object = None;
        }

        if refresh_scene_singletons {
            persisted.scene.refresh_editor_singletons();
        }
    }

    fn draw_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Inspector");

            match self.selected_game_object {
                Some(selected_game_object) => {
                    self.draw_game_object_inspector(ui, persisted, selected_game_object);
                }
                None => self.draw_scene_inspector(ui, persisted, ctx),
            }

            ui.add_space(10.0);
            ui.separator();

            egui::CollapsingHeader::new("Renderer")
                .default_open(true)
                .show(ui, |ui| {
                    self.draw_renderer_section(ui, persisted, ctx);
                });

            egui::CollapsingHeader::new("Sequence")
                .default_open(false)
                .show(ui, |ui| {
                    self.draw_sequence_section(ui, persisted);
                });
        });
    }

    fn draw_renderer_section(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        ui.horizontal(|ui| {
            ui.colored_label(
                Self::muted_color(),
                format!("CPU {:.2} ms", ctx.dt_filtered * 1000.0),
            );

            if let Some(report) = gpu_profiler::profiler().last_report() {
                let gpu_time_ms: f64 = report.scopes.iter().map(|scope| scope.duration.ms()).sum();
                ui.colored_label(Self::muted_color(), format!("GPU {:.2} ms", gpu_time_ms));
            }

            ui.colored_label(
                Self::muted_color(),
                format!("{}x{}", ctx.render_extent[0], ctx.render_extent[1]),
            );
        });
        ui.separator();

        egui::CollapsingHeader::new("Tweaks")
            .default_open(true)
            .show(ui, |ui| {
                Self::drag_f32(
                    ui,
                    "Camera speed",
                    &mut persisted.movement.camera_speed,
                    0.025,
                    0.0,
                    10.0,
                );
                Self::drag_f32(
                    ui,
                    "Camera smoothness",
                    &mut persisted.movement.camera_smoothness,
                    0.1,
                    0.0,
                    20.0,
                );
                Self::drag_f32(
                    ui,
                    "Sun rotation smoothness",
                    &mut persisted.movement.sun_rotation_smoothness,
                    0.1,
                    0.0,
                    20.0,
                );
                ui.checkbox(
                    &mut ctx.world_renderer.ircache.enable_scroll,
                    "Scroll irradiance cache",
                );
                Self::drag_u32(
                    ui,
                    "GI spatial reuse passes",
                    &mut ctx.world_renderer.rtdgi.spatial_reuse_pass_count,
                    1,
                    3,
                );
                ctx.world_renderer.rtdgi.spatial_reuse_pass_count = ctx
                    .world_renderer
                    .rtdgi
                    .spatial_reuse_pass_count
                    .clamp(1, 3);
                ui.checkbox(
                    &mut ctx.world_renderer.rtdgi.use_raytraced_reservoir_visibility,
                    "Ray-traced reservoir visibility",
                );
                ui.checkbox(
                    &mut ctx.world_renderer.rtr.reuse_rtdgi_rays,
                    "Allow diffuse ray reuse for reflections",
                );

                #[cfg(feature = "dlss")]
                {
                    ui.checkbox(&mut ctx.world_renderer.use_dlss, "Use DLSS");
                }
            });

        egui::CollapsingHeader::new("Overrides")
            .default_open(false)
            .show(ui, |ui| {
                macro_rules! do_flag {
                    ($flag:path, $name:literal) => {
                        let mut is_set = ctx.world_renderer.render_overrides.has_flag($flag);
                        if ui.checkbox(&mut is_set, $name).changed() {
                            ctx.world_renderer.render_overrides.set_flag($flag, is_set);
                        }
                    };
                }

                do_flag!(RenderOverrideFlags::FORCE_FACE_NORMALS, "Force face normals");
                do_flag!(RenderOverrideFlags::NO_NORMAL_MAPS, "No normal maps");
                do_flag!(RenderOverrideFlags::FLIP_NORMAL_MAP_YZ, "Flip normal map YZ");
                do_flag!(RenderOverrideFlags::NO_METAL, "No metal");
                Self::drag_f32(
                    ui,
                    "Roughness scale",
                    &mut ctx.world_renderer.render_overrides.material_roughness_scale,
                    0.001,
                    0.0,
                    4.0,
                );
            });

        egui::CollapsingHeader::new("Debug")
            .default_open(false)
            .show(ui, |ui| {
                ui.radio_value(
                    &mut ctx.world_renderer.debug_mode,
                    RenderDebugMode::None,
                    "Scene geometry",
                );

                egui::ComboBox::from_label("Shading")
                    .selected_text(match ctx.world_renderer.debug_shading_mode {
                        0 => "Default",
                        1 => "No base color",
                        2 => "Diffuse GI",
                        3 => "Reflections",
                        4 => "RTX OFF",
                        _ => "Irradiance cache",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 0usize, "Default");
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 1usize, "No base color");
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 2usize, "Diffuse GI");
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 3usize, "Reflections");
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 4usize, "RTX OFF");
                        ui.selectable_value(&mut ctx.world_renderer.debug_shading_mode, 5usize, "Irradiance cache");
                    });

                Self::drag_u32(ui, "Max FPS", &mut self.max_fps, 1, MAX_FPS_LIMIT);

                let mut allow_pass_overlap = unsafe { dist_render::rg::RG_ALLOW_PASS_OVERLAP };
                if ui.checkbox(&mut allow_pass_overlap, "Allow pass overlap").changed() {
                    unsafe {
                        dist_render::rg::RG_ALLOW_PASS_OVERLAP = allow_pass_overlap;
                    }
                }
            });

        egui::CollapsingHeader::new("GPU passes")
            .default_open(true)
            .show(ui, |ui| {
                if let Some(report) = gpu_profiler::profiler().last_report() {
                    for (scope_index, scope) in report.scopes.iter().enumerate() {
                        if scope.name == "debug" || scope.name.starts_with('_') {
                            continue;
                        }

                        let render_debug_hook = dist_render::rg::RenderDebugHook {
                            name: scope.name.clone(),
                            id: scope_index as u64,
                        };
                        let is_selected = self
                            .locked_rg_debug_hook
                            .as_ref()
                            .map(|hook| hook.render_debug_hook == render_debug_hook)
                            .unwrap_or(false);

                        let response = ui.selectable_label(
                            is_selected,
                            format!("{:>7.3} ms  {}", scope.duration.ms(), scope.name),
                        );

                        if response.hovered() {
                            ctx.world_renderer.rg_debug_hook = Some(dist_render::rg::GraphDebugHook {
                                render_debug_hook,
                            });
                        }

                        if response.clicked() {
                            if self.locked_rg_debug_hook == ctx.world_renderer.rg_debug_hook {
                                self.locked_rg_debug_hook = None;
                            } else {
                                self.locked_rg_debug_hook = ctx.world_renderer.rg_debug_hook.clone();
                            }
                        }
                    }
                } else {
                    ui.colored_label(Self::muted_color(), "No GPU profile data yet.");
                }
            });
    }

    fn draw_sequence_section(&mut self, ui: &mut egui::Ui, persisted: &mut PersistedState) {
        ui.horizontal(|ui| {
            if ui.button("Add key").clicked() {
                self.add_sequence_keyframe(persisted);
            }

            if self.is_sequence_playing() {
                if ui.button("Stop").clicked() {
                    self.stop_sequence();
                }
            } else if ui.button("Play").clicked() {
                self.play_sequence(persisted);
            }

            if self.active_camera_key.is_some() && ui.button("Deselect key").clicked() {
                self.active_camera_key = None;
            }
        });

        Self::drag_f32(
            ui,
            "Playback speed",
            &mut self.sequence_playback_speed,
            0.01,
            0.0,
            4.0,
        );
        ui.separator();

        enum Command {
            JumpToKey(usize),
            DeleteKey(usize),
            ReplaceKey(usize),
            None,
        }

        let mut command = Command::None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            persisted.sequence.each_key(|i, item| {
                ui.group(|ui| {
                    let active = Some(i) == self.active_camera_key;

                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(active, format!("Key {}", i))
                            .clicked()
                        {
                            command = Command::JumpToKey(i);
                        }

                        ui.add(
                            egui::DragValue::new(&mut item.duration)
                                .speed(0.01)
                                .prefix("duration "),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut item.value.camera_position.is_some, "Pos");
                        ui.checkbox(&mut item.value.camera_direction.is_some, "Dir");
                        ui.checkbox(&mut item.value.towards_sun.is_some, "Sun");
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Replace").clicked() {
                            command = Command::ReplaceKey(i);
                        }

                        if ui.button("Delete").clicked() {
                            command = Command::DeleteKey(i);
                        }
                    });
                });
            });
        });

        match command {
            Command::JumpToKey(i) => self.jump_to_sequence_key(persisted, i),
            Command::DeleteKey(i) => self.delete_camera_sequence_key(persisted, i),
            Command::ReplaceKey(i) => self.replace_camera_sequence_key(persisted, i),
            Command::None => {}
        }
    }

    fn draw_menu_bar(
        &mut self,
        egui_ctx: &egui::CtxRef,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        egui::TopBottomPanel::top("main_menu_bar")
            .height_range(34.0..=34.0)
            .frame(egui::Frame::none().fill(Self::chrome_fill()))
            .show(egui_ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "Create", |ui| {
                        if ui.button("Empty GameObject").clicked() {
                            let name = format!("GameObject {}", persisted.scene.next_game_object_id);
                            let game_object_id = persisted
                                .scene
                                .create_game_object(name, SceneElementTransform::IDENTITY);
                            self.selected_game_object = Some(game_object_id);
                        }
                    });

                    egui::menu::menu(ui, "Playback", |ui| {
                        if ui.button("Add Keyframe").clicked() {
                            self.add_sequence_keyframe(persisted);
                        }

                        if self.is_sequence_playing() {
                            if ui.button("Stop").clicked() {
                                self.stop_sequence();
                            }
                        } else if ui.button("Play").clicked() {
                            self.play_sequence(persisted);
                        }
                    });

                    if ui.button("Scene Root").clicked() {
                        self.selected_game_object = None;
                    }

                    ui.separator();
                    ui.colored_label(
                        Self::muted_color(),
                        format!("CPU {:.2} ms", ctx.dt_filtered * 1000.0),
                    );

                    if let Some(report) = gpu_profiler::profiler().last_report() {
                        let gpu_time_ms: f64 = report.scopes.iter().map(|scope| scope.duration.ms()).sum();
                        ui.colored_label(Self::muted_color(), format!("GPU {:.2} ms", gpu_time_ms));
                    }

                    ui.colored_label(
                        Self::muted_color(),
                        format!("Viewport {}x{}", ctx.render_extent[0], ctx.render_extent[1]),
                    );
                });
            });
    }

    fn draw_viewport_panel(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        ui.horizontal(|ui| {
            ui.heading("Viewport");
            ui.colored_label(
                Self::muted_color(),
                format!("{}x{}", ctx.render_extent[0], ctx.render_extent[1]),
            );
        });
        ui.separator();

        let available = ui.available_size();
        if available.x <= 1.0 || available.y <= 1.0 {
            self.viewport_hovered = false;
            return;
        }

        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        ctx.render_extent = [
            Self::quantize_viewport_extent(available.x * pixels_per_point),
            Self::quantize_viewport_extent(available.y * pixels_per_point),
        ];

        let (world_to_clip, world_to_view, eye_position) =
            self.viewport_camera_matrices(persisted, ctx);

        let drag_mask = 1 | (1 << 2);

        egui::Frame::none()
            .fill(Self::viewport_fill())
            .show(ui, |ui| {
                let Some(texture_id) = ctx.viewport_texture_id else {
                    self.viewport_hovered = false;
                    ui.colored_label(Self::warn_color(), "Viewport texture is unavailable.");
                    return;
                };

                let response = ui.add(
                    egui::Image::new(texture_id, ui.available_size()).sense(egui::Sense::hover()),
                );
                self.viewport_hovered = response.hovered();

                let viewport_rect = response.rect;
                let projected_objects = Self::project_scene_objects(
                    &persisted.scene,
                    viewport_rect,
                    world_to_clip,
                    world_to_view,
                );
                let selected_projection = self.selected_game_object.and_then(|selected| {
                    projected_objects
                        .iter()
                        .find(|projected| projected.game_object_id == selected)
                });
                let gizmo_axes = selected_projection
                    .map(|selected| {
                        Self::gizmo_axis_projections(
                            viewport_rect,
                            world_to_clip,
                            world_to_view,
                            eye_position,
                            selected.world_position,
                        )
                    })
                    .unwrap_or_default();

                let pointer_position = ui.input().pointer.interact_pos();
                let primary_down = ui.input().pointer.button_down(egui::PointerButton::Primary);
                let primary_pressed = ui.input().pointer.any_pressed() && primary_down;
                let primary_released = ui.input().pointer.any_released() && !primary_down;

                if self.viewport_hovered && primary_pressed {
                    if let Some(pointer_position) = pointer_position {
                        const AXIS_PICK_RADIUS: f32 = 10.0;

                        let axis_hit = gizmo_axes
                            .iter()
                            .filter_map(|axis| {
                                let distance = Self::screen_distance_to_segment(
                                    pointer_position,
                                    axis.screen_origin,
                                    axis.screen_end,
                                );
                                if distance <= AXIS_PICK_RADIUS {
                                    Some((axis, distance))
                                } else {
                                    None
                                }
                            })
                            .min_by(|lhs, rhs| {
                                lhs.1.partial_cmp(&rhs.1).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(axis, _)| axis);

                        if let Some(axis) = axis_hit {
                            if let Some(selected) = selected_projection {
                                self.viewport_gizmo_drag = Some(ViewportGizmoDragState {
                                    game_object_id: selected.game_object_id,
                                    axis: axis.axis,
                                    pointer_origin: Vec2::new(pointer_position.x, pointer_position.y),
                                    screen_direction: axis.screen_direction,
                                    start_world_position: selected.world_position,
                                    pixels_per_unit: axis.pixels_per_unit.max(1e-4),
                                });
                                self.viewport_click_origin = None;
                            }
                        } else {
                            self.viewport_click_origin = Some(Vec2::new(pointer_position.x, pointer_position.y));
                        }
                    }
                }

                if let Some(drag_state) = self.viewport_gizmo_drag {
                    if primary_down {
                        if let Some(pointer_position) = pointer_position {
                            let pointer_delta =
                                Vec2::new(pointer_position.x, pointer_position.y) - drag_state.pointer_origin;
                            let axis_distance = pointer_delta.dot(drag_state.screen_direction);
                            let world_offset =
                                drag_state.axis.world_vector() * (axis_distance / drag_state.pixels_per_unit);
                            Self::set_game_object_world_position(
                                &mut persisted.scene,
                                drag_state.game_object_id,
                                drag_state.start_world_position + world_offset,
                            );
                        }
                    } else {
                        self.viewport_gizmo_drag = None;
                    }
                }

                if primary_released {
                    self.viewport_gizmo_drag = None;

                    if let Some(click_origin) = self.viewport_click_origin.take() {
                        if let Some(pointer_position) = pointer_position {
                            let pointer_position = Vec2::new(pointer_position.x, pointer_position.y);
                            if (pointer_position - click_origin).length() <= 6.0 {
                                self.selected_game_object = Self::pick_game_object_at(
                                    &projected_objects,
                                    egui::pos2(pointer_position.x, pointer_position.y),
                                );
                            }
                        }
                    }
                }

                let painter = ui.painter_at(viewport_rect);
                for projected in &projected_objects {
                    if !viewport_rect.expand(20.0).contains(projected.screen_position) {
                        continue;
                    }

                    let is_selected = self.selected_game_object == Some(projected.game_object_id);
                    let fill = if is_selected {
                        Color32::from_rgb(244, 244, 248)
                    } else {
                        Color32::from_rgba_unmultiplied(214, 220, 228, 170)
                    };
                    painter.circle_filled(
                        projected.screen_position,
                        if is_selected { 5.0 } else { 3.0 },
                        fill,
                    );

                    if is_selected {
                        painter.circle_stroke(
                            projected.screen_position,
                            8.0,
                            egui::Stroke::new(1.5, Color32::from_rgb(61, 180, 255)),
                        );
                    }
                }

                let active_axis = self.viewport_gizmo_drag.map(|drag_state| drag_state.axis);
                for axis in &gizmo_axes {
                    let color = if active_axis == Some(axis.axis) {
                        Color32::WHITE
                    } else {
                        Self::axis_color(axis.axis)
                    };
                    painter.line_segment(
                        [axis.screen_origin, axis.screen_end],
                        egui::Stroke::new(2.5, color),
                    );
                    painter.circle_filled(axis.screen_end, 5.0, color);
                }

                if self.viewport_hovered && (self.mouse.buttons_pressed & drag_mask) != 0 {
                    self.viewport_pointer_captured = true;
                }
            });

        if (self.mouse.buttons_held & drag_mask) == 0 {
            self.viewport_pointer_captured = false;
        }
    }

    pub fn do_gui(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.keyboard.was_just_pressed(self.keymap_config.ui.toggle) {
            self.show_gui = !self.show_gui;
        }

        ctx.world_renderer.rg_debug_hook = self.locked_rg_debug_hook.clone();
        self.viewport_hovered = false;

        if !self.show_gui {
            self.viewport_pointer_captured = false;
        }

        if self.show_gui {
            ctx.egui.take().unwrap().frame(|egui_ctx| {
                self.validate_selected_game_object(&persisted.scene);
                self.draw_menu_bar(egui_ctx, persisted, ctx);

                egui::SidePanel::left("hierarchy_panel")
                    .default_width(320.0)
                    .frame(egui::Frame::none().fill(Self::panel_fill()))
                    .show(egui_ctx, |ui| {
                        self.draw_hierarchy_panel(ui, persisted);
                    });

                egui::SidePanel::right("inspector_panel")
                    .default_width(380.0)
                    .frame(egui::Frame::none().fill(Self::panel_fill()))
                    .show(egui_ctx, |ui| {
                        self.draw_inspector_panel(ui, persisted, ctx);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Self::chrome_fill()))
                    .show(egui_ctx, |ui| {
                    self.draw_viewport_panel(ui, persisted, ctx);
                    });
            });
        } else {
            self.viewport_hovered = true;
            ctx.render_extent = [ctx.window.inner_size().width.max(1), ctx.window.inner_size().height.max(1)];
            self.viewport_click_origin = None;
            self.viewport_gizmo_drag = None;
        }
    }
}
