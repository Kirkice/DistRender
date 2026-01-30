//! 渲染设置面板
//!
//! 提供清除颜色、光照强度、光照方向、相机 FOV 等渲染参数的调整。

use egui;
use crate::gui::state::GuiState;

/// 渲染渲染设置面板
pub fn render(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.collapsing("Rendering", |ui| {
        ui.label("Clear Color:");
        ui.horizontal(|ui| {
            ui.color_edit_button_rgba_unmultiplied(&mut state.clear_color);
        });

        ui.label("Light Intensity:");
        ui.add(egui::Slider::new(&mut state.light_intensity, 0.0..=5.0));

        ui.label("Light Direction:");
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::DragValue::new(&mut state.light_direction[0]).speed(0.1));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut state.light_direction[1]).speed(0.1));
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut state.light_direction[2]).speed(0.1));
        });

        ui.label("Camera FOV:");
        ui.add(egui::Slider::new(&mut state.camera_fov, 30.0..=120.0).suffix("°"));
    });
}
