use super::*;

struct ProjectedViewportObject {
    game_object_id: GameObjectId,
    world_position: Vec3,
    screen_position: egui::Pos2,
    depth: f32,
    /// Screen-space AABB for mesh objects (for click-to-select).
    screen_aabb: Option<(egui::Pos2, egui::Pos2)>,
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

    /// Project scene objects including AABB-based screen bounding boxes for mesh objects.
    fn project_scene_objects(
        &self,
        scene: &SceneState,
        world_renderer: &WorldRenderer,
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

                // Compute screen-space AABB for mesh objects
                let screen_aabb = self
                    .runtime_scene
                    .game_object_world_aabb(scene, world_renderer, game_object.id)
                    .and_then(|(w_min, w_max)| {
                        let w_min = Vec3::from(w_min);
                        let w_max = Vec3::from(w_max);
                        let corners = [
                            Vec3::new(w_min.x, w_min.y, w_min.z),
                            Vec3::new(w_max.x, w_min.y, w_min.z),
                            Vec3::new(w_min.x, w_max.y, w_min.z),
                            Vec3::new(w_min.x, w_min.y, w_max.z),
                            Vec3::new(w_max.x, w_max.y, w_min.z),
                            Vec3::new(w_max.x, w_min.y, w_max.z),
                            Vec3::new(w_min.x, w_max.y, w_max.z),
                            Vec3::new(w_max.x, w_max.y, w_max.z),
                        ];

                        let mut s_min = egui::pos2(f32::MAX, f32::MAX);
                        let mut s_max = egui::pos2(f32::MIN, f32::MIN);
                        let mut any_visible = false;
                        for c in &corners {
                            if let Some((sp, _)) = Self::project_world_point(
                                viewport_rect,
                                world_to_clip,
                                world_to_view,
                                *c,
                            ) {
                                s_min.x = s_min.x.min(sp.x);
                                s_min.y = s_min.y.min(sp.y);
                                s_max.x = s_max.x.max(sp.x);
                                s_max.y = s_max.y.max(sp.y);
                                any_visible = true;
                            }
                        }

                        if any_visible {
                            Some((s_min, s_max))
                        } else {
                            None
                        }
                    });

                Some(ProjectedViewportObject {
                    game_object_id: game_object.id,
                    world_position,
                    screen_position,
                    depth,
                    screen_aabb,
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

    /// Pick a game object at the given pointer position, preferring AABB hits over origin-dot hits.
    fn pick_game_object_at(
        projected_objects: &[ProjectedViewportObject],
        pointer_position: egui::Pos2,
    ) -> Option<GameObjectId> {
        // First try AABB-based picking (mesh objects)
        let aabb_hit = projected_objects
            .iter()
            .filter_map(|projected| {
                let (s_min, s_max) = projected.screen_aabb?;
                let inside = pointer_position.x >= s_min.x
                    && pointer_position.x <= s_max.x
                    && pointer_position.y >= s_min.y
                    && pointer_position.y <= s_max.y;
                if inside {
                    Some((projected.game_object_id, projected.depth))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id);

        if aabb_hit.is_some() {
            return aabb_hit;
        }

        // Fallback: origin-dot picking for non-mesh objects
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
            ViewportGizmoAxis::X => Self::axis_x_color(),
            ViewportGizmoAxis::Y => Self::axis_y_color(),
            ViewportGizmoAxis::Z => Self::axis_z_color(),
        }
    }

    fn gizmo_mode_label(mode: GizmoMode) -> &'static str {
        match mode {
            GizmoMode::Translate => "W  Translate",
            GizmoMode::Rotate => "E  Rotate",
            GizmoMode::Scale => "R  Scale",
        }
    }

    // --- Rotation gizmo arc points in screen space ---
    fn gizmo_rotation_arc_points(
        viewport_rect: egui::Rect,
        world_to_clip: Mat4,
        world_to_view: Mat4,
        world_position: Vec3,
        axis: ViewportGizmoAxis,
        arc_radius_world: f32,
        segments: usize,
    ) -> Vec<egui::Pos2> {
        let (tangent1, tangent2) = match axis {
            ViewportGizmoAxis::X => (Vec3::Y, Vec3::Z),
            ViewportGizmoAxis::Y => (Vec3::X, Vec3::Z),
            ViewportGizmoAxis::Z => (Vec3::X, Vec3::Y),
        };

        (0..=segments)
            .filter_map(|i| {
                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                let point = world_position
                    + tangent1 * (angle.cos() * arc_radius_world)
                    + tangent2 * (angle.sin() * arc_radius_world);
                Self::project_world_point(viewport_rect, world_to_clip, world_to_view, point)
                    .map(|(sp, _)| sp)
            })
            .collect()
    }

    fn screen_distance_to_arc(
        point: egui::Pos2,
        arc_points: &[egui::Pos2],
    ) -> f32 {
        let mut min_dist = f32::MAX;
        for window in arc_points.windows(2) {
            let dist = Self::screen_distance_to_segment(point, window[0], window[1]);
            min_dist = min_dist.min(dist);
        }
        min_dist
    }

    // --- Handle gizmo drag update for all modes ---
    fn handle_gizmo_drag(
        scene: &mut SceneState,
        gizmo_mode: GizmoMode,
        drag_state: &ViewportGizmoDragState,
        pointer_position: egui::Pos2,
    ) {
        let pointer_delta =
            Vec2::new(pointer_position.x, pointer_position.y) - drag_state.pointer_origin;

        match gizmo_mode {
            GizmoMode::Translate => {
                let axis_distance = pointer_delta.dot(drag_state.screen_direction);
                let world_offset =
                    drag_state.axis.world_vector() * (axis_distance / drag_state.pixels_per_unit);
                Self::set_game_object_world_position(
                    scene,
                    drag_state.game_object_id,
                    drag_state.start_world_position + world_offset,
                );
            }
            GizmoMode::Rotate => {
                // Horizontal drag → rotation in degrees (180 px = 90°)
                let axis_distance = pointer_delta.dot(drag_state.screen_direction);
                let degrees = axis_distance * 0.5; // 0.5 deg per pixel

                if let Some(game_object) = scene.find_game_object_mut(drag_state.game_object_id) {
                    let mut euler = drag_state.start_rotation_euler;
                    match drag_state.axis {
                        ViewportGizmoAxis::X => euler.x += degrees,
                        ViewportGizmoAxis::Y => euler.y += degrees,
                        ViewportGizmoAxis::Z => euler.z += degrees,
                    }
                    game_object.transform.rotation_euler_degrees = euler;
                }
            }
            GizmoMode::Scale => {
                // Drag along axis direction → scale multiplier
                let axis_distance = pointer_delta.dot(drag_state.screen_direction);
                let scale_factor = (1.0 + axis_distance * 0.005).max(0.001); // 0.5% per pixel

                if let Some(game_object) = scene.find_game_object_mut(drag_state.game_object_id) {
                    let mut scale = drag_state.start_scale;
                    match drag_state.axis {
                        ViewportGizmoAxis::X => scale.x *= scale_factor,
                        ViewportGizmoAxis::Y => scale.y *= scale_factor,
                        ViewportGizmoAxis::Z => scale.z *= scale_factor,
                    }
                    scale = scale.max(Vec3::splat(0.001)).min(Vec3::splat(1000.0));
                    game_object.transform.scale = scale;
                }
            }
        }
    }

    pub(super) fn draw_viewport_panel(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        // --- Gizmo mode shortcuts (W/E/R like Unity) ---
        if self.viewport_keyboard_focused {
            if self.keyboard.was_just_pressed(VirtualKeyCode::W) {
                self.gizmo_mode = GizmoMode::Translate;
            }
            if self.keyboard.was_just_pressed(VirtualKeyCode::E) {
                self.gizmo_mode = GizmoMode::Rotate;
            }
            if self.keyboard.was_just_pressed(VirtualKeyCode::R) {
                self.gizmo_mode = GizmoMode::Scale;
            }
        }

        // Viewport toolbar
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let bar_height = ui.fonts().row_height(egui::TextStyle::Heading);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(3.0, bar_height),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 1.5, Self::accent_color());
            ui.add_space(4.0);
            ui.add(
                egui::Label::new("Viewport")
                    .heading()
                    .text_color(Self::text_bright()),
            );
            ui.add_space(8.0);

            // Gizmo mode selector buttons
            for mode in &[GizmoMode::Translate, GizmoMode::Rotate, GizmoMode::Scale] {
                let is_active = self.gizmo_mode == *mode;
                let btn = egui::Button::new(Self::gizmo_mode_label(*mode))
                    .fill(if is_active {
                        Self::accent_dim()
                    } else {
                        Color32::from_rgb(42, 42, 56)
                    })
                    .text_color(if is_active {
                        Color32::WHITE
                    } else {
                        Self::text_normal()
                    });
                if ui.add(btn).clicked() {
                    self.gizmo_mode = *mode;
                }
                ui.add_space(2.0);
            }

            ui.add_space(8.0);
            ui.add(
                egui::Label::new(format!("{}x{}", ctx.render_extent[0], ctx.render_extent[1]))
                    .small()
                    .text_color(Self::text_dim()),
            );
        });
        // Thin separator line
        let sep_rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [
                egui::pos2(sep_rect.left(), sep_rect.top()),
                egui::pos2(sep_rect.right(), sep_rect.top()),
            ],
            egui::Stroke::new(1.0, Self::border_color()),
        );
        ui.add_space(2.0);

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

