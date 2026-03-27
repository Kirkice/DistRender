mod console;
mod hierarchy;
mod inspector;
mod viewport;

pub(super) use self::console::{ConsoleState, ConsoleFilter};

pub(super) use dist_render::RenderOverrideFlags;
pub(super) use dist_render::world_renderer::{
    WorldRenderer, MATERIAL_MAP_NAMES,
};
pub(super) use dist_render_simple::*;
pub(super) use egui::{self, Color32};

pub(super) use crate::{
    runtime::{
        component::{
            CameraComponent, GameObject, GameObjectId, LocalLightsComponent, SceneComponent,
            SceneElementTransform, SceneState, SunComponent,
        },
        RuntimeState, ViewportGizmoAxis, ViewportGizmoDragState, GizmoMode, MAX_CAMERA_SPEED,
        MAX_FPS_LIMIT, MIN_CAMERA_SPEED,
    },
    PersistedState,
};

// ---------------------------------------------------------------------------
// Modern editor theme (Unity / Unreal inspired)
// ---------------------------------------------------------------------------

impl RuntimeState {
    // -- Base surface colors ------------------------------------------------
    fn chrome_fill() -> Color32 {
        Color32::from_rgb(22, 22, 30)
    }

    fn panel_fill() -> Color32 {
        Color32::from_rgb(25, 25, 34)
    }

    fn section_bg() -> Color32 {
        Color32::from_rgb(32, 32, 42)
    }

    fn viewport_fill() -> Color32 {
        Color32::from_rgb(8, 8, 14)
    }

    fn border_color() -> Color32 {
        Color32::from_rgb(48, 48, 62)
    }

    fn border_subtle() -> Color32 {
        Color32::from_rgb(38, 38, 52)
    }

    // -- Accent / interactive -----------------------------------------------
    fn accent_color() -> Color32 {
        Color32::from_rgb(66, 135, 245)
    }

    fn accent_dim() -> Color32 {
        Color32::from_rgb(45, 100, 200)
    }

    // -- Text ---------------------------------------------------------------
    fn text_bright() -> Color32 {
        Color32::from_rgb(230, 233, 240)
    }

    fn text_normal() -> Color32 {
        Color32::from_rgb(190, 195, 210)
    }

    fn muted_color() -> Color32 {
        Color32::from_rgb(120, 126, 148)
    }

    fn text_dim() -> Color32 {
        Color32::from_rgb(80, 86, 108)
    }

    // -- Status -------------------------------------------------------------
    fn warn_color() -> Color32 {
        Color32::from_rgb(255, 186, 66)
    }

    fn success_color() -> Color32 {
        Color32::from_rgb(72, 210, 120)
    }

    // -- Axis colors (X=Red, Y=Green, Z=Blue like Unity) --------------------
    fn axis_x_color() -> Color32 {
        Color32::from_rgb(235, 70, 80)
    }

    fn axis_y_color() -> Color32 {
        Color32::from_rgb(80, 200, 96)
    }

    fn axis_z_color() -> Color32 {
        Color32::from_rgb(66, 140, 255)
    }

