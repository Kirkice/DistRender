//! 后端切换面板
//!
//! 提供图形后端切换功能（需要重启应用）。

use egui;
use crate::gui::state::GuiState;

/// 渲染后端切换面板
pub fn render(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.collapsing("图形后端", |ui| {
        ui.label(format!("当前后端: {}", state.current_backend));

        ui.label("选择后端:");
        egui::ComboBox::from_label("")
            .selected_text(&state.selected_backend)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.selected_backend, "vulkan".to_string(), "Vulkan");
                ui.selectable_value(&mut state.selected_backend, "dx12".to_string(), "DirectX 12");
                ui.selectable_value(&mut state.selected_backend, "wgpu".to_string(), "wgpu");
            });

        if state.selected_backend != state.current_backend {
            ui.colored_label(egui::Color32::YELLOW, "⚠ 需要重启应用以应用后端更改");
            if ui.button("应用并退出").clicked() {
                state.backend_changed = true;
            }
        }
    });
}
