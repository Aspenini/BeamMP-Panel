use super::theme::{self, palette};
use crate::app::App;
use egui::{Grid, Key, RichText, ScrollArea, TextEdit, Ui};

enum Action {
    Command(String),
    RefreshPlayers,
}

/// Quick commands: (label, command, tooltip)
const QUICK_COMMANDS: &[(&str, &str, &str)] = &[
    ("Status", "status", "Uptime, players, mods and memory usage"),
    ("Version", "version", "Server version"),
    (
        "Reload plugins",
        "reloadmods",
        "Reload server (Lua) plugins",
    ),
    ("Help", "help", "List all console commands"),
];

const MAX_WIDTH: f32 = 760.0;

pub fn show(app: &mut App, ui: &mut Ui, id: &str) {
    let p = palette(ui);
    let Some(session) = app.sessions.get(id).filter(|s| s.is_running()) else {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                RichText::new("Server is not running")
                    .color(p.text_strong)
                    .size(16.0),
            );
            ui.add_space(2.0);
            ui.label(RichText::new("Start it to manage players and send commands.").color(p.muted));
        });
        return;
    };
    let players = &session.players;
    let pending = session.player_list_pending();
    let (kick_name, kick_reason, broadcast) =
        (&mut app.kick_name, &mut app.kick_reason, &mut app.broadcast);
    let mut action = None;

    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.set_max_width(MAX_WIDTH);

        theme::card(ui, "Players", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} online", players.len())).color(p.text_strong));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(!pending, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        action = Some(Action::RefreshPlayers);
                    }
                    if pending {
                        ui.spinner();
                    }
                });
            });

            if players.is_empty() {
                ui.label(
                    RichText::new("No players listed yet. Refresh asks the server.")
                        .small()
                        .color(p.muted),
                );
            } else {
                ui.add_space(6.0);
                Grid::new("players")
                    .num_columns(4)
                    .striped(true)
                    .spacing([28.0, 8.0])
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        for header in ["NAME", "ID", "CARS", ""] {
                            ui.label(RichText::new(header).small().strong().color(p.muted));
                        }
                        ui.end_row();
                        for player in players {
                            ui.label(RichText::new(&player.name).color(p.text_strong));
                            ui.label(&player.id);
                            ui.label(&player.cars);
                            if ui.add(theme::link_button(ui, "Kick")).clicked() {
                                kick_name.clone_from(&player.name);
                            }
                            ui.end_row();
                        }
                    });
            }
        });
        ui.add_space(12.0);

        theme::card(ui, "Kick player", |ui| {
            Grid::new("kick_form")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Player").color(p.muted));
                    ui.add(TextEdit::singleline(kick_name).desired_width(280.0));
                    ui.end_row();
                    ui.label(RichText::new("Reason").color(p.muted));
                    ui.add(
                        TextEdit::singleline(kick_reason)
                            .hint_text("Optional")
                            .desired_width(280.0),
                    );
                    ui.end_row();
                    ui.label("");
                    ui.horizontal(|ui| {
                        let name = kick_name.trim();
                        if ui
                            .add_enabled(!name.is_empty(), theme::danger_button(ui, "Kick"))
                            .clicked()
                        {
                            let reason = kick_reason.trim();
                            action = Some(Action::Command(if reason.is_empty() {
                                format!("kick {name}")
                            } else {
                                format!("kick {name} {reason}")
                            }));
                            kick_name.clear();
                            kick_reason.clear();
                        }
                        if ui.add(theme::link_button(ui, "Clear")).clicked() {
                            kick_name.clear();
                            kick_reason.clear();
                        }
                    });
                    ui.end_row();
                });
        });
        ui.add_space(12.0);

        theme::card(ui, "Broadcast", |ui| {
            ui.horizontal(|ui| {
                let input = ui.add(
                    TextEdit::singleline(broadcast)
                        .hint_text("Message to all players")
                        .desired_width((ui.available_width() - 80.0).max(160.0)),
                );
                let enter = input.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                let send = ui.add(theme::primary_button(ui, "Send")).clicked();
                if (enter || send) && !broadcast.trim().is_empty() {
                    action = Some(Action::Command(format!("say {}", broadcast.trim())));
                    broadcast.clear();
                    if enter {
                        input.request_focus();
                    }
                }
            });
        });
        ui.add_space(12.0);

        theme::card(ui, "Commands", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, command, tip) in QUICK_COMMANDS {
                    if ui.button(*label).on_hover_text(*tip).clicked() {
                        action = Some(Action::Command((*command).to_string()));
                    }
                }
                if ui
                    .button("Lua console")
                    .on_hover_text(
                        "Switch the console into Lua mode. Type exit() to leave it; \
                         Stop won't work gracefully while it's active.",
                    )
                    .clicked()
                {
                    action = Some(Action::Command("lua".to_string()));
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("Output appears in the console, which also accepts any command.")
                    .small()
                    .color(p.muted),
            );
        });
        ui.add_space(8.0);
    });

    match action {
        Some(Action::Command(command)) => app.send_command(id, &command),
        Some(Action::RefreshPlayers) => app.refresh_players(id),
        None => {}
    }
}
