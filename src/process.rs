use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct ServerProcess {
    child: Child,
    output_receiver: Receiver<String>,
    stdin: Arc<Mutex<ChildStdin>>,
    _output_thread: thread::JoinHandle<()>,
}

impl ServerProcess {
    pub fn start(server_path: &Path) -> Result<Self> {
        // Look for BeamMP-Server.exe (Windows) or BeamMP-Server (Linux/Mac)
        let exe_name = if cfg!(windows) {
            "BeamMP-Server.exe"
        } else {
            "BeamMP-Server"
        };

        let exe_path = server_path.join(exe_name);
        if !exe_path.exists() {
            return Err(anyhow!("BeamMP server executable not found: {}", exe_path.display()));
        }

        let mut command = Command::new(&exe_path);
        command
            .current_dir(server_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Prevent console window from appearing on Windows
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;
        let stdin = Arc::new(Mutex::new(
            child.stdin.take().ok_or_else(|| anyhow!("Failed to capture stdin"))?
        ));

        // Use bounded channel to prevent unbounded memory growth
        let (tx, rx) = sync_channel(1000);

        // Spawn thread to read stdout
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx_clone.send(line);
            }
        });

        // Spawn thread to read stderr
        let tx_clone = tx.clone();
        let output_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx_clone.send(format!("[ERROR] {}", line));
            }
        });

        Ok(Self {
            child,
            output_receiver: rx,
            stdin,
            _output_thread: output_thread,
        })
    }

    pub fn send_command(&self, command: &str) -> Result<()> {
        let mut stdin = self.stdin.lock().map_err(|e| anyhow!("Failed to lock stdin: {}", e))?;
        writeln!(stdin, "{}", command)?;
        stdin.flush()?;
        Ok(())
    }

    /// Immediately kill the process. Graceful shutdown is requested by the
    /// caller via `send_command("exit")`; this is the timeout fallback.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    pub fn read_output(&self) -> Vec<String> {
        let mut lines = Vec::new();
        while let Ok(line) = self.output_receiver.try_recv() {
            lines.push(line);
        }
        lines
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        // Don't leave an orphaned (hidden) server process behind, e.g. when
        // the panel window is closed while a server is still running.
        if self.is_running() {
            let _ = self.send_command("exit");
            for _ in 0..10 {
                if !self.is_running() {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

