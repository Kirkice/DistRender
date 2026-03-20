use super::*;

impl RuntimeState {
    fn game_object_has_scene_role_component(game_object: &GameObject) -> bool {
        game_object.components.iter().any(|component| {
            matches!(
                component,
                SceneComponent::Camera(_) | SceneComponent::Sun(_) | SceneComponent::LocalLights(_)
            )
        })
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

    fn draw_component_inspector(&mut self, ui: &mut egui::Ui, component: &mut SceneComponent) {
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
                Self::drag_f32(
                    ui,
                    "Camera speed",
                    &mut persisted.movement.camera_speed,
                    0.025,
                    MIN_CAMERA_SPEED,
                    MAX_CAMERA_SPEED,
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
            self.viewport_click_origin = None;
            self.viewport_gizmo_drag = None;
        }

        if refresh_scene_singletons {
            persisted.scene.refresh_editor_singletons();
        }
    }

    pub(super) fn draw_inspector_panel(
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
                    MIN_CAMERA_SPEED,
                    MAX_CAMERA_SPEED,
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
}