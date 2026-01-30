//! 场景控制面板
//!
//! 提供模型位置、旋转、缩放等场景参数的调整。

use egui;
use crate::gui::state::GuiState;

/// 渲染场景控制面板
pub fn render(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.collapsing("Scene", |ui| {
        ui.label("Model Position:");
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::DragValue::new(&mut state.model_position[0]).speed(0.1));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut state.model_position[1]).speed(0.1));
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut state.model_position[2]).speed(0.1));
        });

        ui.label("Model Rotation (deg):");
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::DragValue::new(&mut state.model_rotation[0]).speed(1.0));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut state.model_rotation[1]).speed(1.0));
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut state.model_rotation[2]).speed(1.0));
        });

        ui.label("Model Scale:");
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::DragValue::new(&mut state.model_scale[0]).speed(0.1));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut state.model_scale[1]).speed(0.1));
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut state.model_scale[2]).speed(0.1));
        });

        if ui.button("Reset Transform").clicked() {
            state.model_position = [0.0, 0.0, 0.0];
            state.model_rotation = [0.0, 0.0, 0.0];
            state.model_scale = [1.0, 1.0, 1.0];
        }
    });
}
