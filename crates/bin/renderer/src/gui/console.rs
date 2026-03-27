use super::*;
use dist_render::logging::{drain_log_entries, LogEntry};

/// Filter level for the console display.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsoleFilter {
    All,
    Info,
    Warn,
    Error,
}

impl ConsoleFilter {
    fn label(self) -> &'static str {
        match self {
            ConsoleFilter::All => "All",
            ConsoleFilter::Info => "Info",
            ConsoleFilter::Warn => "Warn",
            ConsoleFilter::Error => "Error",
        }
    }

    fn passes(self, level: log::Level) -> bool {
        match self {
            ConsoleFilter::All => true,
            ConsoleFilter::Info => level <= log::Level::Info,
            ConsoleFilter::Warn => level <= log::Level::Warn,
            ConsoleFilter::Error => level == log::Level::Error,
        }
    }
}

/// Persistent console state kept on RuntimeState.
pub struct ConsoleState {
    pub entries: Vec<LogEntry>,
    pub filter: ConsoleFilter,
    pub auto_scroll: bool,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            filter: ConsoleFilter::All,
            auto_scroll: true,
        }
    }
}

impl ConsoleState {
    /// Drain new log messages from the global buffer.
    pub fn poll(&mut self) {
        self.entries.extend(drain_log_entries());
        const MAX_DISPLAY: usize = 2048;
        if self.entries.len() > MAX_DISPLAY {
            let excess = self.entries.len() - MAX_DISPLAY;
            self.entries.drain(..excess);
        }
    }
}

impl RuntimeState {
    pub(super) fn draw_console_panel(&mut self, ui: &mut egui::Ui) {
        // Poll new entries every frame.
        self.console.poll();

        // Header row
        ui.horizontal(|ui| {
            Self::panel_title(ui, "Console");

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                if Self::subtle_button(ui, "Clear").clicked() {
                    self.console.entries.clear();
                }

                ui.separator();

                // Filter buttons
                for &f in &[
                    ConsoleFilter::All,
                    ConsoleFilter::Info,
                    ConsoleFilter::Warn,
                    ConsoleFilter::Error,
                ] {
                    let active = self.console.filter == f;
                    let label = f.label();

                    let count = match f {
                        ConsoleFilter::All => self.console.entries.len(),
                        _ => self
                            .console
                            .entries
                            .iter()
                            .filter(|e| f.passes(e.level))
                            .count(),
                    };

                    let text = format!("{} ({})", label, count);
                    if ui.selectable_label(active, text).clicked() {
                        self.console.filter = f;
                    }
                }

                ui.checkbox(&mut self.console.auto_scroll, "Auto-scroll");
            });
        });

        ui.add_space(2.0);

        // Separator
        let sep = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [
                egui::pos2(sep.left(), sep.top()),
                egui::pos2(sep.right(), sep.top()),
            ],
            egui::Stroke::new(1.0, Self::border_color()),
        );
        ui.add_space(2.0);

        // Log entries
        let filter = self.console.filter;
        let auto_scroll = self.console.auto_scroll;
        let text_style = egui::TextStyle::Monospace;

        egui::ScrollArea::vertical()
            .show(ui, |ui| {
                for entry in &self.console.entries {
                    if !filter.passes(entry.level) {
                        continue;
                    }

                    let (level_color, level_str) = match entry.level {
                        log::Level::Error => (Color32::from_rgb(235, 70, 80), "ERR"),
                        log::Level::Warn => (Color32::from_rgb(255, 186, 66), "WRN"),
                        log::Level::Info => (Color32::from_rgb(72, 210, 120), "INF"),
                        log::Level::Debug => (Color32::from_rgb(130, 140, 170), "DBG"),
                        log::Level::Trace => (Color32::from_rgb(90, 95, 115), "TRC"),
                    };

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(&entry.timestamp)
                                .text_style(text_style)
                                .text_color(Self::text_dim()),
                        );
                        ui.add(
                            egui::Label::new(format!("[{}]", level_str))
                                .text_style(text_style)
                                .text_color(level_color),
                        );

                        // Shorten target
                        let short_target: &str = entry
                            .target
                            .rsplit("::")
                            .next()
                            .unwrap_or(&entry.target);
                        ui.add(
                            egui::Label::new(short_target)
                                .text_style(text_style)
                                .text_color(Self::muted_color()),
                        );

                        ui.add(
                            egui::Label::new(&entry.message)
                                .text_style(text_style)
                                .text_color(Self::text_normal())
                                .wrap(true),
                        );
                    });
                }
            });
    }
}
