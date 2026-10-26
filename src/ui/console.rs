use super::theme::{self, palette};
use crate::app::App;
use crate::console::LineKind;
use crate::util;
use egui::{Key, Margin, RichText, Stroke, TextStyle, TextWrapMode, Ui};
use std::sync::Arc;

pub fn show(app: &mut App, ui: &mut Ui, id: &str) {
    let folder = app.servers.get(id).map(|s| s.path.clone());
    let Some(session) = app.sessions.get_mut(id) else {
        return;
    };
    let p = palette(ui);
    let running = session.is_running();
    let console = Arc::clone(&session.console);
    let mut error = None;

    egui::Frame::new()
        .fill(p.sunken)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(10)
        .show(ui, |ui| {
            // Header
            egui::Frame::new()
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::section_title(ui, "Console");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(theme::link_button(ui, "Clear")).clicked() {
                                console.clear();
                            }
                            if ui
                                .add(theme::link_button(ui, "Copy"))
                                .on_hover_text("Copy the whole log")
                                .clicked()
                            {
                                ui.ctx().copy_text(console.to_text());
                            }
                            if let Some(folder) = &folder
                                && ui
                                    .add(theme::link_button(ui, "Open folder"))
                                    .on_hover_text("Open the server folder (Server.log lives here)")
                                    .clicked()
                                && let Err(e) = util::open_in_file_manager(folder)
                            {
                                error = Some(format!("Failed to open folder: {e}"));
                            }
                            ui.checkbox(&mut session.auto_scroll, "Auto-scroll");
                        });
                    });
                });
            let rect = ui.min_rect();
            ui.painter()
                .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.border));

            // Command line at the bottom; the log fills the space above it.
            egui::Panel::bottom("console_input")
                .show_separator_line(true)
                .frame(egui::Frame::new().inner_margin(Margin::symmetric(14, 8)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(">").monospace().color(p.accent).strong());
                        let input = ui.add_enabled(
                            running,
                            egui::TextEdit::singleline(&mut session.command_input)
                                .font(TextStyle::Monospace)
                                .frame(egui::Frame::NONE)
                                .hint_text(if running {
                                    "Type a server command and press Enter (Up/Down for history)"
                                } else {
                                    "Start the server to send commands"
                                })
                                .desired_width(ui.available_width()),
                        );
                        if input.has_focus() {
                            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                                session.browse_history(true);
                            } else if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                                session.browse_history(false);
                            }
                        }
                        if input.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            if let Err(e) = session.submit_command_input() {
                                error = Some(format!("Failed to send command: {e}"));
                            }
                            input.request_focus();
                        }
                    });
                });

            let row_height = ui.text_style_height(&TextStyle::Monospace);
            let colors = LineColors::new(p);
            let lines = console.lock();
            egui::Frame::new()
                .inner_margin(Margin {
                    left: 14,
                    right: 6,
                    top: 6,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink(false)
                        .stick_to_bottom(session.auto_scroll)
                        .show_rows(ui, row_height, lines.lines.len(), |ui, rows| {
                            // Rows must be exactly row_height tall for show_rows' virtualization
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for line in lines.lines.range(rows) {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&line.text)
                                            .monospace()
                                            .color(colors.get(line.kind)),
                                    )
                                    .wrap_mode(TextWrapMode::Extend),
                                );
                            }
                        });
                });
        });

    if let Some(error) = error {
        app.set_status(error, true);
    }
}

struct LineColors {
    normal: egui::Color32,
    warn: egui::Color32,
    error: egui::Color32,
    weak: egui::Color32,
    chat: egui::Color32,
    input: egui::Color32,
    system: egui::Color32,
}

impl LineColors {
    fn new(p: &theme::Palette) -> Self {
        Self {
            normal: p.text,
            warn: p.warning,
            error: p.danger,
            weak: p.muted,
            chat: p.info,
            input: p.text_strong,
            system: p.accent,
        }
    }

    fn get(&self, kind: LineKind) -> egui::Color32 {
        match kind {
            LineKind::Normal => self.normal,
            LineKind::Warn => self.warn,
            LineKind::Error => self.error,
            LineKind::Debug => self.weak,
            LineKind::Chat => self.chat,
            LineKind::Input => self.input,
            LineKind::System => self.system,
        }
    }
}
