pub mod config_tab;
pub mod console;
pub mod control_tab;
pub mod dialogs;
pub mod mods_tab;
pub mod server_list;
pub mod settings_window;
pub mod theme;

use crate::app::{App, Tab};
use crate::session::State;
use egui::{Color32, Margin, RichText, Ui};
use std::time::Duration;
use theme::palette;

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    dialogs::show(app, &ctx);
    settings_window::show(app, &ctx);

    status_bar(app, ui);
    server_list::show(app, ui);
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(palette(ui).bg)
                .inner_margin(Margin::symmetric(24, 16)),
        )
        .show(ui, |ui| server_view(app, ui));
}

pub fn state_color(ui: &Ui, state: State) -> Color32 {
    let p = palette(ui);
    match state {
        State::Running => p.success,
        State::Stopping | State::Restarting => p.warning,
        State::Stopped => p.muted,
    }
}

pub fn state_label(state: State) -> &'static str {
    match state {
        State::Running => "Running",
        State::Stopping => "Stopping",
        State::Restarting => "Restarting",
        State::Stopped => "Stopped",
    }
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

// ------------------------------------------------------------------ layout

fn status_bar(app: &mut App, ui: &mut Ui) {
    let p = palette(ui);
    egui::Panel::bottom("status_bar")
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(p.sidebar)
                .stroke(egui::Stroke::new(1.0, p.border))
                .inner_margin(Margin::symmetric(14, 5)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(msg) = &app.status {
                    let color = if msg.is_error { p.danger } else { p.success };
                    theme::dot(ui, color);
                    let label = ui
                        .add(
                            egui::Label::new(RichText::new(&msg.text).small().color(p.text))
                                .truncate()
                                .sense(egui::Sense::click()),
                        )
                        .on_hover_text("Click to dismiss");
                    if label.clicked() {
                        app.status = None;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(p.muted),
                    );
                    let running = app.running_count();
                    if running > 0 {
                        ui.label(
                            RichText::new(format!("{running} running"))
                                .small()
                                .color(p.text),
                        );
                        theme::dot(ui, p.success);
                    }
                });
            });
        });
}

fn server_view(app: &mut App, ui: &mut Ui) {
    let Some(id) = app.selected.clone() else {
        empty_state(app, ui);
        return;
    };
    let Some(server) = app.servers.get(&id) else {
        app.selected = None;
        return;
    };
    let (name, path, exe_ok) = (
        server.name.clone(),
        server.path.clone(),
        server.executable_exists(),
    );
    let state = app.state_of(&id);
    let p = palette(ui);

    // Header: name, path, state and controls
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                theme::heading(ui, &name);
                ui.add_space(4.0);
                let session = app.sessions.get(&id);
                let label = match (state, session) {
                    (State::Running, Some(s)) => format!(
                        "RUNNING  {}",
                        format_duration(s.uptime().unwrap_or_default())
                    ),
                    (State::Restarting, Some(s)) => format!(
                        "RESTARTING IN {}s",
                        s.restart_in().map_or(0, |d| d.as_secs() + 1)
                    ),
                    _ => state_label(state).to_uppercase(),
                };
                theme::pill(ui, &label, state_color(ui, state));
            });
            ui.add(
                egui::Label::new(
                    RichText::new(path.display().to_string())
                        .small()
                        .color(p.muted),
                )
                .truncate(),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            server_controls(app, ui, &id, state, exe_ok);
        });
    });
    ui.add_space(10.0);

    theme::tabs(
        ui,
        &mut app.tab,
        &[
            (Tab::Config, "Config"),
            (Tab::Mods, "Mods"),
            (Tab::Control, "Control"),
        ],
    );
    ui.add_space(12.0);

    // The console stays available after the server exits so its log can be read.
    if app.sessions.contains_key(&id) {
        egui::Panel::bottom("server_console")
            .resizable(true)
            .min_size(120.0)
            .default_size(230.0)
            .show_separator_line(false)
            .frame(egui::Frame::new().inner_margin(Margin {
                top: 12,
                ..Default::default()
            }))
            .show(ui, |ui| console::show(app, ui, &id));
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::new())
        .show(ui, |ui| match app.tab {
            Tab::Config => config_tab::show(app, ui, &id),
            Tab::Mods => mods_tab::show(app, ui, &id),
            Tab::Control => control_tab::show(app, ui, &id),
        });
}

fn server_controls(app: &mut App, ui: &mut Ui, id: &str, state: State, exe_ok: bool) {
    match state {
        State::Stopped => {
            let start = ui
                .add_enabled(exe_ok, theme::primary_button(ui, "Start server"))
                .on_disabled_hover_text(format!(
                    "{} was not found in the server folder",
                    crate::process::SERVER_EXE
                ));
            if start.clicked() {
                app.start_server(id);
            }
        }
        State::Running => {
            if ui.add(theme::danger_button(ui, "Stop")).clicked() {
                app.stop_server(id);
            }
            if ui.button("Restart").clicked() {
                app.restart_server(id);
            }
        }
        State::Stopping => {
            if ui
                .add(theme::danger_button(ui, "Force stop"))
                .on_hover_text("Kill the server process immediately")
                .clicked()
            {
                app.force_stop_server(id);
            }
            ui.spinner();
        }
        State::Restarting => {
            if ui.button("Cancel restart").clicked() {
                app.stop_server(id);
            }
        }
    }
}

fn empty_state(app: &mut App, ui: &mut Ui) {
    let p = palette(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.32);
        if app.servers.servers.is_empty() {
            theme::heading(ui, "No servers yet");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Add the folder of an existing BeamMP server, the one containing ServerConfig.toml.",
                )
                .color(p.muted),
            );
            ui.add_space(14.0);
            if ui.add(theme::primary_button(ui, "Add server")).clicked() {
                app.add_server();
            }
        } else {
            theme::heading(ui, "No server selected");
            ui.add_space(4.0);
            ui.label(RichText::new("Pick a server from the list on the left.").color(p.muted));
        }
    });
}
