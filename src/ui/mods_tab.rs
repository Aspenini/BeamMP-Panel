use crate::mods;
use crate::server::ServerEntry;
use crate::{DeleteConfirmation, ModsCache, StatusMessage};
use egui::{ScrollArea, Ui};

pub fn show(
    ui: &mut Ui,
    server: &ServerEntry,
    mods_cache: &mut Option<ModsCache>,
    status: &mut Option<StatusMessage>,
    delete_confirmation: &mut Option<DeleteConfirmation>,
) {
    ui.horizontal(|ui| {
        if ui.button("Add Mod...").clicked() {
            if let Some(files) = rfd::FileDialog::new().pick_files() {
                let resource_folder = server.get_resource_folder();
                let mut added_count = 0;
                let mut errors = Vec::new();

                for file in files {
                    match mods::add_mod(&server.path, &resource_folder, &file) {
                        Ok(_) => added_count += 1,
                        Err(e) => errors.push(format!("{}: {}", file.display(), e)),
                    }
                }

                if added_count > 0 {
                    *status = Some(StatusMessage {
                        text: format!("Added {} mod(s)", added_count),
                        is_error: false,
                    });
                    *mods_cache = None; // Force reload
                }

                if !errors.is_empty() {
                    *status = Some(StatusMessage {
                        text: format!("Errors: {}", errors.join(", ")),
                        is_error: true,
                    });
                }
            }
        }

        if ui.button("Refresh").clicked() {
            *mods_cache = None; // Force reload
        }
    });

    ui.separator();

    // Use a flag to track if we need to reload after the UI is done
    let mut needs_reload = false;

    match mods_cache {
        Some(cache) => {
            if cache.mods.is_empty() {
                ui.label("No mods found");
            } else {
                ui.label(format!("Total mods: {}", cache.mods.len()));
                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    let resource_folder = server.get_resource_folder();

                    for (idx, mod_entry) in cache.mods.iter().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let status_text = if mod_entry.enabled {
                                    "✓ Enabled"
                                } else {
                                    "✗ Disabled"
                                };
                                let status_color = if mod_entry.enabled {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::GRAY
                                };

                                ui.colored_label(status_color, status_text);
                                ui.label(&mod_entry.relative_path);

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Delete").clicked() {
                                        *delete_confirmation = Some(DeleteConfirmation::Mod(idx));
                                    }

                                    if mod_entry.enabled {
                                        if ui.button("Disable").clicked() {
                                            match mods::disable_mod(
                                                &server.path,
                                                &resource_folder,
                                                &mod_entry.relative_path,
                                            ) {
                                                Ok(_) => {
                                                    *status = Some(StatusMessage {
                                                        text: "Mod disabled".to_string(),
                                                        is_error: false,
                                                    });
                                                    needs_reload = true;
                                                }
                                                Err(e) => {
                                                    *status = Some(StatusMessage {
                                                        text: format!("Failed to disable: {}", e),
                                                        is_error: true,
                                                    });
                                                }
                                            }
                                        }
                                    } else {
                                        if ui.button("Enable").clicked() {
                                            match mods::enable_mod(
                                                &server.path,
                                                &resource_folder,
                                                &mod_entry.relative_path,
                                            ) {
                                                Ok(_) => {
                                                    *status = Some(StatusMessage {
                                                        text: "Mod enabled".to_string(),
                                                        is_error: false,
                                                    });
                                                    needs_reload = true;
                                                }
                                                Err(e) => {
                                                    *status = Some(StatusMessage {
                                                        text: format!("Failed to enable: {}", e),
                                                        is_error: true,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                        });
                    }
                });
            }
        }
        None => {
            ui.label("Loading mods...");
        }
    }

    // Reload if needed after all UI is done
    if needs_reload {
        *mods_cache = None;
    }
}