        let gizmo_mode = self.gizmo_mode;

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
                let projected_objects = self.project_scene_objects(
                    &persisted.scene,
                    ctx.world_renderer,
                    viewport_rect,
                    world_to_clip,
                    world_to_view,
                );
                let selected_projection = self.selected_game_object.and_then(|selected| {
                    projected_objects
                        .iter()
                        .find(|projected| projected.game_object_id == selected)
                });

                let selected_world_pos = selected_projection.map(|s| s.world_position);

                // Compute gizmo visuals based on current mode
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

                let arc_radius_world = selected_world_pos
                    .map(|wp| eye_position.distance(wp).mul_add(0.12, 0.0).clamp(0.4, 4.0))
                    .unwrap_or(1.0);

                let pointer_position = ui.input().pointer.interact_pos();
                let primary_down = ui.input().pointer.button_down(egui::PointerButton::Primary);
                let primary_pressed = ui.input().pointer.any_pressed() && primary_down;
                let primary_released = ui.input().pointer.any_released() && !primary_down;

                // --- Begin gizmo interaction ---
                if self.viewport_hovered && primary_pressed {
                    if let Some(pointer_position) = pointer_position {
                        const AXIS_PICK_RADIUS: f32 = 10.0;

                        let axis_hit = match gizmo_mode {
                            GizmoMode::Translate | GizmoMode::Scale => {
                                // Pick axis handle lines
                                gizmo_axes
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
                                    .min_by(|a, b| {
                                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                    .map(|(axis, _)| axis)
                            }
                            GizmoMode::Rotate => {
                                // Pick rotation arcs
                                if let Some(wp) = selected_world_pos {
                                    let mut best: Option<(&ProjectedGizmoAxis, f32)> = None;
                                    for axis_proj in &gizmo_axes {
                                        let arc_points = Self::gizmo_rotation_arc_points(
                                            viewport_rect,
                                            world_to_clip,
                                            world_to_view,
                                            wp,
                                            axis_proj.axis,
                                            arc_radius_world,
                                            32,
                                        );
                                        let dist = Self::screen_distance_to_arc(pointer_position, &arc_points);
                                        if dist <= AXIS_PICK_RADIUS {
                                            if best.is_none() || dist < best.unwrap().1 {
                                                best = Some((axis_proj, dist));
                                            }
                                        }
                                    }
                                    best.map(|(a, _)| a)
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some(axis) = axis_hit {
                            if let Some(selected) = selected_projection {
                                let go = persisted.scene.find_game_object(selected.game_object_id);
                                let (start_rotation_euler, start_scale) = go
                                    .map(|g| (g.transform.rotation_euler_degrees, g.transform.scale))
                                    .unwrap_or((Vec3::ZERO, Vec3::ONE));

                                self.viewport_gizmo_drag = Some(ViewportGizmoDragState {
                                    game_object_id: selected.game_object_id,
                                    axis: axis.axis,
                                    pointer_origin: Vec2::new(pointer_position.x, pointer_position.y),
                                    screen_direction: axis.screen_direction,
                                    start_world_position: selected.world_position,
                                    pixels_per_unit: axis.pixels_per_unit.max(1e-4),
                                    start_rotation_euler,
                                    start_scale,
                                });
                                self.viewport_click_origin = None;
                            }
                        } else {
                            self.viewport_click_origin = Some(Vec2::new(pointer_position.x, pointer_position.y));
                        }
                    }
                }

                // --- Gizmo drag update ---
                if let Some(drag_state) = self.viewport_gizmo_drag {
                    if primary_down {
                        if let Some(pointer_position) = pointer_position {
                            Self::handle_gizmo_drag(
                                &mut persisted.scene,
                                gizmo_mode,
                                &drag_state,
                                pointer_position,
                            );
                        }
                    } else {
                        self.viewport_gizmo_drag = None;
                    }
                }

                // --- Click-to-select ---
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

                // =================================================================
                // Drawing
                // =================================================================
                let painter = ui.painter_at(viewport_rect);

                // Draw object dots and selection highlight
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

                        // Draw screen-space AABB wireframe for selected mesh objects
                        if let Some((s_min, s_max)) = projected.screen_aabb {
                            let rect = egui::Rect::from_min_max(s_min, s_max);
                            painter.rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(61, 180, 255, 100)),
                            );
                        }
                    }
                }

                // Draw gizmo based on mode
                let active_axis = self.viewport_gizmo_drag.map(|d| d.axis);

                match gizmo_mode {
                    GizmoMode::Translate => {
                        // Arrow-style lines + arrow tip circles
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
                    }
                    GizmoMode::Rotate => {
                        // Draw circular arc for each axis
                        if let Some(wp) = selected_world_pos {
                            for axis_proj in &gizmo_axes {
                                let color = if active_axis == Some(axis_proj.axis) {
                                    Color32::WHITE
                                } else {
                                    Self::axis_color(axis_proj.axis)
                                };
                                let arc_points = Self::gizmo_rotation_arc_points(
                                    viewport_rect,
                                    world_to_clip,
                                    world_to_view,
                                    wp,
                                    axis_proj.axis,
                                    arc_radius_world,
                                    48,
                                );
                                for window in arc_points.windows(2) {
                                    painter.line_segment(
                                        [window[0], window[1]],
                                        egui::Stroke::new(2.0, color),
                                    );
                                }
                            }
                        }
                    }
                    GizmoMode::Scale => {
                        // Lines + box endpoints
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
                            // Box endpoint (filled rect)
                            let box_half = 4.0;
                            let box_rect = egui::Rect::from_center_size(
                                axis.screen_end,
                                egui::vec2(box_half * 2.0, box_half * 2.0),
                            );
                            painter.rect_filled(box_rect, 1.0, color);
                        }
                    }
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