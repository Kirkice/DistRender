use super::*;

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

    pub(super) fn draw_viewport_panel(
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
        let focus_mask = drag_mask | (1 << 1);

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

                if self.viewport_hovered && (self.mouse.buttons_pressed & focus_mask) != 0 {
                    self.viewport_keyboard_focused = true;
                }

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
}