use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct ServerProcess {
    child: Child,
    output_receiver: Receiver<String>,
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

        let mut child = Command::new(&exe_path)
            .current_dir(server_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let (tx, rx) = channel();

        // Spawn thread to read stdout
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = tx_clone.send(line);
                }
            }
        });

        // Spawn thread to read stderr
        let tx_clone = tx.clone();
        let output_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = tx_clone.send(format!("[ERROR] {}", line));
                }
            }
        });

        Ok(Self {
            child,
            output_receiver: rx,
            _output_thread: output_thread,
        })
    }

    pub fn stop(&mut self) -> Result<()> {
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

