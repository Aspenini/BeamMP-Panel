use crate::console::{Console, LineKind};
use crate::process::ServerProcess;
use anyhow::Result;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long to wait for the rest of a `list` reply before finalizing it.
const PLAYER_LIST_TIMEOUT: Duration = Duration::from_secs(2);
const RESTART_DELAY: Duration = Duration::from_secs(5);
/// Give up auto-restarting after this many crashes within `CRASH_WINDOW`.
const MAX_CRASHES: usize = 3;
const CRASH_WINDOW: Duration = Duration::from_secs(300);
const MAX_HISTORY: usize = 100;

/// A connected player parsed from the server's `list` command output.
#[derive(Clone, Debug)]
pub struct PlayerEntry {
    pub name: String,
    pub id: String,
    pub cars: String,
}

/// Tracks an in-flight `list` command whose output we are watching for.
struct PlayerListRequest {
    started: Instant,
    /// True once the "Name  ID  Cars" table header has been seen.
    capturing: bool,
    /// Console position the reply starts after.
    seq: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Stopped,
    Running,
    Stopping,
    /// Waiting to start again after a crash or a restart request.
    Restarting,
}

/// Notable things that happened during `tick`, surfaced in the status bar.
pub enum Event {
    Exited { code: Option<i32>, expected: bool },
    ForceKilled,
    Restarted,
    RestartFailed(String),
    GaveUpRestarting,
    PlayersUpdated(usize),
}

/// Everything the panel tracks about one server it has started: the process,
/// its console (kept after exit so the log stays readable), players and the
/// console's command line.
pub struct ServerSession {
    pub console: Arc<Console>,
    process: Option<ServerProcess>,
    started_at: Option<Instant>,
    stop_deadline: Option<Instant>,
    /// Stop was requested by the user, so the exit isn't a crash.
    stop_requested: bool,
    /// Start again as soon as the process exits (Restart button).
    restart_after_stop: bool,
    restart_at: Option<Instant>,
    crash_times: VecDeque<Instant>,
    player_request: Option<PlayerListRequest>,
    pub players: Vec<PlayerEntry>,

    // Console command line
    pub command_input: String,
    history: Vec<String>,
    history_pos: Option<usize>,
    pub auto_scroll: bool,
}

impl ServerSession {
    pub fn new(console: Console) -> Self {
        Self {
            console: Arc::new(console),
            process: None,
            started_at: None,
            stop_deadline: None,
            stop_requested: false,
            restart_after_stop: false,
            restart_at: None,
            crash_times: VecDeque::new(),
            player_request: None,
            players: Vec::new(),
            command_input: String::new(),
            history: Vec::new(),
            history_pos: None,
            auto_scroll: true,
        }
    }

