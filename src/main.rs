mod config;
mod server;
mod mods;
mod ui;
mod process;

use eframe::egui;
use server::ServerList;
use process::ServerProcess;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("BeamMP Panel"),
        ..Default::default()
    };

    eframe::run_native(
        "BeamMP Panel",
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
    running_process: Option<RunningProcess>,
    terminal_output: Vec<String>,
    auto_scroll_terminal: bool,
    player_list: Vec<String>,
    kick_player_name: String,
    kick_reason: String,
    broadcast_message: String,
}

struct RunningProcess {
    server_id: String,
    process: ServerProcess,
}

#[derive(PartialEq)]
enum Tab {
    Config,
    Mods,
    Control,
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
            running_process: None,
            terminal_output: Vec::with_capacity(1000), // Preallocate
            auto_scroll_terminal: true,
            player_list: Vec::with_capacity(32), // Preallocate for typical player counts
            kick_player_name: String::new(),
            kick_reason: String::new(),
            broadcast_message: String::new(),
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

    fn start_server(&mut self, server_id: String, server_path: std::path::PathBuf) {
        match ServerProcess::start(&server_path) {
            Ok(process) => {
                self.terminal_output.clear();
                self.terminal_output.push(format!("Starting server at {}...", server_path.display()));
                self.running_process = Some(RunningProcess {
                    server_id,
                    process,
                });
                self.set_status("Server started".to_string(), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to start server: {}", e), true);
            }
        }
    }

    fn stop_server(&mut self) {
        if let Some(mut running) = self.running_process.take() {
            match running.process.stop() {
                Ok(_) => {
                    self.terminal_output.push("Server stopped.".to_string());
                    self.set_status("Server stopped".to_string(), false);
                }
                Err(e) => {
                    self.set_status(format!("Failed to stop server: {}", e), true);
                }
            }
        }
    }

    fn update_terminal(&mut self) -> bool {
        // Check if process is still running and read output
        // Returns true if terminal was updated (for conditional repainting)
        if let Some(running) = &mut self.running_process {
            if !running.process.is_running() {
                self.terminal_output.push("Server process exited.".to_string());
                self.running_process = None;
                return true;
            } else {
                let new_lines = running.process.read_output();
                let has_new_output = !new_lines.is_empty();
                self.terminal_output.extend(new_lines);
                
                // Limit terminal output to last 1000 lines
                if self.terminal_output.len() > 1000 {
                    self.terminal_output.drain(0..self.terminal_output.len() - 1000);
                }
                return has_new_output;
            }
        }
        false
    }

    fn send_server_command(&mut self, command: &str) {
        if let Some(running) = &self.running_process {
            match running.process.send_command(command) {
                Ok(_) => {
                    self.terminal_output.push(format!("> {}", command));
                    self.set_status(format!("Command sent: {}", command), false);
                }
                Err(e) => {
                    self.set_status(format!("Failed to send command: {}", e), true);
                }
            }
        } else {
            self.set_status("No server is running".to_string(), true);
        }
    }

    fn refresh_player_list(&mut self) {
        self.player_list.clear();
        self.send_server_command("list");
        // Player list will be populated from terminal output parsing
        // For now, just trigger the command
    }
}

impl eframe::App for BeamMpManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update terminal output and check if there were changes
        let terminal_changed = self.update_terminal();
        
        // Only request repaint if terminal actually changed (optimization)
        if terminal_changed {
            ctx.request_repaint();
        }
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

                // Get server info without holding mutable borrow
                let server_info = self.server_list.servers.get(idx).map(|s| (s.id.clone(), s.path.clone()));
                
                if server_info.is_none() {
                    self.selected_server_index = None;
                } else {
                    let (server_id, server_path) = server_info.unwrap();
                    let is_running = self.running_process.as_ref()
                        .map(|r| r.server_id == server_id)
                        .unwrap_or(false);

                    // Track actions to perform after UI
                    let mut should_start = false;
                    let mut should_stop = false;
                    let mut should_clear_terminal = false;
                    let mut control_action = ui::control_tab::ControlAction::None;

                    // Top section with tabs and server controls
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.current_tab, Tab::Config, "Config");
                        ui.selectable_value(&mut self.current_tab, Tab::Mods, "Mods");
                        ui.selectable_value(&mut self.current_tab, Tab::Control, "Control");
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Start/Stop buttons
                            if is_running {
                                ui.colored_label(egui::Color32::GREEN, "● Running");
                                if ui.button("Stop Server").clicked() {
                                    should_stop = true;
                                }
                            } else {
                                if ui.button("Start Server").clicked() {
                                    should_start = true;
                                }
                            }
                        });
                    });
                    ui.separator();

                    // Main content area - split vertically if server is running
                    if is_running {
                        // Split view: tabs on top, terminal on bottom
                        egui::TopBottomPanel::bottom("server_terminal")
                            .resizable(true)
                            .min_height(150.0)
                            .default_height(250.0)
                            .show_inside(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.heading("Server Console");
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Clear").clicked() {
                                            should_clear_terminal = true;
                                        }
                                        
                                        ui.checkbox(&mut self.auto_scroll_terminal, "Auto-scroll");
                                    });
                                });
                                
                                ui.separator();
                                
                                let text_style = egui::TextStyle::Monospace;
                                let row_height = ui.text_style_height(&text_style);
                                
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .stick_to_bottom(self.auto_scroll_terminal)
                                    .show_rows(
                                        ui,
                                        row_height,
                                        self.terminal_output.len(),
                                        |ui, row_range| {
                                            for row in row_range {
                                                if let Some(line) = self.terminal_output.get(row) {
                                                    ui.label(egui::RichText::new(line).monospace());
                                                }
                                            }
                                        },
                                    );
                            });
                    }

                    // Tab content in remaining space
                    if let Some(server) = self.server_list.servers.get_mut(idx) {
                        egui::CentralPanel::default().show_inside(ui, |ui| {
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
                                Tab::Control => {
                                    control_action = ui::control_tab::show(
                                        ui,
                                        is_running,
                                        &mut self.player_list,
                                        &mut self.kick_player_name,
                                        &mut self.kick_reason,
                                        &mut self.broadcast_message,
                                    );
                                }
                            }
                        });
                    }

                    // Execute deferred actions
                    if should_start {
                        self.start_server(server_id, server_path);
                    }
                    if should_stop {
                        self.stop_server();
                    }
                    if should_clear_terminal {
                        self.terminal_output.clear();
                    }
                    
                    // Handle control tab actions
                    match control_action {
                        ui::control_tab::ControlAction::SendCommand(cmd) => {
                            self.send_server_command(&cmd);
                        }
                        ui::control_tab::ControlAction::RefreshPlayers => {
                            self.refresh_player_list();
                        }
                        ui::control_tab::ControlAction::None => {}
                    }
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

