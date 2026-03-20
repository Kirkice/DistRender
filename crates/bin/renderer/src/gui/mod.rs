mod hierarchy;
mod inspector;
mod viewport;

pub(super) use dist_render::RenderOverrideFlags;
pub(super) use dist_render_simple::*;
pub(super) use egui::{self, Color32};

pub(super) use crate::{
    runtime::{
        component::{
            CameraComponent, GameObject, GameObjectId, LocalLightsComponent, SceneComponent,
            SceneElementTransform, SceneState, SunComponent,
        },
        RuntimeState, ViewportGizmoAxis, ViewportGizmoDragState, MAX_CAMERA_SPEED,
        MAX_FPS_LIMIT, MIN_CAMERA_SPEED,
    },
    PersistedState,
};

impl RuntimeState {
    fn chrome_fill() -> Color32 {
        Color32::from_rgb(14, 18, 26)
    }

    fn panel_fill() -> Color32 {
        Color32::from_rgb(16, 22, 31)
    }

    fn viewport_fill() -> Color32 {
        Color32::from_rgb(6, 9, 14)
    }

    fn warn_color() -> Color32 {
        Color32::from_rgb(255, 210, 96)
    }

    fn muted_color() -> Color32 {
        Color32::from_rgb(146, 158, 174)
    }

    fn validate_selected_game_object(&mut self, scene: &SceneState) {
        if let Some(selected_game_object) = self.selected_game_object {
            if scene.find_game_object(selected_game_object).is_none() {
                self.selected_game_object = None;
            }
        }

        if let Some(drag_state) = self.viewport_gizmo_drag {
            if scene.find_game_object(drag_state.game_object_id).is_none() {
                self.viewport_gizmo_drag = None;
            }
        }
    }

    fn drag_f32(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut f32,
        speed: f64,
        min: f32,
        max: f32,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(
                egui::DragValue::new(value)
                    .speed(speed)
                    .clamp_range(min..=max),
            );
        });
    }

    fn drag_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, min: u32, max: u32) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).speed(1.0).clamp_range(min..=max));
        });
    }

    fn drag_triplet(
        ui: &mut egui::Ui,
        label: &str,
        values: [&mut f32; 3],
        speed: f64,
        min: f32,
        max: f32,
    ) {
        ui.label(label);
        ui.horizontal(|ui| {
            for (axis, value) in ["x", "y", "z"].into_iter().zip(values.into_iter()) {
                ui.add(
                    egui::DragValue::new(value)
                        .prefix(format!("{} ", axis))
                        .speed(speed)
                        .clamp_range(min..=max),
                );
            }
        });
    }

    fn draw_menu_bar(
        &mut self,
        egui_ctx: &egui::CtxRef,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        egui::TopBottomPanel::top("main_menu_bar")
            .height_range(34.0..=34.0)
            .frame(egui::Frame::none().fill(Self::chrome_fill()))
            .show(egui_ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "Create", |ui| {
                        if ui.button("Empty GameObject").clicked() {
                            let name = format!("GameObject {}", persisted.scene.next_game_object_id);
                            let game_object_id = persisted
                                .scene
                                .create_game_object(name, SceneElementTransform::IDENTITY);
                            self.selected_game_object = Some(game_object_id);
                        }
                    });

                    egui::menu::menu(ui, "Playback", |ui| {
                        if ui.button("Add Keyframe").clicked() {
                            self.add_sequence_keyframe(persisted);
                        }

                        if self.is_sequence_playing() {
                            if ui.button("Stop").clicked() {
                                self.stop_sequence();
                            }
                        } else if ui.button("Play").clicked() {
                            self.play_sequence(persisted);
                        }
                    });

                    if ui.button("Scene Root").clicked() {
                        self.selected_game_object = None;
                    }

                    ui.separator();
                    ui.colored_label(
                        Self::muted_color(),
                        format!("CPU {:.2} ms", ctx.dt_filtered * 1000.0),
                    );

                    if let Some(report) = gpu_profiler::profiler().last_report() {
                        let gpu_time_ms: f64 = report.scopes.iter().map(|scope| scope.duration.ms()).sum();
                        ui.colored_label(Self::muted_color(), format!("GPU {:.2} ms", gpu_time_ms));
                    }

                    ui.colored_label(
                        Self::muted_color(),
                        format!("Viewport {}x{}", ctx.render_extent[0], ctx.render_extent[1]),
                    );
                });
            });
    }

    pub fn do_gui(&mut self, persisted: &mut PersistedState, ctx: &mut FrameContext) {
        if self.keyboard.was_just_pressed(self.keymap_config.ui.toggle) {
            self.show_gui = !self.show_gui;
        }

        ctx.world_renderer.rg_debug_hook = self.locked_rg_debug_hook.clone();
        self.viewport_hovered = false;

        if !self.show_gui {
            self.viewport_pointer_captured = false;
        }

        if self.show_gui {
            ctx.egui.take().unwrap().frame(|egui_ctx| {
                self.validate_selected_game_object(&persisted.scene);
                self.draw_menu_bar(egui_ctx, persisted, ctx);

                egui::SidePanel::left("hierarchy_panel")
                    .default_width(320.0)
                    .resizable(true)
                    .width_range(220.0..=720.0)
                    .frame(egui::Frame::none().fill(Self::panel_fill()))
                    .show(egui_ctx, |ui| {
                        self.draw_hierarchy_panel(ui, persisted);
                    });

                egui::SidePanel::right("inspector_panel")
                    .default_width(460.0)
                    .resizable(true)
                    .width_range(300.0..=820.0)
                    .frame(egui::Frame::none().fill(Self::panel_fill()))
                    .show(egui_ctx, |ui| {
                        self.draw_inspector_panel(ui, persisted, ctx);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Self::chrome_fill()))
                    .show(egui_ctx, |ui| {
                        self.draw_viewport_panel(ui, persisted, ctx);
                    });
            });

            if (self.mouse.buttons_pressed & 0b111) != 0
                && !self.viewport_hovered
                && !self.viewport_pointer_captured
            {
                self.viewport_keyboard_focused = false;
            }
        } else {
            self.viewport_hovered = true;
            ctx.render_extent = [ctx.window.inner_size().width.max(1), ctx.window.inner_size().height.max(1)];
            self.viewport_keyboard_focused = false;
            self.viewport_click_origin = None;
            self.viewport_gizmo_drag = None;
        }
    }
}