use super::theme::{self, palette};
use crate::app::App;
use egui::{DragValue, Grid, Key, KeyboardShortcut, Modifiers, RichText, ScrollArea, TextEdit, Ui};

const KEYMASTER_URL: &str = "https://keymaster.beammp.com";
/// Keep forms readable on wide windows.
const MAX_FORM_WIDTH: f32 = 760.0;
const FIELD_WIDTH: f32 = 360.0;

/// Stock BeamNG.drive levels, for the map picker.
const STOCK_MAPS: &[(&str, &str)] = &[
    ("Grid Map v2", "gridmap_v2"),
    ("Automation Test Track", "automation_test_track"),
    ("Derby Arenas", "derby"),
    ("Driver Training", "driver_training"),
    ("East Coast, USA", "east_coast_usa"),
    ("Hirochi Raceway", "hirochi_raceway"),
    ("Industrial Site", "industrial"),
    ("Italy", "italy"),
    ("Johnson Valley", "johnson_valley"),
    ("Jungle Rock Island", "jungle_rock_island"),
    ("Small Island, USA", "small_island"),
    ("Small Grid", "smallgrid"),
    ("Utah, USA", "utah"),
    ("West Coast, USA", "west_coast_usa"),
];

pub fn show(app: &mut App, ui: &mut Ui, id: &str) {
    let running = app.sessions.get(id).is_some_and(|s| s.is_running());
    let show_auth_key = &mut app.show_auth_key;
    let Some(server) = app.servers.get_mut(id) else {
        return;
    };
    let p = palette(ui);

    if let Some(error) = server.config_error.clone() {
        theme::card(ui, "ServerConfig.toml", |ui| {
            ui.label(RichText::new(format!("Could not load the config: {error}")).color(p.danger));
            ui.add_space(8.0);
            if ui.button("Reload").clicked() {
                server.load_config();
            }
        });
        return;
    }

    let mut save =
        ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S)));
    let mut revert = false;
    let mut reload = false;
    let mut panel_options_changed = false;
    let is_dirty = server.is_config_dirty();

    // Action bar pinned to the bottom of the tab
    egui::Panel::bottom("config_actions")
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                save |= ui
                    .add_enabled(is_dirty, theme::primary_button(ui, "Save changes"))
                    .on_hover_text("Ctrl+S")
                    .clicked();
                revert = ui.add_enabled(is_dirty, egui::Button::new("Revert")).clicked();
                reload = ui
                    .add(theme::link_button(ui, "Reload from disk"))
                    .on_hover_text("Discard edits and re-read ServerConfig.toml")
                    .clicked();
                if is_dirty {
                    theme::pill(ui, "UNSAVED CHANGES", p.warning);
                } else if running {
                    ui.label(
                        RichText::new("Restart the server to apply saved changes.")
                            .small()
                            .color(p.muted),
                    );
                }
            });
        });

    let Some(config) = &mut server.edited_config else {
        label(ui, "No config loaded");
        return;
    };
    let general = &mut config.general;

    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.set_max_width(MAX_FORM_WIDTH);

        theme::card(ui, "Server listing", |ui| {
            form(ui, "listing", |ui| {
                label(ui, "Name");
                ui.add(TextEdit::singleline(&mut general.name).desired_width(FIELD_WIDTH))
                    .on_hover_text("Shown in the server browser. Supports ^ color codes.");
                ui.end_row();

                label(ui, "Description");
                ui.add(
                    TextEdit::multiline(&mut general.description)
                        .desired_rows(3)
                        .desired_width(FIELD_WIDTH),
                );
                ui.end_row();

                label(ui, "Tags");
                ui.add(TextEdit::singleline(&mut general.tags).desired_width(FIELD_WIDTH))
                    .on_hover_text("Comma separated, e.g. Freeroam,Roleplay");
                ui.end_row();

                label(ui, "Map");
                map_picker(ui, &mut general.map);
                ui.end_row();

                label(ui, "Visibility");
                ui.checkbox(&mut general.private, "Private (hidden from the server list)");
                ui.end_row();
            });
        });
        ui.add_space(12.0);

        theme::card(ui, "Authentication", |ui| {
            form(ui, "auth", |ui| {
                label(ui, "Auth key");
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut general.auth_key)
                            .password(!*show_auth_key)
                            .desired_width(FIELD_WIDTH - 70.0),
                    );
                    let toggle = if *show_auth_key { "Hide" } else { "Show" };
                    if ui.add(theme::link_button(ui, toggle)).clicked() {
                        *show_auth_key = !*show_auth_key;
                    }
                });
                ui.end_row();

                label(ui, "");
                ui.hyperlink_to(
                    RichText::new("Get a key from the BeamMP Keymaster").small(),
                    KEYMASTER_URL,
                );
                ui.end_row();

                label(ui, "Guests");
                ui.checkbox(&mut general.allow_guests, "Allow players without a BeamMP account");
                ui.end_row();
            });
        });
        ui.add_space(12.0);

        theme::card(ui, "Gameplay and network", |ui| {
            form(ui, "network", |ui| {
                label(ui, "Max players");
                ui.add(DragValue::new(&mut general.max_players).range(1..=256));
                ui.end_row();

                label(ui, "Cars per player");
                ui.add(DragValue::new(&mut general.max_cars).range(0..=50));
                ui.end_row();

                label(ui, "Port");
                ui.add(DragValue::new(&mut general.port).range(1..=65535));
                ui.end_row();

                label(ui, "Bind address");
                ui.add(TextEdit::singleline(&mut general.ip).desired_width(180.0))
                    .on_hover_text(":: listens on all IPv4 and IPv6 addresses");
                ui.end_row();

                label(ui, "Resource folder");
                ui.add(TextEdit::singleline(&mut general.resource_folder).desired_width(180.0));
                ui.end_row();
            });
        });
        ui.add_space(12.0);

        theme::card(ui, "Logging and updates", |ui| {
            form(ui, "logging", |ui| {
                label(ui, "Logging");
                ui.vertical(|ui| {
                    ui.checkbox(&mut general.log_chat, "Log chat messages");
                    ui.checkbox(&mut general.debug, "Debug output");
                    ui.checkbox(&mut general.information_packet, "Answer server info queries")
                        .on_hover_text("Lets the launcher show server details without joining");
                });
                ui.end_row();

                label(ui, "Updates");
                ui.checkbox(&mut config.misc.im_scared_of_updates, "Hide update reminders");
                ui.end_row();

                label(ui, "Reminder interval");
                ui.add(
                    TextEdit::singleline(&mut config.misc.update_reminder_time)
                        .desired_width(80.0),
                )
                .on_hover_text("e.g. 30s, 10min, 1h, 1d");
                ui.end_row();
            });
        });
        ui.add_space(12.0);

        theme::card(ui, "Panel", |ui| {
            ui.label(
                RichText::new("Saved by the panel right away, not in ServerConfig.toml.")
                    .small()
                    .color(p.muted),
            );
            ui.add_space(4.0);
            panel_options_changed |= ui
                .checkbox(&mut server.auto_start, "Start this server when the panel opens")
                .changed();
            panel_options_changed |= ui
                .checkbox(&mut server.auto_restart, "Restart automatically if it crashes")
                .on_hover_text("Gives up after 3 crashes within 5 minutes")
                .changed();
        });
        ui.add_space(8.0);
    });

    let mut status = None;
    let mut renamed = false;
    if save && is_dirty {
        status = Some(match server.save_config() {
            Ok(r) => {
                renamed = r;
                ("Configuration saved".to_string(), false)
            }
            Err(e) => (format!("Failed to save config: {e}"), true),
        });
    } else if revert {
        server.revert_config();
        status = Some(("Changes reverted".to_string(), false));
    } else if reload {
        server.load_config();
        status = Some(("Reloaded ServerConfig.toml".to_string(), false));
    }

    if renamed || panel_options_changed {
        app.save_server_list();
    }
    if let Some((text, is_error)) = status {
        app.set_status(text, is_error);
    }
}

