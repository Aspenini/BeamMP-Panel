#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod server;
mod mods;
mod ui;
mod process;

use eframe::egui;
use server::ServerList;
use process::ServerProcess;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    // Load and set the window icon
    let icon = load_icon();
    
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 700.0])
        .with_title("BeamMP Panel");
    
    if let Some(icon_data) = icon {
        viewport_builder = viewport_builder.with_icon(Arc::new(icon_data));
    }
    
    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    eframe::run_native(
        "BeamMP Panel",
        options,
        Box::new(|_cc| Ok(Box::new(BeamMpManagerApp::new()))),
    )
}

fn load_icon() -> Option<egui::IconData> {
    // Load the embedded icon file
    let icon_bytes = include_bytes!("../icons/BeamMP-Panel.ico");
    
    // Try to load from embedded bytes
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}

struct BeamMpManagerApp {
    server_list: ServerList,
    selected_server_index: Option<usize>,
    current_tab: Tab,
    status_message: Option<StatusMessage>,
    mods_cache: Option<ModsCache>,
    current_mod_type: ModType,
    current_mod_filter: ModFilter,
    delete_confirmation: Option<DeleteConfirmation>,
    running_process: Option<RunningProcess>,
    terminal_output: Vec<String>,
    auto_scroll_terminal: bool,
    player_list: Vec<PlayerEntry>,
    player_list_request: Option<PlayerListRequest>,
    kick_player_name: String,
    kick_reason: String,
    broadcast_message: String,
    mod_details_view: Option<ModDetailsView>,
}

struct ModDetailsView {
    mod_name: String,
    details: Option<mods::ModDetailInfo>,
}

struct RunningProcess {
    server_id: String,
    process: ServerProcess,
    /// Set when a graceful stop was requested; force-kill after this deadline.
    stop_deadline: Option<std::time::Instant>,
}

/// A connected player parsed from the server's `list` command output.
#[derive(Clone)]
pub struct PlayerEntry {
    pub name: String,
    pub id: String,
    pub cars: String,
}

/// Tracks an in-flight `list` command whose output we are watching for.
struct PlayerListRequest {
    started: std::time::Instant,
    /// True once the "Name  ID  Cars" table header has been seen.
    capturing: bool,
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
    mod_type: ModType,
}

#[derive(PartialEq, Clone, Copy)]
enum ModType {
    Server,
    Client,
}

#[derive(PartialEq, Clone, Copy)]
enum ModFilter {
    All,
    LevelsOnly,
    VehiclesOnly,
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
            current_mod_type: ModType::Client, // Default to Client mods
            current_mod_filter: ModFilter::All,
            delete_confirmation: None,
            running_process: None,
            terminal_output: Vec::with_capacity(1000), // Preallocate
            auto_scroll_terminal: true,
            player_list: Vec::with_capacity(32), // Preallocate for typical player counts
            player_list_request: None,
            kick_player_name: String::new(),
            kick_reason: String::new(),
            broadcast_message: String::new(),
            mod_details_view: None,
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

