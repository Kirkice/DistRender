use imgui::im_str;
use dist_render::RenderOverrideFlags;
use dist_render_simple::*;

use crate::{
    persisted::{
        CameraComponent, GameObject, GameObjectId, LocalLightsComponent, SceneComponent,
        SceneElementTransform, SceneState, SunComponent,
    },
    runtime::{RuntimeState, MAX_FPS_LIMIT},
    PersistedState,
};

impl RuntimeState {
    fn hierarchy_drag_drop_name() -> &'static imgui::ImStr {
        im_str!("hierarchy_game_object")
    }

    fn validate_selected_game_object(&mut self, scene: &SceneState) {
        if let Some(selected_game_object) = self.selected_game_object {
            if scene.find_game_object(selected_game_object).is_none() {
                self.selected_game_object = None;
            }
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
        ui: &imgui::Ui<'_>,
        scene: &SceneState,
        game_object_id: GameObjectId,
        depth: usize,
        pending_reparent: &mut Option<(GameObjectId, Option<GameObjectId>)>,
    ) {
        let Some((label, child_ids, is_builtin, drag_label, drop_label)) = scene
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
                    format!("Reparent {}", game_object.name),
                    format!("Drop to parent under {}", game_object.name),
                )
            })
        else {
            return;
        };

        let style = if self.selected_game_object == Some(game_object_id) {
            Some(ui.push_style_color(imgui::StyleColor::Text, [1.0, 1.0, 0.1, 1.0]))
        } else {
            None
        };

        if ui.button(&im_str!("{}##hierarchy_{}", label, game_object_id.0), [0.0, 0.0]) {
            self.selected_game_object = Some(game_object_id);
        }

        if let Some(style) = style {
            style.pop(ui);
        }

        if !is_builtin {
            if let Some(tooltip) =
                imgui::DragDropSource::new(Self::hierarchy_drag_drop_name()).begin_payload(
                    ui,
                    game_object_id,
                )
            {
                ui.text(drag_label);
                tooltip.end();
            }
        }

        if let Some(target) = imgui::DragDropTarget::new(ui) {
            if let Some(Ok(payload)) = target.accept_payload::<GameObjectId>(
                Self::hierarchy_drag_drop_name(),
                imgui::DragDropFlags::ACCEPT_BEFORE_DELIVERY,
            ) {
                let is_valid = scene.can_reparent_game_object(payload.data, Some(game_object_id));
                if payload.preview {
                    if is_valid {
                        ui.tooltip_text(drop_label);
                    } else {
                        ui.tooltip_text(
                            "Built-in scene objects must stay at root, and GameObjects cannot be parented to themselves or their descendants.",
                        );
                    }
                }

                if payload.delivery && is_valid {
                    *pending_reparent = Some((payload.data, Some(game_object_id)));
                    self.selected_game_object = Some(payload.data);
                }
            }

            target.pop();
        }

        for child_id in child_ids {
            self.draw_hierarchy_node(ui, scene, child_id, depth + 1, pending_reparent);
        }
    }

    fn draw_hierarchy_panel(&mut self, ui: &imgui::Ui<'_>, persisted: &mut PersistedState) {
        imgui::Window::new(im_str!("Hierarchy"))
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .size([360.0, 520.0], imgui::Condition::FirstUseEver)
            .build(ui, || {
                let mut pending_reparent = None;

                ui.text(format!("GameObjects: {}", persisted.scene.game_objects.len()));
                ui.text_disabled("Drag onto a GameObject to reparent, or onto Scene to make it root.");

                let scene_style = if self.selected_game_object.is_none() {
                    Some(ui.push_style_color(imgui::StyleColor::Text, [1.0, 1.0, 0.1, 1.0]))
                } else {
                    None
                };
                if ui.button(im_str!("Scene"), [0.0, 0.0]) {
                    self.selected_game_object = None;
                }
                if let Some(style) = scene_style {
                    style.pop(ui);
                }

                if let Some(target) = imgui::DragDropTarget::new(ui) {
                    if let Some(Ok(payload)) = target.accept_payload::<GameObjectId>(
                        Self::hierarchy_drag_drop_name(),
                        imgui::DragDropFlags::ACCEPT_BEFORE_DELIVERY,
                    ) {
                        let is_valid = persisted
                            .scene
                            .can_reparent_game_object(payload.data, None);
                        if payload.preview {
                            if is_valid {
                                ui.tooltip_text("Drop to make this GameObject a root object.");
                            } else {
                                ui.tooltip_text("This GameObject is already at the root of the scene.");
                            }
                        }

                        if payload.delivery && is_valid {
                            pending_reparent = Some((payload.data, None));
                            self.selected_game_object = Some(payload.data);
                        }
                    }

                    target.pop();
                }

                ui.same_line(0.0);
                if ui.button(im_str!("Create Empty"), [0.0, 0.0]) {
                    let name = format!("GameObject {}", persisted.scene.next_game_object_id);
                    let game_object_id =
                        persisted.scene.create_game_object(name, SceneElementTransform::IDENTITY);
                    self.selected_game_object = Some(game_object_id);
                }

                let root_ids: Vec<_> = persisted
                    .scene
                    .game_objects
                    .iter()
                    .filter(|game_object| game_object.parent.is_none())
                    .map(|game_object| game_object.id)
                    .collect();

                for root_id in root_ids {
                    self.draw_hierarchy_node(
                        ui,
                        &persisted.scene,
                        root_id,
                        0,
                        &mut pending_reparent,
                    );
                }

                if let Some((game_object_id, new_parent)) = pending_reparent {
                    let _ = persisted.scene.reparent_game_object(game_object_id, new_parent);
                }
            });
    }

    fn draw_transform_inspector(ui: &imgui::Ui<'_>, transform: &mut SceneElementTransform) {
        ui.text("Position");
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("x"))
            .speed(0.01)
            .build(ui, &mut transform.position.x);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("y"))
            .speed(0.01)
            .build(ui, &mut transform.position.y);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("z"))
            .speed(0.01)
            .build(ui, &mut transform.position.z);

        ui.text("Rotation");
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("rx"))
            .speed(0.1)
            .build(ui, &mut transform.rotation_euler_degrees.x);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("ry"))
            .speed(0.1)
            .build(ui, &mut transform.rotation_euler_degrees.y);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("rz"))
            .speed(0.1)
            .build(ui, &mut transform.rotation_euler_degrees.z);

        ui.text("Scale");
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("sx"))
            .range(0.001..=1000.0)
            .speed(0.05)
            .flags(imgui::SliderFlags::LOGARITHMIC)
            .build(ui, &mut transform.scale.x);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("sy"))
            .range(0.001..=1000.0)
            .speed(0.05)
            .flags(imgui::SliderFlags::LOGARITHMIC)
            .build(ui, &mut transform.scale.y);

        ui.same_line(0.0);
        ui.set_next_item_width(90.0);
        imgui::Drag::<f32>::new(im_str!("sz"))
            .range(0.001..=1000.0)
            .speed(0.05)
            .flags(imgui::SliderFlags::LOGARITHMIC)
            .build(ui, &mut transform.scale.z);
    }

    fn draw_component_inspector(
        &mut self,
        ui: &imgui::Ui<'_>,
        component: &mut SceneComponent,
    ) {
        match component {
            SceneComponent::MeshRenderer(mesh_renderer) => {
                ui.checkbox(im_str!("Enabled"), &mut mesh_renderer.enabled);
                ui.text(format!("Source: {:?}", mesh_renderer.source));

                imgui::Drag::<f32>::new(im_str!("Emissive"))
                    .range(0.0..=10.0)
                    .speed(0.05)
                    .build(ui, &mut mesh_renderer.emissive_multiplier);
            }
            SceneComponent::Camera(camera) => {
                ui.checkbox(im_str!("Enabled"), &mut camera.enabled);
                ui.checkbox(im_str!("Primary"), &mut camera.primary);

                imgui::Drag::<f32>::new(im_str!("Vertical FOV"))
                    .range(1.0..=120.0)
                    .speed(0.25)
                    .build(ui, &mut camera.vertical_fov);
            }
            SceneComponent::Sun(sun) => {
                ui.checkbox(im_str!("Enabled"), &mut sun.enabled);

                imgui::Drag::<f32>::new(im_str!("Sun size"))
                    .range(0.0..=10.0)
                    .speed(0.02)
                    .build(ui, &mut sun.size_multiplier);

                ui.text(format!(
                    "Direction: {:.3}, {:.3}, {:.3}",
                    sun.controller.towards_sun().x,
                    sun.controller.towards_sun().y,
                    sun.controller.towards_sun().z,
                ));
            }
            SceneComponent::LocalLights(local_lights) => {
                ui.checkbox(im_str!("Enabled"), &mut local_lights.enabled);

                imgui::Drag::<u32>::new(im_str!("Count"))
                    .range(0..=64)
                    .build(ui, &mut local_lights.count);

                imgui::Drag::<f32>::new(im_str!("Theta"))
                    .range(-10.0..=10.0)
                    .speed(0.01)
                    .build(ui, &mut local_lights.theta);

                imgui::Drag::<f32>::new(im_str!("Phi"))
                    .range(-10.0..=10.0)
                    .speed(0.01)
                    .build(ui, &mut local_lights.phi);

                imgui::Drag::<f32>::new(im_str!("Distance"))
                    .range(0.0..=100.0)
                    .speed(0.05)
                    .build(ui, &mut local_lights.distance);

                imgui::Drag::<f32>::new(im_str!("Multiplier"))
                    .range(0.0..=1000.0)
                    .speed(0.5)
                    .build(ui, &mut local_lights.multiplier);
            }
        }
    }

    fn draw_scene_inspector(
        &mut self,
        ui: &imgui::Ui<'_>,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        ui.text("Scene");
        ui.text(format!("GameObjects: {}", persisted.scene.game_objects.len()));

        if imgui::CollapsingHeader::new(im_str!("Environment"))
            .default_open(true)
            .build(ui)
        {
            if let Some(ibl) = persisted.scene.ibl.as_ref() {
                ui.text(format!("IBL: {:?}", ibl));
                if ui.button(im_str!("Unload IBL"), [0.0, 0.0]) {
                    ctx.world_renderer.ibl.unload_image();
                    persisted.scene.ibl = None;
                }
            } else {
                ui.text("Drag a sphere-mapped .hdr/.exr to load as IBL");
            }
        }

        if imgui::CollapsingHeader::new(im_str!("Exposure"))
            .default_open(true)
            .build(ui)
        {
            imgui::Drag::<f32>::new(im_str!("EV shift"))
                .range(-8.0..=12.0)
                .speed(0.01)
                .build(ui, &mut persisted.exposure.ev_shift);

            ui.checkbox(
                im_str!("Use dynamic exposure"),
                &mut persisted.exposure.use_dynamic_adaptation,
            );

            imgui::Drag::<f32>::new(im_str!("Adaptation speed"))
                .range(-4.0..=4.0)
                .speed(0.01)
                .build(ui, &mut persisted.exposure.dynamic_adaptation_speed);

            imgui::Drag::<f32>::new(im_str!("Luminance histogram low clip"))
                .range(0.0..=1.0)
                .speed(0.001)
                .build(ui, &mut persisted.exposure.dynamic_adaptation_low_clip);
            persisted.exposure.dynamic_adaptation_low_clip = persisted
                .exposure
                .dynamic_adaptation_low_clip
                .clamp(0.0, 1.0);

            imgui::Drag::<f32>::new(im_str!("Luminance histogram high clip"))
                .range(0.0..=1.0)
                .speed(0.001)
                .build(ui, &mut persisted.exposure.dynamic_adaptation_high_clip);
            persisted.exposure.dynamic_adaptation_high_clip = persisted
                .exposure
                .dynamic_adaptation_high_clip
                .clamp(0.0, 1.0);

            imgui::Drag::<f32>::new(im_str!("Contrast"))
                .range(1.0..=1.5)
                .speed(0.001)
                .build(ui, &mut persisted.exposure.contrast);
        }

        if imgui::CollapsingHeader::new(im_str!("Lighting"))
            .default_open(true)
            .build(ui)
        {
            imgui::Drag::<f32>::new(im_str!("Emissive multiplier"))
                .range(0.0..=10.0)
                .speed(0.1)
                .build(ui, &mut persisted.scene.render_settings.emissive_multiplier);

            ui.checkbox(
                im_str!("Enable emissive"),
                &mut persisted.scene.render_settings.enable_emissive,
            );

            ui.text_disabled("Select Main Camera, Sun, or Local Lights in Hierarchy for per-object controls.");
        }

        if imgui::CollapsingHeader::new(im_str!("Editor"))
            .default_open(false)
            .build(ui)
        {
            imgui::Drag::<f32>::new(im_str!("Camera speed"))
                .range(0.0..=10.0)
                .speed(0.025)
                .build(ui, &mut persisted.movement.camera_speed);

            imgui::Drag::<f32>::new(im_str!("Camera smoothness"))
                .range(0.0..=20.0)
                .speed(0.1)
                .build(ui, &mut persisted.movement.camera_smoothness);

            imgui::Drag::<f32>::new(im_str!("Sun rotation smoothness"))
                .range(0.0..=20.0)
                .speed(0.1)
                .build(ui, &mut persisted.movement.sun_rotation_smoothness);
        }

        ui.text("Select a GameObject in Hierarchy to edit its Transform and Components.");
    }

    fn draw_game_object_inspector(
        &mut self,
        ui: &imgui::Ui<'_>,
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
        let mut component_to_add = None;
        let mut component_to_remove = None;
        let mut refresh_scene_singletons = false;

        if let Some(game_object) = persisted.scene.find_game_object_mut(game_object_id) {
            let mut name = imgui::ImString::new(game_object.name.as_str());
            ui.set_next_item_width(-1.0);
            if ui.input_text(im_str!("Name"), &mut name).build() {
                let trimmed = name.to_str().trim();
                game_object.name = if trimmed.is_empty() {
                    format!("GameObject {}", game_object.id.0)
                } else {
                    trimmed.to_owned()
                };
            }

            ui.text(format!("ID: {}", game_object.id.0));

            ui.checkbox(im_str!("Enabled"), &mut game_object.enabled);

            ui.text(format!("Parent: {}", parent_label));

            if let Some(builtin) = game_object.builtin {
                ui.text(format!("Built-in role: {}", builtin.display_name()));
            } else if ui.button(im_str!("Delete GameObject"), [0.0, 0.0]) {
                delete_selected_game_object = true;
            }

            if imgui::CollapsingHeader::new(im_str!("Transform"))
                .default_open(true)
                .build(ui)
            {
                Self::draw_transform_inspector(ui, &mut game_object.transform);
            }

            if game_object.components.is_empty() {
                ui.text("No Components");
            }

            if imgui::CollapsingHeader::new(im_str!("Add Component"))
                .default_open(true)
                .build(ui)
            {
                if Self::game_object_has_scene_role_component(game_object) {
                    ui.text_disabled(
                        "This GameObject already owns a Camera, Sun, or Local Lights component.",
                    );
                } else {
                    if ui.button(im_str!("Add Camera"), [0.0, 0.0]) {
                        component_to_add = Some(SceneComponent::Camera(CameraComponent::default()));
                    }

                    ui.same_line(0.0);
                    if ui.button(im_str!("Add Sun"), [0.0, 0.0]) {
                        component_to_add = Some(SceneComponent::Sun(SunComponent::default()));
                    }

                    ui.same_line(0.0);
                    if ui.button(im_str!("Add Local Lights"), [0.0, 0.0]) {
                        component_to_add =
                            Some(SceneComponent::LocalLights(LocalLightsComponent::default()));
                    }
                }

                ui.text_disabled("MeshRenderer creation still comes from importing or dragging mesh assets.");
            }

            for (component_index, component) in game_object.components.iter_mut().enumerate() {
                let label = im_str!(
                    "{}##component_{}_{}",
                    component.kind_name(),
                    game_object.id.0,
                    component_index
                );

                if imgui::CollapsingHeader::new(&label)
                    .default_open(true)
                    .build(ui)
                {
                    let is_builtin_component = game_object
                        .builtin
                        .map(|builtin| builtin.matches_component(component))
                        .unwrap_or(false);

                    if is_builtin_component {
                        ui.text_disabled("Protected scene singleton component");
                    } else if ui.small_button(&im_str!(
                        "Remove##remove_component_{}_{}",
                        game_object.id.0,
                        component_index
                    )) {
                        component_to_remove = Some(component_index);
                    }

                    self.draw_component_inspector(ui, component);

                    if matches!(
                        component,
                        SceneComponent::Camera(_) | SceneComponent::Sun(_) | SceneComponent::LocalLights(_)
                    ) {
                        refresh_scene_singletons = true;
                    }
                }
            }
        } else {
            self.selected_game_object = None;
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
        ui: &imgui::Ui<'_>,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        imgui::Window::new(im_str!("Inspector"))
            .position([380.0, 10.0], imgui::Condition::FirstUseEver)
            .size([430.0, 520.0], imgui::Condition::FirstUseEver)
            .build(ui, || match self.selected_game_object {
                Some(selected_game_object) => {
                    self.draw_game_object_inspector(ui, persisted, selected_game_object);
                }
                None => self.draw_scene_inspector(ui, persisted, ctx),
            });
    }

    pub fn do_gui(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.keyboard.was_just_pressed(self.keymap_config.ui.toggle) {
            self.show_gui = !self.show_gui;
        }

        ctx.world_renderer.rg_debug_hook = self.locked_rg_debug_hook.clone();

        if self.show_gui {
            ctx.imgui.take().unwrap().frame(|ui| {
                self.validate_selected_game_object(&persisted.scene);
                self.draw_hierarchy_panel(ui, persisted);
                self.draw_inspector_panel(ui, persisted, ctx);

                if imgui::CollapsingHeader::new(im_str!("Tweaks"))
                    .default_open(true)
                    .build(ui)
                {
                    imgui::Drag::<f32>::new(im_str!("Camera speed"))
                        .range(0.0..=10.0)
                        .speed(0.025)
                        .build(ui, &mut persisted.movement.camera_speed);

                    imgui::Drag::<f32>::new(im_str!("Camera smoothness"))
                        .range(0.0..=20.0)
                        .speed(0.1)
                        .build(ui, &mut persisted.movement.camera_smoothness);

                    imgui::Drag::<f32>::new(im_str!("Sun rotation smoothness"))
                        .range(0.0..=20.0)
                        .speed(0.1)
                        .build(ui, &mut persisted.movement.sun_rotation_smoothness);

                    /*ui.checkbox(
                        im_str!("Show world radiance cache"),
                        &mut ctx.world_renderer.debug_show_wrc,
                    );*/

                    /*if ui.radio_button_bool(
                        im_str!("Move sun"),
                        left_click_edit_mode == LeftClickEditMode::MoveSun,
                    ) {
                        left_click_edit_mode = LeftClickEditMode::MoveSun;
                    }

                    if ui.radio_button_bool(
                        im_str!("Move local lights"),
                        left_click_edit_mode == LeftClickEditMode::MoveLocalLights,
                    ) {
                        left_click_edit_mode = LeftClickEditMode::MoveLocalLights;
                    }

                    imgui::Drag::<u32>::new(im_str!("Light count"))
                        .range(0..=10)
                        .build(ui, &mut state.lights.count);*/

                    ui.checkbox(
                        im_str!("Scroll irradiance cache"),
                        &mut ctx.world_renderer.ircache.enable_scroll,
                    );

                    imgui::Drag::<u32>::new(im_str!("GI spatial reuse passes"))
                        .range(1..=3)
                        .build(ui, &mut ctx.world_renderer.rtdgi.spatial_reuse_pass_count);

                    ctx.world_renderer.rtdgi.spatial_reuse_pass_count = ctx
                        .world_renderer
                        .rtdgi
                        .spatial_reuse_pass_count
                        .clamp(1, 3);

                    ui.checkbox(
                        im_str!("Ray-traced reservoir visibility"),
                        &mut ctx.world_renderer.rtdgi.use_raytraced_reservoir_visibility,
                    );

                    ui.checkbox(
                        im_str!("Allow diffuse ray reuse for reflections"),
                        &mut ctx.world_renderer.rtr.reuse_rtdgi_rays,
                    );

                    #[cfg(feature = "dlss")]
                    {
                        ui.checkbox(im_str!("Use DLSS"), &mut ctx.world_renderer.use_dlss);
                    }
                }

                if imgui::CollapsingHeader::new(im_str!("Overrides"))
                    .default_open(false)
                    .build(ui)
                {
                    macro_rules! do_flag {
                        ($flag:path, $name:literal) => {
                            let mut is_set: bool =
                                ctx.world_renderer.render_overrides.has_flag($flag);
                            ui.checkbox(im_str!($name), &mut is_set);
                            ctx.world_renderer.render_overrides.set_flag($flag, is_set);
                        };
                    }

                    do_flag!(
                        RenderOverrideFlags::FORCE_FACE_NORMALS,
                        "Force face normals"
                    );
                    do_flag!(RenderOverrideFlags::NO_NORMAL_MAPS, "No normal maps");
                    do_flag!(
                        RenderOverrideFlags::FLIP_NORMAL_MAP_YZ,
                        "Flip normal map YZ"
                    );
                    do_flag!(RenderOverrideFlags::NO_METAL, "No metal");

                    imgui::Drag::<f32>::new(im_str!("Roughness scale"))
                        .range(0.0..=4.0)
                        .speed(0.001)
                        .build(
                            ui,
                            &mut ctx.world_renderer.render_overrides.material_roughness_scale,
                        );
                }

                if imgui::CollapsingHeader::new(im_str!("Sequence"))
                    .default_open(false)
                    .build(ui)
                {
                    if ui.button(im_str!("Add key"), [0.0, 0.0]) {
                        self.add_sequence_keyframe(persisted);
                    }

                    ui.same_line(0.0);
                    if self.is_sequence_playing() {
                        if ui.button(im_str!("Stop"), [0.0, 0.0]) {
                            self.stop_sequence();
                        }
                    } else if ui.button(im_str!("Play"), [0.0, 0.0]) {
                        self.play_sequence(persisted);
                    }

                    ui.same_line(0.0);
                    ui.set_next_item_width(60.0);
                    imgui::Drag::<f32>::new(im_str!("Speed"))
                        .range(0.0..=4.0)
                        .speed(0.01)
                        .build(ui, &mut self.sequence_playback_speed);

                    if self.active_camera_key.is_some() {
                        ui.same_line(0.0);
                        if ui.button(im_str!("Deselect key"), [0.0, 0.0]) {
                            self.active_camera_key = None;
                        }
                    }

                    enum Cmd {
                        JumpToKey(usize),
                        DeleteKey(usize),
                        ReplaceKey(usize),
                        None,
                    }
                    let mut cmd = Cmd::None;

                    persisted.sequence.each_key(|i, item| {
                        let active = Some(i) == self.active_camera_key;

                        let label = if active {
                            im_str!("-> {}:", i)
                        } else {
                            im_str!("{}:", i)
                        };

                        if ui.button(&label, [0.0, 0.0]) {
                            cmd = Cmd::JumpToKey(i);
                        }

                        ui.same_line(0.0);
                        ui.set_next_item_width(60.0);
                        imgui::InputFloat::new(ui, &im_str!("duration##{}", i), &mut item.duration)
                            .build();

                        ui.same_line(0.0);
                        ui.checkbox(
                            &im_str!("Pos##{}", i),
                            &mut item.value.camera_position.is_some,
                        );

                        ui.same_line(0.0);
                        ui.checkbox(
                            &im_str!("Dir##{}", i),
                            &mut item.value.camera_direction.is_some,
                        );

                        ui.same_line(0.0);
                        ui.checkbox(&im_str!("Sun##{}", i), &mut item.value.towards_sun.is_some);

                        ui.same_line(0.0);
                        if ui.button(&im_str!("Delete##{}", i), [0.0, 0.0]) {
                            cmd = Cmd::DeleteKey(i);
                        }

                        ui.same_line(0.0);
                        if ui.button(&im_str!("Replace##{}:", i), [0.0, 0.0]) {
                            cmd = Cmd::ReplaceKey(i);
                        }
                    });

                    match cmd {
                        Cmd::JumpToKey(i) => self.jump_to_sequence_key(persisted, i),
                        Cmd::DeleteKey(i) => self.delete_camera_sequence_key(persisted, i),
                        Cmd::ReplaceKey(i) => self.replace_camera_sequence_key(persisted, i),
                        Cmd::None => {}
                    }
                }

                if imgui::CollapsingHeader::new(im_str!("Debug"))
                    .default_open(false)
                    .build(ui)
                {
                    if ui.radio_button_bool(
                        im_str!("Scene geometry"),
                        ctx.world_renderer.debug_mode == RenderDebugMode::None,
                    ) {
                        ctx.world_renderer.debug_mode = RenderDebugMode::None;
                    }

                    /*if ui.radio_button_bool(
                        im_str!("World radiance cache"),
                        ctx.world_renderer.debug_mode == RenderDebugMode::WorldRadianceCache,
                    ) {
                        ctx.world_renderer.debug_mode = RenderDebugMode::WorldRadianceCache;
                    }*/

                    imgui::ComboBox::new(im_str!("Shading")).build_simple_string(
                        ui,
                        &mut ctx.world_renderer.debug_shading_mode,
                        &[
                            im_str!("Default"),
                            im_str!("No base color"),
                            im_str!("Diffuse GI"),
                            im_str!("Reflections"),
                            im_str!("RTX OFF"),
                            im_str!("Irradiance cache"),
                        ],
                    );

                    imgui::Drag::<u32>::new(im_str!("Max FPS"))
                        .range(1..=MAX_FPS_LIMIT)
                        .build(ui, &mut self.max_fps);

                    let mut allow_pass_overlap = unsafe { dist_render::rg::RG_ALLOW_PASS_OVERLAP };
                    ui.checkbox(im_str!("Allow pass overlap"), &mut allow_pass_overlap);
                    unsafe {
                        dist_render::rg::RG_ALLOW_PASS_OVERLAP = allow_pass_overlap;
                    }
                }

                if imgui::CollapsingHeader::new(im_str!("GPU passes"))
                    .default_open(true)
                    .build(ui)
                {
                    ui.text(format!("CPU frame time: {:.3}ms", ctx.dt_filtered * 1000.0));

                    if let Some(report) = gpu_profiler::profiler().last_report() {
                        let ordered_scopes = report.scopes.as_slice();
                        let gpu_time_ms: f64 =
                            ordered_scopes.iter().map(|scope| scope.duration.ms()).sum();

                        ui.text(format!("GPU frame time: {:.3}ms", gpu_time_ms));

                        for (scope_index, scope) in ordered_scopes.iter().enumerate() {
                            if scope.name == "debug" || scope.name.starts_with('_') {
                                continue;
                            }

                            let render_debug_hook = dist_render::rg::RenderDebugHook {
                                name: scope.name.clone(),
                                id: scope_index as u64,
                            };

                            let style = self.locked_rg_debug_hook.as_ref().and_then(|hook| {
                                if hook.render_debug_hook == render_debug_hook {
                                    Some(ui.push_style_color(
                                        imgui::StyleColor::Text,
                                        [1.0, 1.0, 0.1, 1.0],
                                    ))
                                } else {
                                    None
                                }
                            });

                            ui.text(format!("{}: {:.3}ms", scope.name, scope.duration.ms()));

                            if let Some(style) = style {
                                style.pop(ui);
                            }

                            if ui.is_item_hovered() {
                                ctx.world_renderer.rg_debug_hook =
                                    Some(dist_render::rg::GraphDebugHook { render_debug_hook });

                                if ui.is_item_clicked(imgui::MouseButton::Left) {
                                    if self.locked_rg_debug_hook == ctx.world_renderer.rg_debug_hook
                                    {
                                        self.locked_rg_debug_hook = None;
                                    } else {
                                        self.locked_rg_debug_hook =
                                            ctx.world_renderer.rg_debug_hook.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}