    pub fn state(&self) -> State {
        if self.process.is_some() {
            if self.stop_deadline.is_some() {
                State::Stopping
            } else {
                State::Running
            }
        } else if self.restart_at.is_some() {
            State::Restarting
        } else {
            State::Stopped
        }
    }

    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.process.as_ref().and(self.started_at.map(|t| t.elapsed()))
    }

    pub fn restart_in(&self) -> Option<Duration> {
        self.restart_at
            .map(|t| t.saturating_duration_since(Instant::now()))
    }

    pub fn start(&mut self, server_path: &Path) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }
        self.console
            .system(format!("Starting server in {}", server_path.display()));
        let process = ServerProcess::start(server_path, Arc::clone(&self.console))
            .inspect_err(|e| self.console.system(format!("Failed to start: {e}")))?;
        self.process = Some(process);
        self.started_at = Some(Instant::now());
        self.stop_deadline = None;
        self.stop_requested = false;
        self.restart_after_stop = false;
        self.restart_at = None;
        self.players.clear();
        self.player_request = None;
        Ok(())
    }

    /// Ask the server to shut down; `tick` force-kills it after `timeout`.
    pub fn stop(&mut self, timeout: Duration) {
        // A pending auto-restart is cancelled by an explicit stop.
        self.restart_at = None;
        let Some(process) = &mut self.process else {
            return;
        };
        if self.stop_deadline.is_some() {
            return;
        }
        self.stop_requested = true;
        self.stop_deadline = Some(Instant::now() + timeout);
        self.console.system("Stopping server...");
        if process.send_command("exit").is_err() {
            // Can't ask nicely; don't make the user wait for the timeout.
            self.stop_deadline = Some(Instant::now());
        }
    }

    pub fn restart(&mut self, timeout: Duration) {
        if self.process.is_some() {
            self.stop(timeout);
            self.restart_after_stop = true;
        }
    }

    pub fn force_stop(&mut self) {
        if self.process.is_some() {
            self.stop_requested = true;
            self.stop_deadline = Some(Instant::now());
        }
    }

    pub fn send_command(&mut self, command: &str) -> Result<()> {
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Server is not running"))?;
        process.send_command(command)?;
        self.console.push(format!("> {command}"), LineKind::Input);
        Ok(())
    }

    /// Send the console command line and add it to the history.
    pub fn submit_command_input(&mut self) -> Result<()> {
        let command = self.command_input.trim().to_string();
        self.history_pos = None;
        if command.is_empty() {
            return Ok(());
        }
        if command == "list" {
            // Typed by hand: still pick the reply up for the players card
            self.request_player_list()?;
        } else {
            self.send_command(&command)?;
        }
        if self.history.last() != Some(&command) {
            self.history.push(command);
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
        self.command_input.clear();
        Ok(())
    }

    /// Step through command history: `older` = Up arrow.
    pub fn browse_history(&mut self, older: bool) {
        if self.history.is_empty() {
            return;
        }
        let last = self.history.len() - 1;
        self.history_pos = match (self.history_pos, older) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i >= last => None,
            (Some(i), false) => Some(i + 1),
        };
        self.command_input = self
            .history_pos
            .map(|i| self.history[i].clone())
            .unwrap_or_default();
    }

    pub fn request_player_list(&mut self) -> Result<()> {
        let seq = self.console.end_seq();
        self.send_command("list")?;
        self.player_request = Some(PlayerListRequest {
            started: Instant::now(),
            capturing: false,
            seq,
        });
        Ok(())
    }

    pub fn player_list_pending(&self) -> bool {
        self.player_request.is_some()
    }

    /// Advance timers and react to process output/exit. Returns the events
    /// that happened and the next time `tick` needs to run, if any.
    pub fn tick(&mut self, server_path: &Path, auto_restart: bool) -> (Vec<Event>, Option<Instant>) {
        let now = Instant::now();
        let mut events = Vec::new();

        if let Some(process) = &mut self.process {
            if let Some(status) = process.try_wait() {
                let expected = self.stop_requested;
                let code = status.code();
                self.console.system(match code {
                    Some(code) => format!("Server exited with code {code}."),
                    None => "Server exited.".to_string(),
                });
                self.on_exit(now, expected, auto_restart, &mut events);
                events.push(Event::Exited { code, expected });
            } else if self.stop_deadline.is_some_and(|d| now >= d) {
                process.kill();
                self.console
                    .system("Server did not exit in time; force-killed.");
                self.on_exit(now, true, auto_restart, &mut events);
                events.push(Event::ForceKilled);
            } else {
                self.process_player_reply(&mut events);
            }
        }

        if self.restart_at.is_some_and(|t| now >= t) {
            self.restart_at = None;
            match self.start(server_path) {
                Ok(()) => events.push(Event::Restarted),
                Err(e) => events.push(Event::RestartFailed(e.to_string())),
            }
        }

        if let Some(request) = &self.player_request
            && request.started.elapsed() >= PLAYER_LIST_TIMEOUT
        {
            // The player table has no terminator line, so if the console
            // goes quiet, finalize (or abandon) the request.
            if request.capturing {
                events.push(Event::PlayersUpdated(self.players.len()));
            }
            self.player_request = None;
        }

        (events, self.next_deadline())
    }

    fn on_exit(&mut self, now: Instant, expected: bool, auto_restart: bool, events: &mut Vec<Event>) {
        self.process = None;
        self.started_at = None;
        self.stop_deadline = None;
        self.player_request = None;
        self.players.clear();

        if self.restart_after_stop {
            self.restart_after_stop = false;
            self.restart_at = Some(now);
        } else if !expected && auto_restart {
            self.crash_times.push_back(now);
            while self
                .crash_times
                .front()
                .is_some_and(|t| now.duration_since(*t) > CRASH_WINDOW)
            {
                self.crash_times.pop_front();
            }
            if self.crash_times.len() > MAX_CRASHES {
                self.console.system(format!(
                    "Server crashed {} times in {} minutes; not restarting again.",
                    self.crash_times.len(),
                    CRASH_WINDOW.as_secs() / 60
                ));
                self.crash_times.clear();
                events.push(Event::GaveUpRestarting);
            } else {
                self.console.system(format!(
                    "Restarting in {} seconds...",
                    RESTART_DELAY.as_secs()
                ));
                self.restart_at = Some(now + RESTART_DELAY);
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let mut deadline: Option<Instant> = None;
        let mut consider = |t: Instant| {
            deadline = Some(deadline.map_or(t, |d| d.min(t)));
        };
        if let Some(t) = self.stop_deadline {
            consider(t);
        }
        if let Some(t) = self.restart_at {
            consider(t);
        }
        if let Some(r) = &self.player_request {
            consider(r.started + PLAYER_LIST_TIMEOUT);
        }
        if let Some(p) = &self.process
            && p.output_closed()
        {
            // Pipes closed but the process hasn't been reaped yet.
            consider(Instant::now() + Duration::from_millis(250));
        }
        deadline
    }

    /// Scan console output for the response to a pending `list` command.
    ///
    /// BeamMP-Server replies either with "No players online." or a
    /// fixed-width table: a "Name  ID  Cars" header followed by one row per
    /// player. Rows are parsed from the right (id and cars are numeric) so
    /// player names containing spaces survive.
    fn process_player_reply(&mut self, events: &mut Vec<Event>) {
        let Some(request) = &mut self.player_request else {
            return;
        };
        let (lines, end) = self.console.lines_since(request.seq);
        request.seq = end;

        for line in &lines {
            let trimmed = line.trim();
            if !request.capturing {
                if trimmed == "No players online." {
                    self.players.clear();
                    self.player_request = None;
                    events.push(Event::PlayersUpdated(0));
                    return;
                } else if trimmed.split_whitespace().eq(["Name", "ID", "Cars"]) {
                    self.players.clear();
                    request.capturing = true;
                }
            } else if let Some(entry) = parse_player_row(trimmed) {
                self.players.push(entry);
            } else {
                // First non-row line marks the end of the table
                self.player_request = None;
                events.push(Event::PlayersUpdated(self.players.len()));
                return;
            }
        }
    }

    /// Ask the server to exit without waiting (used when the panel quits).
    pub fn begin_shutdown(&mut self, timeout: Duration) {
        self.restart_after_stop = false;
        self.stop(timeout);
    }

    /// Block until the process exits or `deadline` passes, then kill it.
    pub fn finish_shutdown(&mut self, deadline: Instant) {
        if let Some(process) = &mut self.process {
            while Instant::now() < deadline && process.try_wait().is_none() {
                std::thread::sleep(Duration::from_millis(50));
            }
            process.kill();
        }
        self.process = None;
    }
}

/// Parse one row of the `list` table: `<name (may contain spaces)> <id> <cars>`.
fn parse_player_row(line: &str) -> Option<PlayerEntry> {
    let mut tokens = line.split_whitespace().rev();
    let cars = tokens.next()?;
    let id = tokens.next()?;
    if cars.parse::<u32>().is_err() || id.parse::<u32>().is_err() {
        return None;
    }
    let name_end = line.trim_end().len() - cars.len();
    let name_end = line[..name_end].trim_end().len() - id.len();
    let name = line[..name_end].trim();
    if name.is_empty() {
        return None;
    }
    Some(PlayerEntry {
        name: name.to_string(),
        id: id.to_string(),
        cars: cars.to_string(),
    })
}

/// End-to-end tests against `examples/fake_server.rs`, which `cargo test` builds.
#[cfg(test)]
mod process_tests {
    use super::*;
    use std::path::PathBuf;

    struct FakeServer(PathBuf);

    impl FakeServer {
        fn new() -> Self {
            let exe = std::env::current_exe().unwrap();
            let examples = exe.parent().unwrap().parent().unwrap().join("examples");
            let fake = examples.join(format!("fake_server{}", std::env::consts::EXE_SUFFIX));
            assert!(fake.is_file(), "{} missing; run via `cargo test`", fake.display());

            let dir = std::env::temp_dir().join(format!("beammp-panel-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(&fake, dir.join(crate::process::SERVER_EXE)).unwrap();
            Self(dir)
        }
    }

    impl Drop for FakeServer {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn session(max_lines: usize) -> ServerSession {
        ServerSession::new(Console::new(egui::Context::default(), max_lines))
    }

    /// Tick until an event matches, failing after a few seconds.
    fn tick_until(
        session: &mut ServerSession,
        server: &FakeServer,
        auto_restart: bool,
        mut pred: impl FnMut(&Event) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let (events, _) = session.tick(&server.0, auto_restart);
            if events.iter().any(&mut pred) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out; console:\n{}", session.console.to_text());
    }

    fn wait_for_line(session: &ServerSession, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !session.console.to_text().contains(needle) {
            assert!(Instant::now() < deadline, "no {needle:?} in:\n{}", session.console.to_text());
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn lifecycle_console_and_players() {
        let server = FakeServer::new();
        let mut s = session(1000);
        s.start(&server.0).unwrap();
        assert!(s.state() == State::Running);
        wait_for_line(&s, "ready");
        wait_for_line(&s, "a line on stderr");

        let text = s.console.to_text();
        assert!(text.contains("invalid utf-8: \u{fffd}\u{fffd} end"), "{text}");
        assert!(text.contains("[WARN] colored warning\n"), "ANSI not stripped: {text}");
        {
            let lines = s.console.lock();
            let stderr = lines.lines.iter().find(|l| l.text == "a line on stderr").unwrap();
            assert_eq!(stderr.kind, LineKind::Error);
        }

        s.request_player_list().unwrap();
        tick_until(&mut s, &server, false, |e| matches!(e, Event::PlayersUpdated(2)));
        assert_eq!(s.players[1].name, "John Doe Racing");

        s.command_input = "say hello".into();
        s.submit_command_input().unwrap();
        wait_for_line(&s, "[CHAT] Server: hello");
        s.browse_history(true);
        assert_eq!(s.command_input, "say hello");

        s.stop(Duration::from_secs(5));
        assert!(s.state() == State::Stopping);
        tick_until(&mut s, &server, true, |e| {
            matches!(e, Event::Exited { code: Some(0), expected: true })
        });
        // A requested stop never auto-restarts
        assert!(s.state() == State::Stopped);
    }

    #[test]
    fn crash_schedules_restart() {
        let server = FakeServer::new();
        let mut s = session(1000);
        s.start(&server.0).unwrap();
        wait_for_line(&s, "ready");
        s.send_command("crash").unwrap();
        tick_until(&mut s, &server, true, |e| {
            matches!(e, Event::Exited { code: Some(3), expected: false })
        });
        assert!(s.state() == State::Restarting);
        assert!(s.restart_in().unwrap() > Duration::ZERO);
        // Stop cancels the pending restart
        s.stop(Duration::from_secs(1));
        assert!(s.state() == State::Stopped);
    }

    #[test]
    fn restart_button_starts_again() {
        let server = FakeServer::new();
        let mut s = session(1000);
        s.start(&server.0).unwrap();
        wait_for_line(&s, "ready");
        s.restart(Duration::from_secs(5));
        tick_until(&mut s, &server, false, |e| matches!(e, Event::Restarted));
        assert!(s.state() == State::Running);
        s.force_stop();
        tick_until(&mut s, &server, false, |e| matches!(e, Event::ForceKilled));
    }

    #[test]
    fn hung_server_is_force_killed() {
        let server = FakeServer::new();
        let mut s = session(1000);
        s.start(&server.0).unwrap();
        wait_for_line(&s, "ready");
        s.send_command("hang").unwrap();
        s.stop(Duration::from_millis(300));
        tick_until(&mut s, &server, true, |e| matches!(e, Event::ForceKilled));
        assert!(s.state() == State::Stopped);
    }

    #[test]
    fn output_flood_is_bounded_and_never_blocks() {
        let server = FakeServer::new();
        let mut s = session(500);
        s.start(&server.0).unwrap();
        wait_for_line(&s, "ready");
        s.send_command("spam").unwrap();
        wait_for_line(&s, "spam 19999");
        assert_eq!(s.console.lock().lines.len(), 500);
        // Still responsive after the flood
        s.stop(Duration::from_secs(5));
        tick_until(&mut s, &server, false, |e| matches!(e, Event::Exited { .. }));
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
    fn keeps_inner_spacing_in_names() {
        let entry = parse_player_row("A  B   7 0").unwrap();
        assert_eq!(entry.name, "A  B");
    }

    #[test]
    fn rejects_non_row_lines() {
        assert!(parse_player_row("No players online.").is_none());
        assert!(parse_player_row("[16:20:01] [INFO] something happened").is_none());
        assert!(parse_player_row("").is_none());
        assert!(parse_player_row("12 3").is_none());
        // Header has no trailing numbers
        assert!(parse_player_row("Name                     ID    Cars").is_none());
    }
}
