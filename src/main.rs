mod config;
mod server;
mod mods;
mod ui;
mod process;
mod settings;

use eframe::egui;
use server::{ServerEntry, ServerList};
use process::ServerProcess;
use settings::AppSettings;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};
use std::sync::mpsc::{channel, Receiver};

fn main() -> eframe::Result<()> {
    let settings = AppSettings::load();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("BeamMP Server Manager")
            .with_visible(!settings.start_minimized),
        ..Default::default()
    };

    eframe::run_native(
        "BeamMP Server Manager",
        options,
        Box::new(|cc| {
            // Setup system tray
            let tray_menu = Menu::new();
            let show_item = MenuItem::new("Show", true, None);
            let quit_item = MenuItem::new("Quit", true, None);
            tray_menu.append(&show_item).ok();
            tray_menu.append(&quit_item).ok();

            let icon_rgba = create_tray_icon();
            let icon = tray_icon::icon::Icon::from_rgba(icon_rgba.clone(), 32, 32)
                .expect("Failed to create tray icon");

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("BeamMP Server Manager")
                .with_icon(icon)
                .build()
                .expect("Failed to create tray icon");

            let menu_channel = MenuEvent::receiver();

            Ok(Box::new(BeamMpManagerApp::new(
                cc,
                tray,
                menu_channel,
                show_item.id(),
                quit_item.id(),
            )))
        }),
    )
}

fn create_tray_icon() -> Vec<u8> {
    // Create a simple 32x32 icon (blue circle)
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];
    
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - 16.0;
            let dy = y as f32 - 16.0;
            let dist = (dx * dx + dy * dy).sqrt();
            
            let idx = (y * size + x) * 4;
            if dist < 14.0 {
                rgba[idx] = 30;      // R
                rgba[idx + 1] = 144; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0;   // Transparent
            }
        }
    }
    
    rgba
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
    settings: AppSettings,
    _tray_icon: TrayIcon,
    menu_channel: Receiver<MenuEvent>,
    show_menu_id: tray_icon::menu::MenuId,
    quit_menu_id: tray_icon::menu::MenuId,
}

struct RunningProcess {
    server_id: String,
    process: ServerProcess,
}

#[derive(PartialEq)]
enum Tab {
    Config,
    Mods,
    Settings,
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
    fn new(
        _cc: &eframe::CreationContext,
        tray_icon: TrayIcon,
        menu_channel: Receiver<MenuEvent>,
        show_menu_id: tray_icon::menu::MenuId,
        quit_menu_id: tray_icon::menu::MenuId,
    ) -> Self {
        let server_list = ServerList::load().unwrap_or_default();
        let settings = AppSettings::load();
        
        Self {
            server_list,
            selected_server_index: None,
            current_tab: Tab::Config,
            status_message: None,
            mods_cache: None,
            delete_confirmation: None,
            running_process: None,
            terminal_output: Vec::new(),
            auto_scroll_terminal: true,
            settings,
            _tray_icon: tray_icon,
            menu_channel,
            show_menu_id,
            quit_menu_id,
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

    fn update_terminal(&mut self) {
        // Check if process is still running and read output
        if let Some(running) = &mut self.running_process {
            if !running.process.is_running() {
                self.terminal_output.push("Server process exited.".to_string());
                self.running_process = None;
            } else {
                let new_lines = running.process.read_output();
                self.terminal_output.extend(new_lines);
                
                // Limit terminal output to last 1000 lines
                if self.terminal_output.len() > 1000 {
                    self.terminal_output.drain(0..self.terminal_output.len() - 1000);
                }
            }
        }
    }
}

impl eframe::App for BeamMpManagerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle tray menu events
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.show_menu_id {
                frame.set_visible(true);
                frame.focus();
            } else if event.id == self.quit_menu_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Handle close request (X button)
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.settings.minimize_to_tray {
                // Minimize to tray instead of closing
                frame.set_visible(false);
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else {
                // Allow normal close
                // The app will exit
            }
        }

        // Update terminal output
        self.update_terminal();
        
        // Request continuous repaint when server is running
        if self.running_process.is_some() {
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

        // Terminal Panel at bottom
        egui::TopBottomPanel::bottom("terminal_panel")
            .resizable(true)
            .min_height(150.0)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Server Console");
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear").clicked() {
                            self.terminal_output.clear();
                        }
                        
                        ui.checkbox(&mut self.auto_scroll_terminal, "Auto-scroll");
                        
                        // Start/Stop buttons
                        if let Some(running) = &self.running_process {
                            if let Some(idx) = self.selected_server_index {
                                if let Some(server) = self.server_list.servers.get(idx) {
                                    if running.server_id == server.id {
                                        ui.colored_label(egui::Color32::GREEN, "● Running");
                                        if ui.button("Stop Server").clicked() {
                                            self.stop_server();
                                        }
                                    }
                                }
                            }
                        } else {
                            if let Some(idx) = self.selected_server_index {
                                if let Some(server) = self.server_list.servers.get(idx) {
                                    let server_id = server.id.clone();
                                    let server_path = server.path.clone();
                                    if ui.button("Start Server").clicked() {
                                        self.start_server(server_id, server_path);
                                    }
                                }
                            }
                        }
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
                        ui.selectable_value(&mut self.current_tab, Tab::Settings, "Settings");
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
                        Tab::Settings => {
                            ui::settings_tab::show(ui, &mut self.settings, &mut self.status_message);
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

