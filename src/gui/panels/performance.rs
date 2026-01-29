//! 性能监控面板
//!
//! 显示 FPS、帧时间等性能指标。

use egui;
use crate::gui::state::GuiState;

/// 渲染性能面板
pub fn render(ui: &mut egui::Ui, state: &GuiState) {
    ui.collapsing("性能监控", |ui| {
        ui.label(format!("FPS: {:.1}", state.fps));
        ui.label(format!("帧时间: {:.2} ms", state.frame_time_ms));

        if state.frame_time_ms > 0.0 {
            let target_60fps = 1000.0 / 60.0;
            let color = if state.frame_time_ms <= target_60fps {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };

            ui.colored_label(color,
                if state.frame_time_ms <= target_60fps {
                    "✓ 性能良好"
                } else {
                    "⚠ 性能警告"
                }
            );
        }
    });
}
