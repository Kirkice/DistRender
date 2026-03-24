use super::*;

impl RuntimeState {
    fn game_object_component_summary(game_object: &GameObject) -> String {
        game_object
            .components
            .iter()
            .map(|component| component.kind_name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn draw_hierarchy_node(
        &mut self,
        ui: &mut egui::Ui,
        scene: &SceneState,
        game_object_id: GameObjectId,
        depth: usize,
    ) {
        let Some((game_object_name, component_summary, child_ids, is_builtin, is_enabled)) = scene
            .find_game_object(game_object_id)
            .map(|game_object| {
                let child_ids: Vec<_> = scene
                    .game_objects
                    .iter()
                    .filter(|child| child.parent == Some(game_object_id))
                    .map(|child| child.id)
                    .collect();

                (
                    game_object.name.clone(),
                    Self::game_object_component_summary(game_object),
                    child_ids,
                    game_object.is_builtin(),
                    game_object.enabled,
                )
            })
        else {
            return;
        };

        let selected = self.selected_game_object == Some(game_object_id);
        let has_children = !child_ids.is_empty();

        // Row with subtle alternating/hover background
        let row_frame = if selected {
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(66, 135, 245, 45))
                .corner_radius(3.0)
                .margin(egui::vec2(2.0, 1.0))
        } else {
            egui::Frame::none()
                .corner_radius(3.0)
                .margin(egui::vec2(2.0, 1.0))
        };

        row_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 18.0);

                // Tree connector
                let tree_icon = if has_children { "\u{25B8}" } else { "  " };
                ui.add(egui::Label::new(tree_icon).text_color(Self::text_dim()));

                // Enabled/disabled indicator
                let name_color = if !is_enabled {
                    Self::text_dim()
                } else if selected {
                    Self::text_bright()
                } else {
                    Self::text_normal()
                };

                let response = ui.selectable_label(selected, "");
                // Draw the name manually for the color
                let name_rect = response.rect;
                ui.painter().text(
                    name_rect.left_center() + egui::vec2(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &game_object_name,
                    egui::TextStyle::Body,
                    name_color,
                );

                if response.clicked() {
                    self.selected_game_object = Some(game_object_id);
                }

                // Component tags
                if !component_summary.is_empty() {
                    ui.add(
                        egui::Label::new(component_summary)
                            .small()
                            .text_color(Self::muted_color()),
                    );
                }

                if is_builtin {
                    Self::badge(ui, "builtin");
                }

                if !is_enabled {
                    ui.add(
                        egui::Label::new("off")
                            .small()
                            .text_color(Self::text_dim()),
                    );
                }
            });
        });

        for child_id in child_ids {
            self.draw_hierarchy_node(ui, scene, child_id, depth + 1);
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