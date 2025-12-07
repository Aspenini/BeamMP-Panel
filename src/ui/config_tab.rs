use crate::server::ServerEntry;
use crate::{StatusMessage};
use egui::{ScrollArea, Ui};

pub fn show(ui: &mut Ui, server: &mut ServerEntry, status: &mut Option<StatusMessage>) {
    if let Some(error) = &server.config_error {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
        ui.separator();
        if ui.button("Reload Config").clicked() {
            server.load_config();
        }
        return;
    }

    let config = match &mut server.edited_config {
        Some(c) => c,
        None => {
            ui.label("No config loaded");
            return;
        }
    };

    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("General Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Server Name:");
            ui.text_edit_singleline(&mut config.general.name);
        });

        ui.horizontal(|ui| {
            ui.label("Port:");
            ui.add(egui::DragValue::new(&mut config.general.port).range(1..=65535));
        });

        ui.horizontal(|ui| {
            ui.label("Auth Key:");
            ui.text_edit_singleline(&mut config.general.auth_key);
        });

        ui.horizontal(|ui| {
            ui.label("IP:");
            ui.text_edit_singleline(&mut config.general.ip);
        });

        ui.horizontal(|ui| {
            ui.label("Max Players:");
            ui.add(egui::DragValue::new(&mut config.general.max_players).range(1..=128));
        });

        ui.horizontal(|ui| {
            ui.label("Max Cars:");
            ui.add(egui::DragValue::new(&mut config.general.max_cars).range(1..=10));
        });

        ui.horizontal(|ui| {
            ui.label("Map:");
            ui.text_edit_singleline(&mut config.general.map);
        });

        ui.horizontal(|ui| {
            ui.label("Tags:");
            ui.text_edit_singleline(&mut config.general.tags);
        });

        ui.horizontal(|ui| {
            ui.label("Resource Folder:");
            ui.text_edit_singleline(&mut config.general.resource_folder);
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut config.general.allow_guests, "Allow Guests");
            ui.checkbox(&mut config.general.log_chat, "Log Chat");
            ui.checkbox(&mut config.general.debug, "Debug");
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut config.general.private, "Private");
            ui.checkbox(&mut config.general.information_packet, "Information Packet");
        });

        ui.label("Description:");
        ui.text_edit_multiline(&mut config.general.description);

        ui.add_space(10.0);
        ui.heading("Misc Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut config.misc.im_scared_of_updates,
                "I'm Scared of Updates",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Update Reminder Time:");
            ui.text_edit_singleline(&mut config.misc.update_reminder_time);
        });
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Apply").clicked() {
            match server.save_config() {
                Ok(_) => {
                    *status = Some(StatusMessage {
                        text: "Configuration saved!".to_string(),
                        is_error: false,
                    });
                }
                Err(e) => {
                    *status = Some(StatusMessage {
                        text: format!("Failed to save config: {}", e),
                        is_error: true,
                    });
                }
            }
        }

        let is_dirty = server.is_config_dirty();
        ui.add_enabled_ui(is_dirty, |ui| {
            if ui.button("Revert").clicked() {
                server.revert_config();
                *status = Some(StatusMessage {
                    text: "Changes reverted".to_string(),
                    is_error: false,
                });
            }
        });

        if is_dirty {
            ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
        }
    });
}

