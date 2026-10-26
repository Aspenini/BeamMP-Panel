//! Single-instance handoff. With close-to-tray the panel keeps running after
//! its window disappears, so launching it again would otherwise start a
//! second panel managing the same servers. The first instance listens on a
//! loopback port recorded in `instance.lock`; later launches ask it to show
//! its window and exit.

use crate::app::AppCommand;
use crate::util;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::time::Duration;

const SHOW_REQUEST: &str = "BeamMP-Panel show";
const SHOW_REPLY: &str = "BeamMP-Panel ok";

pub enum Claim {
    /// This is the only instance; hand the listener to `listen`.
    Primary(Option<TcpListener>),
    /// Another instance was found and asked to show itself.
    AlreadyRunning,
}

pub fn claim() -> Claim {
    let Ok(lock_path) = util::config_dir().map(|d| d.join("instance.lock")) else {
        return Claim::Primary(None);
    };

    if let Some(port) = std::fs::read_to_string(&lock_path)
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        && ask_existing_to_show(port)
    {
        return Claim::AlreadyRunning;
    }

    // Stale or missing lock: become the primary instance.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok();
    if let Some(port) = listener
        .as_ref()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
    {
        let _ = util::atomic_write(&lock_path, port.to_string().as_bytes());
    }
    Claim::Primary(listener)
}

fn ask_existing_to_show(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    if writeln!(stream, "{SHOW_REQUEST}").is_err() {
        return false;
    }
    // Only trust the port if the panel answers; something else may have it now.
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).is_ok() && reply.trim() == SHOW_REPLY
}

/// Serve show requests from later launches on a background thread.
pub fn listen(listener: TcpListener, ctx: egui::Context, commands: Sender<AppCommand>) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut line).is_ok() && line.trim() == SHOW_REQUEST {
                let _ = writeln!(&stream, "{SHOW_REPLY}");
                if commands.send(AppCommand::Show).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        }
    });
}