    fn reload_mods(&mut self, mod_type: ModType) {
        if let Some(idx) = self.selected_server_index {
            if let Some(server) = self.server_list.servers.get(idx) {
                let resource_folder = server.get_resource_folder();
                let result = match mod_type {
                    ModType::Server => mods::scan_server_mods(&server.path, &resource_folder),
                    ModType::Client => mods::scan_client_mods(&server.path, &resource_folder),
                };
                
                match result {
                    Ok(mods) => {
                        self.mods_cache = Some(ModsCache {
                            server_id: server.id.clone(),
                            mods,
                            mod_type,
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
                    stop_deadline: None,
                });
                self.set_status("Server started".to_string(), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to start server: {}", e), true);
            }
        }
    }

    fn stop_server(&mut self) {
        // Request a graceful shutdown without blocking the UI thread;
        // update_terminal() force-kills if the deadline passes.
        if let Some(running) = &mut self.running_process {
            if running.stop_deadline.is_none() {
                let _ = running.process.send_command("exit");
                running.stop_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                self.terminal_output.push("Stopping server...".to_string());
                self.set_status("Stopping server...".to_string(), false);
            }
        }
    }

    fn update_terminal(&mut self) {
        // Drain process output and detect exit / stop timeout
        if let Some(running) = &mut self.running_process {
            if !running.process.is_running() {
                self.terminal_output.push("Server process exited.".to_string());
                self.running_process = None;
                self.player_list.clear();
                self.player_list_request = None;
            } else {
                if running
                    .stop_deadline
                    .is_some_and(|deadline| std::time::Instant::now() >= deadline)
                {
                    let _ = running.process.kill();
                    self.terminal_output
                        .push("Server did not exit in time; force-killed.".to_string());
                    self.running_process = None;
                    return;
                }

                let new_lines = running.process.read_output();
                self.process_player_list_lines(&new_lines);
                self.terminal_output.extend(new_lines);

                // Limit terminal output to last 1000 lines
                if self.terminal_output.len() > 1000 {
                    self.terminal_output.drain(0..self.terminal_output.len() - 1000);
                }
            }
        }

        // The player table has no terminator line, so if the console goes
        // quiet, finalize (or abandon) the request after a short window.
        if let Some(request) = &self.player_list_request {
            if request.started.elapsed() > std::time::Duration::from_secs(2) {
                let was_capturing = request.capturing;
                self.player_list_request = None;
                if was_capturing {
                    self.set_status(format!("{} player(s) online", self.player_list.len()), false);
                }
            }
        }
    }

    /// Scan console output for the response to a pending `list` command.
    ///
    /// BeamMP-Server replies either with "No players online." or a
    /// fixed-width table: a "Name  ID  Cars" header followed by one row per
    /// player. Rows are parsed from the right (id and cars are numeric) so
    /// player names containing spaces survive.
    fn process_player_list_lines(&mut self, lines: &[String]) {
        for line in lines {
            let Some(request) = &self.player_list_request else {
                return;
            };
            let trimmed = line.trim();

            if !request.capturing {
                if trimmed == "No players online." {
                    self.player_list.clear();
                    self.player_list_request = None;
                    self.set_status("No players online".to_string(), false);
                } else if trimmed.split_whitespace().eq(["Name", "ID", "Cars"]) {
                    self.player_list.clear();
                    if let Some(request) = &mut self.player_list_request {
                        request.capturing = true;
                    }
                }
            } else if let Some(entry) = parse_player_row(trimmed) {
                self.player_list.push(entry);
            } else {
                // First non-row line marks the end of the table
                self.player_list_request = None;
                self.set_status(format!("{} player(s) online", self.player_list.len()), false);
            }
        }
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
        if self.running_process.is_none() {
            self.set_status("No server is running".to_string(), true);
            return;
        }
        self.player_list.clear();
        self.send_server_command("list");
        self.player_list_request = Some(PlayerListRequest {
            started: std::time::Instant::now(),
            capturing: false,
        });
    }
}

/// Parse one row of the `list` table: `<name (may contain spaces)> <id> <cars>`.
fn parse_player_row(line: &str) -> Option<PlayerEntry> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    let (cars, id) = (tokens[tokens.len() - 1], tokens[tokens.len() - 2]);
    if cars.parse::<u32>().is_err() || id.parse::<u32>().is_err() {
        return None;
    }
    Some(PlayerEntry {
        name: tokens[..tokens.len() - 2].join(" "),
        id: id.to_string(),
        cars: cars.to_string(),
    })
}

impl eframe::App for BeamMpManagerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_terminal();

        // Keep polling for new console output while a server is running,
        // otherwise egui only repaints on user input and output stalls.
        if self.running_process.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;

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
                            let mod_type = cache.mod_type;
                            if let Some(mod_entry) = cache.mods.get(idx) {
                                if let Err(e) = mods::delete_mod(&mod_entry.full_path) {
                                    self.set_status(format!("Failed to delete mod: {}", e), true);
                                } else {
                                    self.set_status("Mod deleted".to_string(), false);
                                    self.reload_mods(mod_type);
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
        }

        // Handle mod details modal
        if let Some(details_view) = &self.mod_details_view {
            let mut should_close = false;

            egui::Window::new("Mod Details")
                .collapsible(false)
                .resizable(true)
                .default_width(500.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading(&details_view.mod_name);
                    ui.separator();
                    
                    if let Some(details) = &details_view.details {
                        ui.horizontal(|ui| {
                            ui.label("Type:");
                            if details.has_levels {
                                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), "Level");
                            }
                            if details.has_vehicles {
                                ui.colored_label(egui::Color32::from_rgb(255, 180, 100), "Vehicle");
                            }
                            if !details.has_levels && !details.has_vehicles {
                                ui.label("Other");
                            }
                        });
                        
                        ui.separator();
                        
                        ui.label(format!("Total Files: {}", details.total_files));
                        ui.label(format!("Total Size: {:.2} MB", details.total_size as f64 / 1_048_576.0));
                        
                        ui.separator();
                        
                        if !details.level_names.is_empty() {
                            ui.heading("Levels:");
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    for level_name in &details.level_names {
                                        ui.label(format!("  • {}", level_name));
                                    }
                                });
                            ui.add_space(10.0);
                        }
                        
                        if !details.vehicle_names.is_empty() {
                            ui.heading("Vehicles:");
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    for vehicle_name in &details.vehicle_names {
                                        ui.label(format!("  • {}", vehicle_name));
                                    }
                                });
                        }
                    } else {
                        ui.label("Loading details...");
                    }
                    
                    ui.separator();
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });

            if should_close {
                self.mod_details_view = None;
            }
        }

        egui::Panel::bottom("status_bar").show(ui, |ui| {
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

        egui::Panel::left("servers_panel")
            .resizable(true)
            .min_size(250.0)
            .show(ui, |ui| {
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
                            response.on_hover_text(server.path.display().to_string());
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

        egui::CentralPanel::default().show(ui, |ui| {
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
                    // Use the current mod type view
                    self.reload_mods(self.current_mod_type);
                }

                // Get server info without holding mutable borrow
                let server_info = self.server_list.servers.get(idx).map(|s| (s.id.clone(), s.path.clone()));

                if server_info.is_none() {
                    self.selected_server_index = None;
                } else if let Some((server_id, server_path)) = server_info {
                    let is_running = self.running_process.as_ref()
                        .is_some_and(|r| r.server_id == server_id);
                    let is_stopping = self.running_process.as_ref()
                        .is_some_and(|r| r.server_id == server_id && r.stop_deadline.is_some());

                    // Track actions to perform after UI
                    let mut should_start = false;
                    let mut should_stop = false;
                    let mut should_clear_terminal = false;
                    let mut control_action = ui::control_tab::ControlAction::None;
                    let mut mods_action = ui::mods_tab::ModsAction::None;

                    // Top section with tabs and server controls
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.current_tab, Tab::Config, "Config");
                        ui.selectable_value(&mut self.current_tab, Tab::Mods, "Mods");
                        ui.selectable_value(&mut self.current_tab, Tab::Control, "Control");
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Start/Stop buttons
                            if is_stopping {
                                ui.colored_label(egui::Color32::YELLOW, "● Stopping...");
                            } else if is_running {
                                ui.colored_label(egui::Color32::GREEN, "● Running");
                                if ui.button("Stop Server").clicked() {
                                    should_stop = true;
                                }
                            } else if ui.button("Start Server").clicked() {
                                should_start = true;
                            }
                        });
                    });
                    ui.separator();

                    // Main content area - split vertically if server is running
                    if is_running {
                        // Split view: tabs on top, terminal on bottom
                        egui::Panel::bottom("server_terminal")
                            .resizable(true)
                            .min_size(150.0)
                            .default_size(250.0)
                            .show(ui, |ui| {
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
                        egui::CentralPanel::default().show(ui, |ui| {
                            match self.current_tab {
                                Tab::Config => {
                                    ui::config_tab::show(ui, server, &mut self.status_message);
                                }
                                Tab::Mods => {
                                    mods_action = ui::mods_tab::show(
                                        ui,
                                        server,
                                        &mut self.mods_cache,
                                        self.current_mod_type,
                                        self.current_mod_filter,
                                        &mut self.status_message,
                                        &mut self.delete_confirmation,
                                    );
                                }
                                Tab::Control => {
                                    control_action = ui::control_tab::show(
                                        ui,
                                        is_running,
                                        &self.player_list,
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
                    
                    // Handle mods tab actions
                    match mods_action {
                        ui::mods_tab::ModsAction::SwitchToServer => {
                            self.current_mod_type = ModType::Server;
                            self.reload_mods(ModType::Server);
                        }
                        ui::mods_tab::ModsAction::SwitchToClient => {
                            self.current_mod_type = ModType::Client;
                            self.reload_mods(ModType::Client);
                        }
                        ui::mods_tab::ModsAction::ViewDetails(mod_idx) => {
                            if let Some(cache) = &self.mods_cache {
                                if let Some(mod_entry) = cache.mods.get(mod_idx) {
                                    // Get mod details
                                    match mods::get_mod_details(&mod_entry.full_path) {
                                        Ok(details) => {
                                            self.mod_details_view = Some(ModDetailsView {
                                                mod_name: mod_entry.relative_path.clone(),
                                                details: Some(details),
                                            });
                                        }
                                        Err(e) => {
                                            self.set_status(format!("Failed to read mod details: {}", e), true);
                                        }
                                    }
                                }
                            }
                        }
                        ui::mods_tab::ModsAction::ChangeFilter(new_filter) => {
                            self.current_mod_filter = new_filter;
                        }
                        ui::mods_tab::ModsAction::None => {}
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


#[cfg(test)]
mod tests {
    use super::parse_player_row;

    #[test]
    fn parses_simple_row() {
        // Real format: name padded to 25, id and cars padded to 6
        let entry = parse_player_row("SomePlayer               0     1     ".trim()).unwrap();
        assert_eq!(entry.name, "SomePlayer");
        assert_eq!(entry.id, "0");
        assert_eq!(entry.cars, "1");
    }

    #[test]
    fn parses_name_with_spaces() {
        let entry = parse_player_row("John Doe Racing          12    3").unwrap();
        assert_eq!(entry.name, "John Doe Racing");
        assert_eq!(entry.id, "12");
        assert_eq!(entry.cars, "3");
    }

    #[test]
    fn rejects_non_row_lines() {
        assert!(parse_player_row("No players online.").is_none());
        assert!(parse_player_row("[16:20:01] [INFO] something happened").is_none());
        assert!(parse_player_row("").is_none());
        // Header has no trailing numbers
        assert!(parse_player_row("Name                     ID    Cars").is_none());
    }
}
