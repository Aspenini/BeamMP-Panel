use crate::mods;
use crate::server::ServerEntry;
use crate::{DeleteConfirmation, ModsCache, StatusMessage, ModType, ModFilter};
use egui::{ScrollArea, Ui};

pub enum ModsAction {
    None,
    SwitchToServer,
    SwitchToClient,
    ViewDetails(usize), // Index of the mod to view details for
    ChangeFilter(ModFilter),
}

pub fn show(
    ui: &mut Ui,
    server: &ServerEntry,
    mods_cache: &mut Option<ModsCache>,
    current_mod_type: ModType,
    current_mod_filter: ModFilter,
    status: &mut Option<StatusMessage>,
    delete_confirmation: &mut Option<DeleteConfirmation>,
) -> ModsAction {
    let mut action = ModsAction::None;
    
    // Mod type selector
    ui.horizontal(|ui| {
        ui.label("View:");
        if ui.selectable_label(current_mod_type == ModType::Client, "📦 Client").clicked()
            && current_mod_type != ModType::Client
        {
            action = ModsAction::SwitchToClient;
        }
        if ui.selectable_label(current_mod_type == ModType::Server, "📁 Server").clicked()
            && current_mod_type != ModType::Server
        {
            action = ModsAction::SwitchToServer;
        }

        // Show filter options only for Client mods
        if current_mod_type == ModType::Client {
            ui.separator();
            ui.label("Filter:");
            for (filter, label) in [
                (ModFilter::All, "All"),
                (ModFilter::LevelsOnly, "Levels"),
                (ModFilter::VehiclesOnly, "Vehicles"),
            ] {
                if ui.selectable_label(current_mod_filter == filter, label).clicked()
                    && current_mod_filter != filter
                {
                    action = ModsAction::ChangeFilter(filter);
                }
            }
        }
    });
    
    ui.separator();
    
    ui.horizontal(|ui| {
        // Only show Add Mod button for Client mods
        if current_mod_type == ModType::Client {
            if ui.button("Add Client Mod...").clicked() {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("ZIP files", &["zip"])
                    .pick_files() 
                {
                    let mut added_count = 0;
                    let mut errors = Vec::new();

                    for file in files {
                        let resource_folder = server.get_resource_folder();
                        match mods::add_client_mod(&server.path, &resource_folder, &file) {
                            Ok(_) => added_count += 1,
                            Err(e) => errors.push(format!("{}: {}", file.display(), e)),
                        }
                    }

                    if added_count > 0 {
                        *status = Some(StatusMessage {
                            text: format!("Added {} client mod(s)", added_count),
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
            
            ui.label("ℹ Client mods must be ZIP files");
        } else {
            ui.label("ℹ Server mods are folders - add them manually to Resources/Server/");
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
            // Apply filter for client mods
            let filtered_mods: Vec<(usize, &mods::ModEntry)> = if current_mod_type == ModType::Client {
                cache.mods.iter().enumerate().filter(|(_, mod_entry)| {
                    match current_mod_filter {
                        ModFilter::All => true,
                        ModFilter::LevelsOnly => mod_entry.is_level,
                        ModFilter::VehiclesOnly => mod_entry.is_vehicle,
                    }
                }).collect()
            } else {
                cache.mods.iter().enumerate().collect()
            };
            
            if filtered_mods.is_empty() {
                ui.label("No mods found");
            } else {
                ui.label(format!("Showing: {} / Total: {}", filtered_mods.len(), cache.mods.len()));
                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    for (idx, mod_entry) in filtered_mods {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                {
                                    // Both Server and Client mods show enable/disable
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
                                    
                                    // Show icon based on mod type
                                    let icon = if current_mod_type == ModType::Server {
                                        "📁" // Folder for server mods
                                    } else {
                                        "📦" // Package for client mods (ZIP)
                                    };
                                    ui.label(icon);
                                    
                                    // Show level indicator for client mods
                                    if current_mod_type == ModType::Client && mod_entry.is_level {
                                        ui.colored_label(egui::Color32::from_rgb(100, 200, 255), "Level");
                                    }
                                    
                                    // Show vehicle indicator for client mods
                                    if current_mod_type == ModType::Client && mod_entry.is_vehicle {
                                        ui.colored_label(egui::Color32::from_rgb(255, 180, 100), "Vehicle");
                                    }
                                    
                                    ui.label(&mod_entry.relative_path);

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Delete").clicked() {
                                            *delete_confirmation = Some(DeleteConfirmation::Mod(idx));
                                        }
                                        
                                        // Show Info button only for client mods
                                        if current_mod_type == ModType::Client
                                            && ui.button("Info").clicked()
                                        {
                                            action = ModsAction::ViewDetails(idx);
                                        }

                                        let resource_folder = server.get_resource_folder();

                                        let (label, enable, verb) = if mod_entry.enabled {
                                            ("Disable", false, "disabled")
                                        } else {
                                            ("Enable", true, "enabled")
                                        };

                                        if ui.button(label).clicked() {
                                            let subdir = match current_mod_type {
                                                ModType::Server => "Server",
                                                ModType::Client => "Client",
                                            };
                                            match mods::set_mod_enabled(
                                                &server.path,
                                                &resource_folder,
                                                subdir,
                                                &mod_entry.relative_path,
                                                enable,
                                            ) {
                                                Ok(_) => {
                                                    *status = Some(StatusMessage {
                                                        text: format!("Mod {}", verb),
                                                        is_error: false,
                                                    });
                                                    needs_reload = true;
                                                }
                                                Err(e) => {
                                                    *status = Some(StatusMessage {
                                                        text: format!("Failed to {}: {}", label.to_lowercase(), e),
                                                        is_error: true,
                                                    });
                                                }
                                            }
                                        }
                                    });
                                }
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
    
    action
}

