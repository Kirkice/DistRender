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
                ui.horizontal(|ui| {
                    Self::prop_label(ui, "Source");
                    ui.add(
                        egui::Label::new(format!("{:?}", mesh_renderer.source))
                            .small()
                            .text_color(Self::muted_color()),
                    );
                });

                ui.add_space(4.0);
                egui::CollapsingHeader::new("Material Overrides")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            Self::badge(ui, "instance");
                            ui.add(
                                egui::Label::new("Applies to all materials on this mesh instance.")
                                    .small()
                                    .text_color(Self::text_dim()),
                            );

                            if Self::subtle_button(ui, "Reset").clicked() {
                                mesh_renderer.reset_material_overrides();
                            }
                        });

                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            Self::prop_label(ui, "Base Color");
                            let mut base_color = [
                                mesh_renderer.base_color_tint[0],
                                mesh_renderer.base_color_tint[1],
                                mesh_renderer.base_color_tint[2],
                            ];
                            if ui.color_edit_button_rgb(&mut base_color).changed() {
                                mesh_renderer.base_color_tint[0] = base_color[0];
                                mesh_renderer.base_color_tint[1] = base_color[1];
                                mesh_renderer.base_color_tint[2] = base_color[2];
                            }
                        });

                        Self::drag_f32(
                            ui,
                            "Roughness",
                            &mut mesh_renderer.roughness_multiplier,
                            0.01,
                            0.0,
                            4.0,
                        );
                        Self::drag_f32(
                            ui,
                            "Metallic",
                            &mut mesh_renderer.metalness_multiplier,
                            0.01,
                            0.0,
                            4.0,
                        );
                        Self::drag_f32(
                            ui,
                            "Emissive",
                            &mut mesh_renderer.emissive_multiplier,
                            0.05,
                            0.0,
                            10.0,
                        );
                    });
            }
            SceneComponent::Camera(camera) => {
                ui.checkbox(&mut camera.enabled, "Enabled");
                ui.checkbox(&mut camera.primary, "Primary");
                Self::drag_f32(ui, "Vertical FOV", &mut camera.vertical_fov, 0.25, 1.0, 120.0);
            }
            SceneComponent::Sun(sun) => {
                ui.checkbox(&mut sun.enabled, "Enabled");
                Self::drag_f32(ui, "Sun size", &mut sun.size_multiplier, 0.02, 0.0, 10.0);
                ui.horizontal(|ui| {
                    Self::prop_label(ui, "Direction");
                    ui.add(
                        egui::Label::new(format!(
                            "{:.3}, {:.3}, {:.3}",
                            sun.controller.towards_sun().x,
                            sun.controller.towards_sun().y,
                            sun.controller.towards_sun().z,
                        ))
                        .text_color(Self::muted_color()),
                    );
                });
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
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Scene").heading().text_color(Self::text_bright()));
            ui.add_space(8.0);
            Self::badge(ui, &format!("{}", persisted.scene.game_objects.len()));
        });
        ui.add_space(4.0);

        // Environment section
        Self::section_frame().show(ui, |ui| {
            egui::CollapsingHeader::new("\u{2600}  Environment")
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                if let Some(ibl) = persisted.scene.ibl.as_ref() {
                    ui.horizontal(|ui| {
                        Self::prop_label(ui, "IBL");
                        ui.add(
                            egui::Label::new(format!("{:?}", ibl))
                                .small()
                                .text_color(Self::muted_color()),
                        );
                    });
                    if Self::danger_button(ui, "Unload IBL").clicked() {
                        ctx.world_renderer.ibl.unload_image();
                        persisted.scene.ibl = None;
                    }
                } else {
                    ui.add(
                        egui::Label::new("Drag a .hdr/.exr to load as IBL")
                            .small()
                            .text_color(Self::text_dim()),
                    );
                }
            });
        });

        ui.add_space(2.0);

        // Exposure section
        Self::section_frame().show(ui, |ui| {
            egui::CollapsingHeader::new("\u{2699}  Exposure")
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
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
                    "Histogram low clip",
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
                    "Histogram high clip",
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
        });

        ui.add_space(2.0);

        // Lighting section
        Self::section_frame().show(ui, |ui| {
            egui::CollapsingHeader::new("\u{2728}  Lighting")
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
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
                ui.add(
                    egui::Label::new("Select Camera, Sun, or Local Lights for per-object controls.")
                        .small()
                        .text_color(Self::text_dim()),
                );
            });
        });

        ui.add_space(2.0);

        // Editor section
        Self::section_frame().show(ui, |ui| {
            egui::CollapsingHeader::new("\u{270E}  Editor")
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(2.0);
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
        });

        ui.add_space(6.0);
        ui.add(
            egui::Label::new("Select a GameObject to edit Transform & Components.")
                .small()
                .text_color(Self::text_dim()),
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
            // Name + ID header
            Self::section_frame().show(ui, |ui| {
                let mut name = game_object.name.clone();
                ui.horizontal(|ui| {
                    Self::prop_label(ui, "Name");
                    if ui.text_edit_singleline(&mut name).changed() {
                        game_object.name = GameObject::sanitized_name(game_object.id, name.trim());
                    }
                });

                ui.horizontal(|ui| {
                    Self::prop_label(ui, "ID");
                    ui.add(
                        egui::Label::new(format!("{}", game_object.id.0))
                            .text_color(Self::muted_color()),
                    );
                });

                ui.checkbox(&mut game_object.enabled, "Enabled");

                // Parent selector
                let mut selected_parent = game_object.parent;
                ui.horizontal(|ui| {
                    Self::prop_label(ui, "Parent");
                    egui::ComboBox::from_id_source("parent_combo")
                        .selected_text(&parent_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(selected_parent.is_none(), "Scene Root")
                                .clicked()
                            {
                                selected_parent = None;
                            }

                            for (candidate_id, candidate_name) in &parent_candidates {
                                if ui
                                    .selectable_label(
                                        selected_parent == Some(*candidate_id),
                                        candidate_name,
                                    )
                                    .clicked()
                                {
                                    selected_parent = Some(*candidate_id);
                                }
                            }
                        });
                });
                if selected_parent != game_object.parent {
                    pending_reparent = Some(selected_parent);
                }

                if let Some(builtin) = game_object.builtin {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        Self::badge(ui, "builtin");
                        ui.add(
                            egui::Label::new(builtin.display_name())
                                .small()
                                .text_color(Self::muted_color()),
                        );
                    });
                } else {
                    ui.add_space(4.0);
                    if Self::danger_button(ui, "Delete GameObject").clicked() {
                        delete_selected_game_object = true;
                    }
                }
            });

            ui.add_space(4.0);

            // Transform section
            Self::section_frame().show(ui, |ui| {
                egui::CollapsingHeader::new("\u{21C4}  Transform")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add_space(2.0);
                    Self::draw_transform_inspector(ui, &mut game_object.transform);
                });
            });

            ui.add_space(4.0);

            // Components
            if game_object.components.is_empty() {
                ui.add(
                    egui::Label::new("No Components attached.")
                        .small()
                        .text_color(Self::text_dim()),
                );
            }

            // Add Component section
            Self::section_frame().show(ui, |ui| {
                egui::CollapsingHeader::new("+  Add Component")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add_space(2.0);
                    if Self::game_object_has_scene_role_component(game_object) {
                        ui.add(
                            egui::Label::new(
                                "Already owns a Camera, Sun, or Local Lights component.",
                            )
                            .small()
                            .text_color(Self::text_dim()),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            if Self::accent_button(ui, "Camera").clicked() {
                                component_to_add =
                                    Some(SceneComponent::Camera(CameraComponent::default()));
                            }
                            if Self::accent_button(ui, "Sun").clicked() {
                                component_to_add =
                                    Some(SceneComponent::Sun(SunComponent::default()));
                            }
                            if Self::accent_button(ui, "Local Lights").clicked() {
                                component_to_add = Some(SceneComponent::LocalLights(
                                    LocalLightsComponent::default(),
                                ));
                            }
                        });
                    }
                    ui.add(
                        egui::Label::new("MeshRenderer is added via mesh import.")
                            .small()
                            .text_color(Self::text_dim()),
                    );
                });
            });

            ui.add_space(2.0);

            for (component_index, component) in game_object.components.iter_mut().enumerate() {
                Self::section_frame().show(ui, |ui| {
                    egui::CollapsingHeader::new(format!(
                            "\u{25A0}  {}",
                            component.kind_name()
                        ))
                    .id_source((game_object.id.0, component_index))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add_space(2.0);

                        let is_builtin_component = game_object
                            .builtin
                            .map(|builtin| builtin.matches_component(component))
                            .unwrap_or(false);

                        if is_builtin_component {
                            ui.horizontal(|ui| {
                                Self::badge(ui, "protected");
                                ui.add(
                                    egui::Label::new("scene singleton")
                                        .small()
                                        .text_color(Self::text_dim()),
                                );
                            });
                        } else if Self::danger_button(ui, "Remove").clicked() {
                            component_to_remove = Some(component_index);
                            component_to_remove_name = Some(component.kind_name().to_owned());
                        }

                        ui.add_space(2.0);
                        self.draw_component_inspector(ui, component);

                        if matches!(
                            component,
                            SceneComponent::Camera(_)
                                | SceneComponent::Sun(_)
                                | SceneComponent::LocalLights(_)
                        ) {
                            refresh_scene_singletons = true;
                        }
                    });
                });
                ui.add_space(2.0);
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
            Self::panel_title(ui, "Inspector");

            match self.selected_game_object {
                Some(selected_game_object) => {
                    self.draw_game_object_inspector(ui, persisted, selected_game_object);
                }
                None => self.draw_scene_inspector(ui, persisted, ctx),
            }

            ui.add_space(10.0);

            // Separator
            let sep_rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [
                    egui::pos2(sep_rect.left(), sep_rect.top()),
                    egui::pos2(sep_rect.right(), sep_rect.top()),
                ],
                egui::Stroke::new(1.0, Self::border_color()),
            );
            ui.add_space(6.0);

            // Renderer section
            Self::section_frame().show(ui, |ui| {
                egui::CollapsingHeader::new("\u{26A1}  Renderer")
                .default_open(true)
                .show(ui, |ui| {
                    self.draw_renderer_section(ui, persisted, ctx);
                });
            });

            ui.add_space(2.0);

            Self::section_frame().show(ui, |ui| {
                egui::CollapsingHeader::new("\u{25B6}  Sequence")
                .default_open(false)
                .show(ui, |ui| {
                    self.draw_sequence_section(ui, persisted);
                });
            });
        });
    }

    fn draw_renderer_section(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        // Performance stats bar
        ui.horizontal(|ui| {
            let cpu_ms = ctx.dt_filtered * 1000.0;
            let cpu_color = if cpu_ms > 16.0 {
                Self::warn_color()
            } else {
                Self::success_color()
            };
            ui.add(
                egui::Label::new(format!("CPU {:.2}ms", cpu_ms)).text_color(cpu_color),
            );

            if let Some(report) = gpu_profiler::profiler().last_report() {
                let gpu_time_ms: f64 = report.scopes.iter().map(|scope| scope.duration.ms()).sum();
                let gpu_color = if gpu_time_ms > 16.0 {
                    Self::warn_color()
                } else {
                    Self::success_color()
                };
                ui.add(
                    egui::Label::new(format!("GPU {:.2}ms", gpu_time_ms))
                        .text_color(gpu_color),
                );
            }

            ui.add(
                egui::Label::new(format!(
                    "{}x{}",
                    ctx.render_extent[0], ctx.render_extent[1]
                ))
                .small()
                .text_color(Self::text_dim()),
            );
        });
        ui.add_space(4.0);

        // Tweaks sub-section
        egui::CollapsingHeader::new("Tweaks")
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(2.0);
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

        // Overrides sub-section
        egui::CollapsingHeader::new("Overrides")
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(2.0);
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

        // Debug sub-section
        egui::CollapsingHeader::new("Debug")
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(2.0);
            ui.radio_value(
                &mut ctx.world_renderer.debug_mode,
                RenderDebugMode::None,
                "Scene geometry",
            );

            ui.horizontal(|ui| {
                Self::prop_label(ui, "Shading");
                egui::ComboBox::from_id_source("shading_mode")
                    .selected_text(match ctx.world_renderer.debug_shading_mode {
                        0 => "Default",
                        1 => "No base color",
                        2 => "Diffuse GI",
                        3 => "Reflections",
                        4 => "RTX OFF",
                        _ => "Irradiance cache",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            0usize,
                            "Default",
                        );
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            1usize,
                            "No base color",
                        );
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            2usize,
                            "Diffuse GI",
                        );
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            3usize,
                            "Reflections",
                        );
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            4usize,
                            "RTX OFF",
                        );
                        ui.selectable_value(
                            &mut ctx.world_renderer.debug_shading_mode,
                            5usize,
                            "Irradiance cache",
                        );
                    });
            });

            Self::drag_u32(ui, "Max FPS", &mut self.max_fps, 1, MAX_FPS_LIMIT);

            let mut allow_pass_overlap = unsafe { dist_render::rg::RG_ALLOW_PASS_OVERLAP };
            if ui
                .checkbox(&mut allow_pass_overlap, "Allow pass overlap")
                .changed()
            {
                unsafe {
                    dist_render::rg::RG_ALLOW_PASS_OVERLAP = allow_pass_overlap;
                }
            }
        });

        // GPU passes sub-section
        egui::CollapsingHeader::new("GPU Passes")
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(2.0);
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

                    // GPU pass row with timing
                    let pass_time_color = if scope.duration.ms() > 2.0 {
                        Self::warn_color()
                    } else {
                        Self::muted_color()
                    };

                    ui.horizontal(|ui| {
                        let response = ui.selectable_label(is_selected, &scope.name);

                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.add(
                                egui::Label::new(format!("{:.3}ms", scope.duration.ms()))
                                    .small()
                                    .text_color(pass_time_color),
                            );
                        });

                        if response.hovered() {
                            ctx.world_renderer.rg_debug_hook =
                                Some(dist_render::rg::GraphDebugHook {
                                    render_debug_hook: render_debug_hook.clone(),
                                });
                        }

                        if response.clicked() {
                            let current_hook =
                                Some(dist_render::rg::GraphDebugHook { render_debug_hook });
                            if self.locked_rg_debug_hook == current_hook {
                                self.locked_rg_debug_hook = None;
                            } else {
                                self.locked_rg_debug_hook = current_hook;
                            }
                        }
                    });
                }
            } else {
                ui.add(
                    egui::Label::new("No GPU profile data yet.")
                        .small()
                        .text_color(Self::text_dim()),
                );
            }
        });
    }

    fn draw_sequence_section(&mut self, ui: &mut egui::Ui, persisted: &mut PersistedState) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if Self::accent_button(ui, "+ Keyframe").clicked() {
                self.add_sequence_keyframe(persisted);
            }

            if self.is_sequence_playing() {
                if Self::danger_button(ui, "Stop").clicked() {
                    self.stop_sequence();
                }
            } else if Self::accent_button(ui, "\u{25B6} Play").clicked() {
                self.play_sequence(persisted);
            }

            if self.active_camera_key.is_some() && Self::subtle_button(ui, "Deselect").clicked() {
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

        ui.add_space(4.0);

        enum Command {
            JumpToKey(usize),
            DeleteKey(usize),
            ReplaceKey(usize),
            None,
        }

        let mut command = Command::None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            persisted.sequence.each_key(|i, item| {
                Self::section_frame().show(ui, |ui| {
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
                        if Self::subtle_button(ui, "Replace").clicked() {
                            command = Command::ReplaceKey(i);
                        }
                        if Self::danger_button(ui, "Delete").clicked() {
                            command = Command::DeleteKey(i);
                        }
                    });
                });
                ui.add_space(2.0);
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