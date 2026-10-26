use crate::console::Console;
use crate::mods::{self, ContentCache, ModDetailInfo, ModEntry, ModType};
use crate::server::ServerList;
use crate::session::{Event, ServerSession, State};
use crate::settings::Settings;
use crate::tray::{self, Tray};
use crate::ui;
use eframe::egui;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Requests from outside the UI thread: the tray icon and later launches.
pub enum AppCommand {
    Show,
    Quit,
}

/// Results of background work, delivered back to the UI thread.
enum TaskResult {
    ModsScanned {
        generation: u64,
        result: Result<Vec<ModEntry>, String>,
    },
    ModDetails(Result<ModDetailInfo, String>),
    ModsAdded {
        server_id: String,
        added: usize,
        errors: Vec<String>,
    },
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Config,
    Mods,
    Control,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ModFilter {
    All,
    Levels,
    Vehicles,
}

pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

pub enum Dialog {
    RemoveServer {
        id: String,
        name: String,
    },
    DeleteMod {
        path: PathBuf,
        name: String,
    },
    /// `None` while the details are being read
    ModDetails(Option<ModDetailInfo>),
    ConfirmQuit,
}

/// Mods tab state for the selected server.
pub struct ModsView {
    pub mod_type: ModType,
    pub filter: ModFilter,
    pub search: String,
    /// Which server/type `mods` belongs to
    pub loaded_for: Option<(String, ModType)>,
    pub mods: Vec<ModEntry>,
    pub loading: bool,
    generation: u64,
    /// Client mod copies in progress
    pub copying: usize,
}

enum QuitState {
    Running,
    /// Waiting for servers to stop before closing.
    StoppingServers {
        deadline: Instant,
    },
    /// Close for real; don't intercept the close request.
    Closing,
}

pub struct App {
    pub servers: ServerList,
    pub sessions: HashMap<String, ServerSession>,
    pub selected: Option<String>,
    pub tab: Tab,
    pub settings: Settings,
    pub settings_open: bool,
    pub status: Option<StatusMessage>,
    pub mods: ModsView,
    pub dialog: Option<Dialog>,
    pub show_auth_key: bool,
    // Control tab form
    pub kick_name: String,
    pub kick_reason: String,
    pub broadcast: String,

    ctx: egui::Context,
    tray: Option<Tray>,
    commands_rx: Receiver<AppCommand>,
    tasks_tx: Sender<TaskResult>,
    tasks_rx: Receiver<TaskResult>,
    content_cache: Arc<ContentCache>,
    window_hidden: bool,
    quit: QuitState,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        icon: Option<egui::IconData>,
        instance_listener: Option<TcpListener>,
        start_minimized: bool,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let settings = Settings::load();
        ui::theme::install(&ctx);
        ctx.set_theme(settings.theme.preference());

        let (commands_tx, commands_rx) = channel();
        let (tasks_tx, tasks_rx) = channel();

        let tray = match Tray::new(&ctx, icon.as_ref(), commands_tx.clone()) {
            Ok(tray) => Some(tray),
            Err(e) => {
                if tray::SUPPORTED {
                    eprintln!("Failed to create tray icon: {e}");
                }
                None
            }
        };
        if let Some(listener) = instance_listener {
            crate::instance::listen(listener, ctx.clone(), commands_tx);
        }

        let (servers, status) = match ServerList::load() {
            Ok(list) => (list, None),
            Err(e) => (
                ServerList::default(),
                Some(StatusMessage {
                    text: format!("Failed to load server list: {e}"),
                    is_error: true,
                }),
            ),
        };

        let mut app = Self {
            selected: servers.servers.first().map(|s| s.id.clone()),
            servers,
            sessions: HashMap::new(),
            tab: Tab::Config,
            settings,
            settings_open: false,
            status,
            mods: ModsView {
                mod_type: ModType::Client,
                filter: ModFilter::All,
                search: String::new(),
                loaded_for: None,
                mods: Vec::new(),
                loading: false,
                generation: 0,
                copying: 0,
            },
            dialog: None,
            show_auth_key: false,
            kick_name: String::new(),
            kick_reason: String::new(),
            broadcast: String::new(),
            ctx,
            tray,
            commands_rx,
            tasks_tx,
            tasks_rx,
            content_cache: Arc::default(),
            window_hidden: false,
            quit: QuitState::Running,
        };

        // Hiding only makes sense if there is a tray icon to bring it back.
        if (start_minimized || app.settings.start_minimized) && app.tray.is_some() {
            app.hide_window();
        }

        let auto_start: Vec<String> = app
            .servers
            .servers
            .iter()
            .filter(|s| s.auto_start)
            .map(|s| s.id.clone())
            .collect();
        for id in auto_start {
            app.start_server(&id);
        }

