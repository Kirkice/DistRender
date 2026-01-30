//! 后端切换面板
//!
//! 提供图形后端切换功能（需要重启应用）。

use egui;
use crate::gui::state::GuiState;

/// 渲染后端切换面板
pub fn render(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.collapsing("Graphics Backend", |ui| {
        ui.label(format!("Current Backend: {}", state.current_backend));

        ui.label("Select Backend:");
        egui::ComboBox::from_label("")
            .selected_text(&state.selected_backend)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.selected_backend, "vulkan".to_string(), "Vulkan");
                ui.selectable_value(&mut state.selected_backend, "dx12".to_string(), "DirectX 12");
                ui.selectable_value(&mut state.selected_backend, "wgpu".to_string(), "wgpu");
            });

        if state.selected_backend != state.current_backend {
            ui.colored_label(egui::Color32::YELLOW, "⚠ Restart required to apply backend change");
            if ui.button("Apply & Exit").clicked() {
                state.backend_changed = true;
            }
        }
    });
}