/// Two-column label/field grid.
fn form(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    Grid::new(id)
        .num_columns(2)
        .spacing([24.0, 10.0])
        .min_col_width(130.0)
        .show(ui, add);
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(palette(ui).muted));
}

fn map_picker(ui: &mut Ui, map: &mut String) {
    // BeamMP stores maps as "/levels/<name>/info.json"
    let current = map
        .strip_prefix("/levels/")
        .and_then(|m| m.strip_suffix("/info.json"))
        .unwrap_or(map.as_str())
        .to_string();
    let label = STOCK_MAPS
        .iter()
        .find(|(_, id)| id.eq_ignore_ascii_case(&current))
        .map_or_else(|| format!("Custom: {current}"), |(name, _)| (*name).to_string());

    ui.vertical(|ui| {
        egui::ComboBox::from_id_salt("map_picker")
            .selected_text(label)
            .width(FIELD_WIDTH)
            .show_ui(ui, |ui| {
                for (name, id) in STOCK_MAPS {
                    if ui
                        .selectable_label(current.eq_ignore_ascii_case(id), *name)
                        .clicked()
                    {
                        *map = format!("/levels/{id}/info.json");
                    }
                }
            });
        ui.add(TextEdit::singleline(map).desired_width(FIELD_WIDTH).font(egui::TextStyle::Monospace))
            .on_hover_text("For modded maps use /levels/<folder name>/info.json");
    });
}
