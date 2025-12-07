mod config;
mod server;
mod mods;
mod ui;

use eframe::egui;
use server::{ServerEntry, ServerList};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("BeamMP Server Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "BeamMP Server Manager",
        options,
        Box::new(|_cc| Ok(Box::new(BeamMpManagerApp::new()))),
    )
}

struct BeamMpManagerApp {
    server_list: ServerList,
    selected_server_index: Option<usize>,
    current_tab: Tab,
    status_message: Option<StatusMessage>,
    mods_cache: Option<ModsCache>,
    delete_confirmation: Option<DeleteConfirmation>,
}

#[derive(PartialEq)]
enum Tab {
    Config,
    Mods,
}

struct StatusMessage {
    text: String,
    is_error: bool,
}

struct ModsCache {
    server_id: String,
    mods: Vec<mods::ModEntry>,
}

enum DeleteConfirmation {
    Server(usize),
    Mod(usize),
}

impl BeamMpManagerApp {
    fn new() -> Self {
        let server_list = ServerList::load().unwrap_or_default();
        
        Self {
            server_list,
            selected_server_index: None,
            current_tab: Tab::Config,
            status_message: None,
            mods_cache: None,
            delete_confirmation: None,
        }
    }

    fn set_status(&mut self, text: String, is_error: bool) {
        self.status_message = Some(StatusMessage { text, is_error });
    }

    fn add_server(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            match self.server_list.add_server(path) {
                Ok(name) => {
                    self.set_status(format!("Added server: {}", name), false);
                    if let Err(e) = self.server_list.save() {
                        self.set_status(format!("Failed to save server list: {}", e), true);
                    }
                }
                Err(e) => {
                    self.set_status(format!("Failed to add server: {}", e), true);
                }
            }
        }
    }

    fn remove_selected_server(&mut self) {
        if let Some(idx) = self.selected_server_index {
            self.server_list.remove_server(idx);
            self.selected_server_index = None;
            self.mods_cache = None;
            if let Err(e) = self.server_list.save() {
                self.set_status(format!("Failed to save server list: {}", e), true);
            } else {
                self.set_status("Server removed from manager".to_string(), false);
            }
        }
    }

    fn get_selected_server_mut(&mut self) -> Option<&mut ServerEntry> {
        self.selected_server_index
            .and_then(|idx| self.server_list.servers.get_mut(idx))
    }

    fn reload_mods(&mut self) {
        if let Some(idx) = self.selected_server_index {
            if let Some(server) = self.server_list.servers.get(idx) {
                match mods::scan_mods(&server.path, &server.get_resource_folder()) {
                    Ok(mods) => {
                        self.mods_cache = Some(ModsCache {
                            server_id: server.id.clone(),
                            mods,
                        });
                    }
                    Err(e) => {
                        self.set_status(format!("Failed to scan mods: {}", e), true);
                        self.mods_cache = None;
                    }
                }
            }
        }
    }
}

impl eframe::App for BeamMpManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle delete confirmation modal
        if let Some(confirmation) = &self.delete_confirmation {
            let mut should_close = false;
            let mut should_confirm = false;

            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    match confirmation {
                        DeleteConfirmation::Server(_) => {
                            ui.label("Remove this server from the manager?");
                            ui.label("This will NOT delete any files.");
                        }
                        DeleteConfirmation::Mod(idx) => {
                            if let Some(cache) = &self.mods_cache {
                                if let Some(mod_entry) = cache.mods.get(*idx) {
                                    ui.label("Delete this mod from disk?");
                                    ui.label(&mod_entry.relative_path);
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        "This cannot be undone!",
                                    );
                                }
                            }
                        }
                    }
                    
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                        if ui.button("Confirm").clicked() {
                            should_confirm = true;
                        }
                    });
                });

            if should_close {
                self.delete_confirmation = None;
            }
            if should_confirm {
                match self.delete_confirmation.take() {
                    Some(DeleteConfirmation::Server(idx)) => {
                        self.selected_server_index = Some(idx);
                        self.remove_selected_server();
                    }
                    Some(DeleteConfirmation::Mod(idx)) => {
                        if let Some(cache) = &self.mods_cache {
                            if let Some(mod_entry) = cache.mods.get(idx) {
                                if let Err(e) = mods::delete_mod(&mod_entry.full_path) {
                                    self.set_status(format!("Failed to delete mod: {}", e), true);
                                } else {
                                    self.set_status("Mod deleted".to_string(), false);
                                    self.reload_mods();
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(msg) = &self.status_message {
                    let color = if msg.is_error {
                        egui::Color32::RED
                    } else {
                        egui::Color32::GREEN
                    };
                    ui.colored_label(color, &msg.text);
                }
            });
        });

        egui::SidePanel::left("servers_panel")
            .min_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Servers");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, server) in self.server_list.servers.iter().enumerate() {
                        let is_selected = self.selected_server_index == Some(idx);
                        let response = ui.selectable_label(is_selected, &server.name);
                        
                        if response.clicked() {
                            self.selected_server_index = Some(idx);
                            self.mods_cache = None;
                        }

                        if response.hovered() {
                            response.on_hover_text(&server.path.display().to_string());
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Add Server").clicked() {
                        self.add_server();
                    }
                    
                    if ui.button("Remove Server").clicked() {
                        if let Some(idx) = self.selected_server_index {
                            self.delete_confirmation = Some(DeleteConfirmation::Server(idx));
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = self.selected_server_index {
                // Check if we need to reload mods before borrowing
                let should_reload_mods = if self.current_tab == Tab::Mods {
                    if let Some(server) = self.server_list.servers.get(idx) {
                        self.mods_cache.is_none() || 
                        self.mods_cache.as_ref().map(|c| &c.server_id) != Some(&server.id)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_reload_mods {
                    self.reload_mods();
                }

                if let Some(server) = self.server_list.servers.get_mut(idx) {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.current_tab, Tab::Config, "Config");
                        ui.selectable_value(&mut self.current_tab, Tab::Mods, "Mods");
                    });
                    ui.separator();

                    match self.current_tab {
                        Tab::Config => {
                            ui::config_tab::show(ui, server, &mut self.status_message);
                        }
                        Tab::Mods => {
                            ui::mods_tab::show(
                                ui,
                                server,
                                &mut self.mods_cache,
                                &mut self.status_message,
                                &mut self.delete_confirmation,
                            );
                        }
                    }
                } else {
                    self.selected_server_index = None;
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.heading("No server selected");
                    ui.label("Select a server from the list or add a new one.");
                });
            }
        });
    }
}

