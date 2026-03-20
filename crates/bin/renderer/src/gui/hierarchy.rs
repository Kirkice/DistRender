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

    pub(super) fn draw_hierarchy_panel(&mut self, ui: &mut egui::Ui, persisted: &mut PersistedState) {
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
}