use super::*;

impl RuntimeState {
    /// Returns a small icon string based on the primary component type of a game object.
    fn hierarchy_icon(game_object: &GameObject) -> &'static str {
        for component in &game_object.components {
            match component {
                SceneComponent::Camera(_) => return "\u{1F3A5}",  // camera
                SceneComponent::Sun(_) => return "\u{2600}",      // sun
                SceneComponent::LocalLights(_) => return "\u{1F4A1}", // light bulb
                SceneComponent::MeshRenderer(_) => return "\u{25A6}", // mesh icon
            }
        }
        "\u{25A1}" // empty square for plain game objects
    }

    fn draw_hierarchy_node(
        &mut self,
        ui: &mut egui::Ui,
        scene: &SceneState,
        game_object_id: GameObjectId,
        depth: usize,
    ) {
        let Some(game_object) = scene.find_game_object(game_object_id) else {
            return;
        };

        let child_ids: Vec<_> = scene
            .game_objects
            .iter()
            .filter(|child| child.parent == Some(game_object_id))
            .map(|child| child.id)
            .collect();

        let game_object_name = game_object.name.clone();
        let is_enabled = game_object.enabled;
        let icon = Self::hierarchy_icon(game_object);
        let has_children = !child_ids.is_empty();
        let selected = self.selected_game_object == Some(game_object_id);
        let expanded = self.hierarchy_expanded.contains(&game_object_id);

        // --- Row colors ---
        let name_color = if !is_enabled {
            Self::text_dim()
        } else if selected {
            Self::text_bright()
        } else {
            Self::text_normal()
        };

        let row_bg = if selected {
            Color32::from_rgba_premultiplied(66, 135, 245, 55)
        } else {
            Color32::TRANSPARENT
        };

        // --- Full-width row ---
        let row_rect = ui.available_rect_before_wrap();
        let row_height = 22.0;
        let (row_id, row_rect) = ui.allocate_space(egui::vec2(
            row_rect.width(),
            row_height,
        ));

        // Hover highlight
        let row_response = ui.interact(row_rect, row_id, egui::Sense::click());
        let bg = if row_response.hovered() && !selected {
            Color32::from_rgba_premultiplied(255, 255, 255, 10)
        } else {
            row_bg
        };
        if bg != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(row_rect, 2.0, bg);
        }

        // Selection left accent bar
        if selected {
            let bar_rect = egui::Rect::from_min_size(
                row_rect.left_top(),
                egui::vec2(2.5, row_height),
            );
            ui.painter()
                .rect_filled(bar_rect, 1.0, Self::accent_color());
        }

        let indent = depth as f32 * 18.0 + 6.0;
        let text_y = row_rect.center().y;

        // --- Expand/collapse arrow ---
        if has_children {
            let arrow_x = row_rect.left() + indent;

            let arrow_str = if expanded {
                "\u{25BE}" // ▾ down
            } else {
                "\u{25B8}" // ▸ right
            };

            let arrow_rect = egui::Rect::from_center_size(
                egui::pos2(arrow_x + 5.0, text_y),
                egui::vec2(16.0, row_height),
            );
            let arrow_response =
                ui.interact(arrow_rect, row_id.with("arrow"), egui::Sense::click());

            let arrow_color = if arrow_response.hovered() {
                Self::text_bright()
            } else {
                Self::text_dim()
            };

            ui.painter().text(
                egui::pos2(arrow_x, text_y),
                egui::Align2::LEFT_CENTER,
                arrow_str,
                egui::TextStyle::Body,
                arrow_color,
            );

            if arrow_response.clicked() {
                if expanded {
                    self.hierarchy_expanded.remove(&game_object_id);
                } else {
                    self.hierarchy_expanded.insert(game_object_id);
                }
            }
        }

        // --- Icon ---
        let icon_x = row_rect.left() + indent + if has_children { 16.0 } else { 4.0 };
        ui.painter().text(
            egui::pos2(icon_x, text_y),
            egui::Align2::LEFT_CENTER,
            icon,
            egui::TextStyle::Body,
            if is_enabled {
                Self::muted_color()
            } else {
                Self::text_dim()
            },
        );

        // --- Name ---
        let name_x = icon_x + 18.0;
        ui.painter().text(
            egui::pos2(name_x, text_y),
            egui::Align2::LEFT_CENTER,
            &game_object_name,
            egui::TextStyle::Body,
            name_color,
        );

        // --- Click to select ---
        if row_response.clicked() {
            self.selected_game_object = Some(game_object_id);
        }

        // --- Draw children if expanded ---
        if has_children && expanded {
            for child_id in child_ids {
                self.draw_hierarchy_node(ui, scene, child_id, depth + 1);
            }
        }
    }

    pub(super) fn draw_hierarchy_panel(
        &mut self,
        ui: &mut egui::Ui,
        persisted: &mut PersistedState,
    ) {
        // Panel header
        Self::panel_title(ui, "Hierarchy");

        ui.horizontal(|ui| {
            Self::badge(ui, &format!("{}", persisted.scene.game_objects.len()));
            ui.add(
                egui::Label::new("game objects")
                    .small()
                    .text_color(Self::muted_color()),
            );
        });
        ui.add_space(4.0);

        // Toolbar row
        Self::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let scene_selected = self.selected_game_object.is_none();
                if ui.selectable_label(scene_selected, "Scene Root").clicked() {
                    self.selected_game_object = None;
                }

                ui.separator();

                if Self::accent_button(ui, "+ Create Empty").clicked() {
                    let name =
                        format!("GameObject {}", persisted.scene.next_game_object_id);
                    let game_object_id = persisted
                        .scene
                        .create_game_object(name, SceneElementTransform::IDENTITY);
                    self.selected_game_object = Some(game_object_id);
                }
            });
        });

        ui.add_space(4.0);

        // Separator line
        let separator_rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [
                egui::pos2(separator_rect.left(), separator_rect.top()),
                egui::pos2(separator_rect.right(), separator_rect.top()),
            ],
            egui::Stroke::new(1.0, Self::border_color()),
        );
        ui.add_space(4.0);

        // Tree view
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
}