    // -- Global egui style --------------------------------------------------
    fn apply_modern_style(ctx: &egui::CtxRef) {
        let mut style = (*ctx.style()).clone();

        // Spacing
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.window_padding = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.indent = 20.0;
        style.spacing.interact_size = egui::vec2(40.0, 22.0);
        style.spacing.slider_width = 140.0;
        style.spacing.scroll_bar_width = 7.0;

        // Visuals
        let mut v = egui::Visuals::dark();
        v.window_corner_radius = 6.0;

        // Non-interactive (labels, backgrounds)
        v.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 30, 40);
        v.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(190, 195, 210));
        v.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(0.5, Color32::from_rgb(48, 48, 62));
        v.widgets.noninteractive.corner_radius = 3.0;

        // Inactive widgets (buttons at rest, checkboxes, etc.)
        v.widgets.inactive.bg_fill = Color32::from_rgb(42, 42, 56);
        v.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(175, 180, 198));
        v.widgets.inactive.bg_stroke =
            egui::Stroke::new(0.5, Color32::from_rgb(56, 56, 72));
        v.widgets.inactive.corner_radius = 4.0;

        // Hovered widgets
        v.widgets.hovered.bg_fill = Color32::from_rgb(52, 52, 72);
        v.widgets.hovered.fg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(225, 228, 238));
        v.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(66, 135, 245));
        v.widgets.hovered.corner_radius = 4.0;

        // Active / pressed widgets
        v.widgets.active.bg_fill = Color32::from_rgb(66, 135, 245);
        v.widgets.active.fg_stroke = egui::Stroke::new(1.5, Color32::WHITE);
        v.widgets.active.bg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(100, 160, 255));
        v.widgets.active.corner_radius = 4.0;

        // Open widgets (combo-box, menu, etc.)
        v.widgets.open.bg_fill = Color32::from_rgb(38, 38, 52);
        v.widgets.open.fg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(200, 204, 218));
        v.widgets.open.bg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(66, 135, 245));
        v.widgets.open.corner_radius = 4.0;

        // Selection highlight
        v.selection.bg_fill = Color32::from_rgba_premultiplied(66, 135, 245, 55);
        v.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(66, 135, 245));

        v.extreme_bg_color = Color32::from_rgb(16, 16, 22);
        v.code_bg_color = Color32::from_rgb(22, 22, 30);

        style.visuals = v;
        ctx.set_style(style);
    }

    // -- Reusable frame for "card" sections ---------------------------------
    fn section_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Self::section_bg())
            .corner_radius(5.0)
            .margin(egui::vec2(8.0, 6.0))
            .stroke(egui::Stroke::new(1.0, Self::border_subtle()))
    }

    // -- Panel title with accent bar ----------------------------------------
    fn panel_title(ui: &mut egui::Ui, title: &str) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // Accent bar height matches heading font so bar and text are the same height
            let bar_height = ui.fonts().row_height(egui::TextStyle::Heading);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(3.0, bar_height),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 1.5, Self::accent_color());
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(title)
                    .heading()
                    .text_color(Self::text_bright()),
            );
        });
        ui.add_space(2.0);
    }

    // -- Inline badge -------------------------------------------------------
    fn badge(ui: &mut egui::Ui, text: &str) {
        let char_width = 7.0;
        let desired_size = egui::vec2((text.len() as f32 * char_width).max(16.0) + 10.0, 18.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        ui.painter().rect_filled(rect, 9.0, Self::accent_dim());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::TextStyle::Small,
            Color32::WHITE,
        );
    }

    // -- Validation ---------------------------------------------------------
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

    // -- Property editor helpers --------------------------------------------
    fn prop_label(ui: &mut egui::Ui, label: &str) {
        ui.add_sized(
            egui::vec2(130.0, 20.0),
            egui::Label::new(label).text_color(Self::text_normal()),
        );
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
            Self::prop_label(ui, label);
            ui.add(
                egui::DragValue::new(value)
                    .speed(speed)
                    .clamp_range(min..=max),
            );
        });
    }

    fn drag_u32(ui: &mut egui::Ui, label: &str, value: &mut u32, min: u32, max: u32) {
        ui.horizontal(|ui| {
            Self::prop_label(ui, label);
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
        ui.horizontal(|ui| {
            Self::prop_label(ui, label);
            let axis_colors = [Self::axis_x_color(), Self::axis_y_color(), Self::axis_z_color()];
            for ((axis, value), color) in ["X", "Y", "Z"]
                .into_iter()
                .zip(values.into_iter())
                .zip(axis_colors.into_iter())
            {
                // Colored axis tag
                let tag_size = egui::vec2(16.0, 20.0);
                let (tag_rect, _) = ui.allocate_exact_size(tag_size, egui::Sense::hover());
                ui.painter().rect_filled(
                    tag_rect,
                    3.0,
                    color,
                );
                ui.painter().text(
                    tag_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    axis,
                    egui::TextStyle::Small,
                    Color32::WHITE,
                );

                ui.add(
                    egui::DragValue::new(value)
                        .speed(speed)
                        .clamp_range(min..=max),
                );
                ui.add_space(2.0);
            }
        });
    }

    // -- Styled button helpers ----------------------------------------------
    fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        let btn = egui::Button::new(label)
            .fill(Self::accent_dim())
            .text_color(Color32::WHITE);
        ui.add(btn)
    }

    fn subtle_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        let btn = egui::Button::new(label)
            .fill(Color32::from_rgb(42, 42, 56))
            .text_color(Self::text_normal());
        ui.add(btn)
    }

    fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        let btn = egui::Button::new(label)
            .fill(Color32::from_rgb(120, 30, 30))
            .text_color(Color32::from_rgb(255, 180, 180));
        ui.add(btn)
    }

    // -- Menu bar -----------------------------------------------------------
    fn draw_menu_bar(
        &mut self,
        egui_ctx: &egui::CtxRef,
        persisted: &mut PersistedState,
        ctx: &mut FrameContext,
    ) {
        egui::TopBottomPanel::top("main_menu_bar")
            .height_range(36.0..=36.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(18, 18, 26))
                    .stroke(egui::Stroke::new(1.0, Self::border_color())),
            )
            .show(egui_ctx, |ui| {
                ui.add_space(2.0);
                egui::menu::bar(ui, |ui| {
                    ui.add_space(6.0);

                    // App title
                    ui.add(
                        egui::Label::new("DIST RENDER")
                            .text_color(Self::accent_color())
                            .strong(),
                    );
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);

                    egui::menu::menu(ui, "  Create  ", |ui| {
                        if ui.button("  Empty GameObject  ").clicked() {
                            let name =
                                format!("GameObject {}", persisted.scene.next_game_object_id);
                            let game_object_id = persisted
                                .scene
                                .create_game_object(name, SceneElementTransform::IDENTITY);
                            self.selected_game_object = Some(game_object_id);
                        }
                    });

                    egui::menu::menu(ui, "  Playback  ", |ui| {
                        if ui.button("  Add Keyframe  ").clicked() {
                            self.add_sequence_keyframe(persisted);
                        }
                        ui.separator();
                        if self.is_sequence_playing() {
                            if ui.button("  Stop  ").clicked() {
                                self.stop_sequence();
                            }
                        } else if ui.button("  Play  ").clicked() {
                            self.play_sequence(persisted);
                        }
                    });

                    if Self::subtle_button(ui, "Scene Root").clicked() {
                        self.selected_game_object = None;
                    }

                    // Right-aligned performance stats
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.add_space(10.0);
                        ui.add(
                            egui::Label::new(format!(
                                "{}x{}",
                                ctx.render_extent[0], ctx.render_extent[1]
                            ))
                            .text_color(Self::text_dim())
                            .small(),
                        );
                        ui.add_space(8.0);

                        if let Some(report) = gpu_profiler::profiler().last_report() {
                            let gpu_time_ms: f64 =
                                report.scopes.iter().map(|scope| scope.duration.ms()).sum();
                            let gpu_color = if gpu_time_ms > 16.0 {
                                Self::warn_color()
                            } else {
                                Self::success_color()
                            };
                            ui.add(
                                egui::Label::new(format!("GPU {:.2}ms", gpu_time_ms))
                                    .text_color(gpu_color)
                                    .small(),
                            );
                            ui.add_space(8.0);
                        }

                        let cpu_ms = ctx.dt_filtered * 1000.0;
                        let cpu_color = if cpu_ms > 16.0 {
                            Self::warn_color()
                        } else {
                            Self::muted_color()
                        };
                        ui.add(
                            egui::Label::new(format!("CPU {:.2}ms", cpu_ms))
                                .text_color(cpu_color)
                                .small(),
                        );
                    });
                });
            });
    }

    /// Register material texture GPU images as egui user textures for the selected game object.
    /// Uses `TextureId::User(100 + n)` for material map images.
    fn register_material_textures(
        &mut self,
        persisted: &PersistedState,
        ctx: &mut FrameContext,
    ) {
        self.material_texture_ids.clear();
        self.cached_material_infos.clear();

        let game_object_id = match self.selected_game_object {
            Some(id) => id,
            None => return,
        };

        let mesh_handle = match self.runtime_scene.game_object_mesh_handle(
            &persisted.scene,
            ctx.world_renderer,
            game_object_id,
        ) {
            Some(h) => h,
            None => return,
        };

        let material_infos = ctx.world_renderer.mesh_material_infos(mesh_handle);
        self.cached_material_infos = material_infos.to_vec();

        let egui_ctx = match ctx.egui.as_mut() {
            Some(ec) => ec,
            None => return,
        };

        // Cap texture registrations to avoid descriptor pool exhaustion.
        const MAX_MATERIAL_TEXTURES: u64 = 64;
        let mut next_slot = 100u64;
        'outer: for (mat_idx, mat) in material_infos.iter().enumerate() {
            for (slot, img_opt) in mat.map_images.iter().enumerate() {
                if let Some(img) = img_opt {
                    if next_slot - 100 >= MAX_MATERIAL_TEXTURES {
                        break 'outer;
                    }
                    let texture_id = egui::TextureId::User(next_slot);
                    egui_ctx.register_user_texture(texture_id, img.clone());
                    self.material_texture_ids.push((mat_idx, slot, texture_id));
                    next_slot += 1;
                }
            }
        }
    }

    // -- Main entry point ---------------------------------------------------
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
            // Pre-register material textures for the selected object before the egui frame.
            self.register_material_textures(persisted, ctx);

            ctx.egui.take().unwrap().frame(|egui_ctx| {
                Self::apply_modern_style(egui_ctx);

                self.validate_selected_game_object(&persisted.scene);
                self.draw_menu_bar(egui_ctx, persisted, ctx);

                egui::SidePanel::left("hierarchy_panel")
                    .default_width(320.0)
                    .resizable(true)
                    .width_range(220.0..=720.0)
                    .frame(
                        egui::Frame::none()
                            .fill(Self::panel_fill())
                            .margin(egui::vec2(10.0, 8.0))
                            .stroke(egui::Stroke::new(1.0, Self::border_color())),
                    )
                    .show(egui_ctx, |ui| {
                        self.draw_hierarchy_panel(ui, persisted);
                    });

                egui::SidePanel::right("inspector_panel")
                    .default_width(460.0)
                    .resizable(true)
                    .width_range(300.0..=820.0)
                    .frame(
                        egui::Frame::none()
                            .fill(Self::panel_fill())
                            .margin(egui::vec2(10.0, 8.0))
                            .stroke(egui::Stroke::new(1.0, Self::border_color())),
                    )
                    .show(egui_ctx, |ui| {
                        self.draw_inspector_panel(ui, persisted, ctx);
                    });

                egui::TopBottomPanel::bottom("console_panel")
                    .default_height(180.0)
                    .resizable(true)
                    .height_range(80.0..=500.0)
                    .frame(
                        egui::Frame::none()
                            .fill(Self::panel_fill())
                            .margin(egui::vec2(10.0, 6.0))
                            .stroke(egui::Stroke::new(1.0, Self::border_color())),
                    )
                    .show(egui_ctx, |ui| {
                        self.draw_console_panel(ui);
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