        app
    }

    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status = Some(StatusMessage {
            text: text.into(),
            is_error,
        });
    }

    pub fn tray_available(&self) -> bool {
        self.tray.is_some()
    }

    pub fn running_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_running()).count()
    }

    pub fn state_of(&self, id: &str) -> State {
        self.sessions.get(id).map_or(State::Stopped, |s| s.state())
    }

    pub fn save_server_list(&mut self) {
        if let Err(e) = self.servers.save() {
            self.set_status(format!("Failed to save server list: {e}"), true);
        }
    }

    pub fn save_settings(&mut self) {
        self.ctx.set_theme(self.settings.theme.preference());
        for session in self.sessions.values() {
            session
                .console
                .set_max_lines(self.settings.console_max_lines);
        }
        if let Err(e) = self.settings.save() {
            self.set_status(format!("Failed to save settings: {e}"), true);
        }
    }

    // ---------------------------------------------------------------- servers

    pub fn add_server(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Select a BeamMP server folder")
            .pick_folder()
        else {
            return;
        };
        match self.servers.add_server(path) {
            Ok(id) => {
                let name = self
                    .servers
                    .get(&id)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                self.selected = Some(id);
                self.set_status(format!("Added server: {name}"), false);
                self.save_server_list();
            }
            Err(e) => self.set_status(format!("Failed to add server: {e}"), true),
        }
    }

    pub fn remove_server(&mut self, id: &str) {
        if self.sessions.get(id).is_some_and(|s| s.is_running()) {
            self.set_status("Stop the server before removing it", true);
            return;
        }
        self.sessions.remove(id);
        if self.servers.remove_server(id).is_some() {
            if self.selected.as_deref() == Some(id) {
                self.selected = self.servers.servers.first().map(|s| s.id.clone());
            }
            self.set_status(
                "Server removed from the panel (files were not deleted)",
                false,
            );
            self.save_server_list();
        }
    }

    fn session_mut(&mut self, id: &str) -> &mut ServerSession {
        let (ctx, max_lines) = (&self.ctx, self.settings.console_max_lines);
        self.sessions
            .entry(id.to_string())
            .or_insert_with(|| ServerSession::new(Console::new(ctx.clone(), max_lines)))
    }

    pub fn start_server(&mut self, id: &str) {
        let Some(server) = self.servers.get(id) else {
            return;
        };
        let (path, name) = (server.path.clone(), server.name.clone());
        match self.session_mut(id).start(&path) {
            Ok(()) => self.set_status(format!("Started {name}"), false),
            Err(e) => self.set_status(format!("Failed to start {name}: {e}"), true),
        }
    }

    pub fn stop_server(&mut self, id: &str) {
        let timeout = self.settings.stop_timeout();
        if let Some(session) = self.sessions.get_mut(id) {
            session.stop(timeout);
        }
    }

    pub fn restart_server(&mut self, id: &str) {
        let timeout = self.settings.stop_timeout();
        if let Some(session) = self.sessions.get_mut(id) {
            session.restart(timeout);
        }
    }

    pub fn force_stop_server(&mut self, id: &str) {
        if let Some(session) = self.sessions.get_mut(id) {
            session.force_stop();
        }
    }

    pub fn send_command(&mut self, id: &str, command: &str) {
        let result = match self.sessions.get_mut(id) {
            Some(session) => session.send_command(command),
            None => Err(anyhow::anyhow!("Server is not running")),
        };
        if let Err(e) = result {
            self.set_status(format!("Failed to send command: {e}"), true);
        }
    }

    pub fn refresh_players(&mut self, id: &str) {
        let result = match self.sessions.get_mut(id) {
            Some(session) => session.request_player_list(),
            None => Err(anyhow::anyhow!("Server is not running")),
        };
        if let Err(e) = result {
            self.set_status(format!("Failed to list players: {e}"), true);
        }
    }

    // ------------------------------------------------------------------- mods

    fn spawn_task(&self, task: impl FnOnce() -> TaskResult + Send + 'static) {
        let (tx, ctx) = (self.tasks_tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(task());
            ctx.request_repaint();
        });
    }

    /// Rescan the selected server's mods in the background.
    pub fn reload_mods(&mut self) {
        let Some(server) = self.selected.as_deref().and_then(|id| self.servers.get(id)) else {
            return;
        };
        let (path, folder) = (server.path.clone(), server.resource_folder().to_string());
        let mod_type = self.mods.mod_type;
        let target = (server.id.clone(), mod_type);

        self.mods.generation += 1;
        let generation = self.mods.generation;
        if self.mods.loaded_for.as_ref() != Some(&target) {
            // Don't show another server's (or type's) mods while loading
            self.mods.mods.clear();
            self.mods.loaded_for = Some(target);
        }
        self.mods.loading = true;

        let cache = Arc::clone(&self.content_cache);
        self.spawn_task(move || TaskResult::ModsScanned {
            generation,
            result: mods::scan(&path, &folder, mod_type, &cache).map_err(|e| e.to_string()),
        });
    }

    /// Rescan if the mods tab is showing a different server or type.
    pub fn ensure_mods_loaded(&mut self) {
        let Some(id) = &self.selected else { return };
        let wanted = (id.clone(), self.mods.mod_type);
        if self.mods.loaded_for.as_ref() != Some(&wanted) {
            self.reload_mods();
        }
    }

    pub fn show_mod_details(&mut self, path: PathBuf) {
        self.dialog = Some(Dialog::ModDetails(None));
        self.spawn_task(move || {
            TaskResult::ModDetails(mods::mod_details(&path).map_err(|e| e.to_string()))
        });
    }

    pub fn add_client_mods(&mut self, files: Vec<PathBuf>) {
        let Some(server) = self.selected.as_deref().and_then(|id| self.servers.get(id)) else {
            return;
        };
        if files.is_empty() {
            return;
        }
        let (server_id, path, folder) = (
            server.id.clone(),
            server.path.clone(),
            server.resource_folder().to_string(),
        );
        self.mods.copying += 1;
        self.set_status(format!("Copying {} mod(s)...", files.len()), false);
        self.spawn_task(move || {
            let mut added = 0;
            let mut errors = Vec::new();
            for file in files {
                match mods::add_client_mod(&path, &folder, &file) {
                    Ok(()) => added += 1,
                    Err(e) => errors.push(format!(
                        "{}: {e}",
                        file.file_name().unwrap_or_default().to_string_lossy()
                    )),
                }
            }
            TaskResult::ModsAdded {
                server_id,
                added,
                errors,
            }
        });
    }

    pub fn set_mod_enabled(&mut self, index: usize, enabled: bool) {
        let Some(server) = self.selected.as_deref().and_then(|id| self.servers.get(id)) else {
            return;
        };
        let Some(entry) = self.mods.mods.get(index) else {
            return;
        };
        match mods::set_enabled(
            &server.path,
            server.resource_folder(),
            self.mods.mod_type,
            &entry.name,
            enabled,
        ) {
            Ok(new_path) => {
                // Update in place instead of rescanning the whole folder
                let entry = &mut self.mods.mods[index];
                entry.enabled = enabled;
                entry.full_path = new_path;
                let text = format!(
                    "{} {}",
                    entry.name,
                    if enabled { "enabled" } else { "disabled" }
                );
                self.set_status(text, false);
            }
            Err(e) => self.set_status(format!("Failed to change mod: {e}"), true),
        }
    }

    pub fn delete_mod(&mut self, path: &std::path::Path, name: &str) {
        match mods::delete(path) {
            Ok(()) => {
                self.mods.mods.retain(|m| m.full_path != path);
                self.set_status(format!("Deleted {name}"), false);
            }
            Err(e) => self.set_status(format!("Failed to delete {name}: {e}"), true),
        }
    }

    fn handle_task(&mut self, task: TaskResult) {
        match task {
            TaskResult::ModsScanned { generation, result } => {
                if generation != self.mods.generation {
                    return; // superseded by a newer scan
                }
                self.mods.loading = false;
                match result {
                    Ok(list) => self.mods.mods = list,
                    Err(e) => {
                        self.mods.mods.clear();
                        self.set_status(format!("Failed to scan mods: {e}"), true);
                    }
                }
            }
            TaskResult::ModDetails(result) => {
                // Ignore if the dialog was closed meanwhile
                if matches!(self.dialog, Some(Dialog::ModDetails(None))) {
                    match result {
                        Ok(details) => self.dialog = Some(Dialog::ModDetails(Some(details))),
                        Err(e) => {
                            self.dialog = None;
                            self.set_status(format!("Failed to read mod: {e}"), true);
                        }
                    }
                }
            }
            TaskResult::ModsAdded {
                server_id,
                added,
                errors,
            } => {
                self.mods.copying = self.mods.copying.saturating_sub(1);
                if errors.is_empty() {
                    self.set_status(format!("Added {added} client mod(s)"), false);
                } else {
                    self.set_status(
                        format!(
                            "Added {added}, failed {}: {}",
                            errors.len(),
                            errors.join("; ")
                        ),
                        true,
                    );
                }
                if self.mods.loaded_for == Some((server_id, ModType::Client)) {
                    self.reload_mods();
                }
            }
        }
    }

    // ----------------------------------------------------------- window/quit

    fn hide_window(&mut self) {
        self.window_hidden = true;
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn show_window(&mut self) {
        self.window_hidden = false;
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Quit, asking first if servers are running and that's enabled.
    pub fn request_quit(&mut self) {
        if self.running_count() > 0 && self.settings.confirm_quit {
            self.show_window();
            self.dialog = Some(Dialog::ConfirmQuit);
        } else {
            self.begin_quit();
        }
    }

    /// Stop every server, then close once they have exited.
    pub fn begin_quit(&mut self) {
        if self.running_count() == 0 {
            self.close_now();
            return;
        }
        let timeout = self.settings.stop_timeout();
        for session in self.sessions.values_mut() {
            session.begin_shutdown(timeout);
        }
        self.quit = QuitState::StoppingServers {
            deadline: Instant::now() + timeout,
        };
    }

    pub fn close_now(&mut self) {
        self.quit = QuitState::Closing;
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub fn stopping_for_quit(&self) -> bool {
        matches!(self.quit, QuitState::StoppingServers { .. })
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if matches!(self.quit, QuitState::Closing) {
            return; // let it close
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        if self.stopping_for_quit() {
            return; // already on our way out
        }
        if self.settings.close_to_tray && self.tray.is_some() {
            self.hide_window();
        } else {
            self.request_quit();
        }
    }

    fn tick_sessions(&mut self) -> Option<Instant> {
        let mut next: Option<Instant> = None;
        let mut messages = Vec::new();

        for (id, session) in &mut self.sessions {
            let Some(server) = self.servers.get(id) else {
                continue;
            };
            let auto_restart = server.auto_restart && matches!(self.quit, QuitState::Running);
            let (events, deadline) = session.tick(&server.path, auto_restart);
            if let Some(d) = deadline {
                next = Some(next.map_or(d, |n| n.min(d)));
            }
            for event in events {
                messages.push(match event {
                    Event::Exited { code, expected } => {
                        let crashed = !expected && code != Some(0);
                        let what = if crashed { "crashed" } else { "stopped" };
                        (format!("{} {what}", server.name), crashed)
                    }
                    Event::ForceKilled => (format!("{} was force-stopped", server.name), true),
                    Event::Restarted => (format!("{} restarted", server.name), false),
                    Event::RestartFailed(e) => {
                        (format!("Failed to restart {}: {e}", server.name), true)
                    }
                    Event::GaveUpRestarting => (
                        format!("{} keeps crashing; auto-restart paused", server.name),
                        true,
                    ),
                    Event::PlayersUpdated(n) => (format!("{n} player(s) online"), false),
                });
            }
        }
        for (text, is_error) in messages {
            self.set_status(text, is_error);
        }

        // Only the console on screen (or one awaiting a reply) needs to wake the UI
        let visible = !self.window_hidden;
        for (id, session) in &self.sessions {
            let watched = visible && self.selected.as_deref() == Some(id.as_str());
            session
                .console
                .set_notify(watched || session.player_list_pending());
        }
        next
    }

    fn update_tray_tooltip(&mut self) {
        let running = self.running_count();
        if let Some(tray) = &mut self.tray {
            let text = match running {
                0 => "BeamMP Panel".to_string(),
                1 => "BeamMP Panel - 1 server running".to_string(),
                n => format!("BeamMP Panel - {n} servers running"),
            };
            tray.set_tooltip(&text);
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(command) = self.commands_rx.try_recv() {
            match command {
                AppCommand::Show => self.show_window(),
                AppCommand::Quit => self.request_quit(),
            }
        }
        while let Ok(task) = self.tasks_rx.try_recv() {
            self.handle_task(task);
        }

        self.handle_close_request(ctx);
        let mut next = self.tick_sessions();

        if let QuitState::StoppingServers { deadline } = self.quit {
            if self.running_count() == 0 {
                self.close_now();
            } else {
                // Sessions force-kill at their own deadline; this is a backstop.
                let backstop = deadline + Duration::from_secs(2);
                if Instant::now() >= backstop {
                    self.close_now();
                }
                next = Some(next.map_or(backstop, |n| n.min(backstop)));
            }
        }

        self.update_tray_tooltip();

        // Event-driven: output, tray clicks and finished tasks request repaints
        // themselves; only timers need scheduling here.
        if let Some(next) = next {
            ctx.request_repaint_after(next.saturating_duration_since(Instant::now()));
        }
        // Uptime / countdown labels tick once a second while visible.
        if !self.window_hidden
            && self
                .selected
                .as_deref()
                .and_then(|id| self.sessions.get(id))
                .is_some_and(|s| s.is_running() || s.state() == State::Restarting)
        {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::show(self, ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Normally every server has already stopped; make sure, stopping them in parallel.
        let timeout = Duration::from_secs(3);
        for session in self.sessions.values_mut() {
            session.begin_shutdown(timeout);
        }
        let deadline = Instant::now() + timeout;
        for session in self.sessions.values_mut() {
            session.finish_shutdown(deadline);
        }
    }
}
