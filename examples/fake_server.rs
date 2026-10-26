//! A stand-in for BeamMP-Server used by the panel's tests (and handy for
//! trying the UI without a real server). Copy the built binary into a folder
//! next to a ServerConfig.toml as `BeamMP-Server(.exe)`.
//!
//! Console commands: `list`, `say <msg>`, `exit`, `crash`, `hang`, `spam`.

use std::io::{BufRead, Write};
use std::time::Duration;

fn main() {
    {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "[25/10/26 21:00:00] [INFO] BeamMP-Server v3 (fake)");
        let _ = writeln!(
            out,
            "\x1b[33m[25/10/26 21:00:00] [WARN] colored warning\x1b[0m"
        );
        // Invalid UTF-8 must not stop the panel's reader
        let _ = out.write_all(b"[25/10/26 21:00:00] [INFO] invalid utf-8: \xff\xfe end\n");
        let _ = writeln!(out, "[25/10/26 21:00:01] [INFO] ready");
        let _ = out.flush();
    }
    eprintln!("a line on stderr");

    std::thread::spawn(|| {
        let mut i = 0u64;
        loop {
            std::thread::sleep(Duration::from_secs(3));
            i += 1;
            println!("[25/10/26 21:00:00] [DEBUG] heartbeat {i}");
        }
    });

    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        match line.trim() {
            "list" => {
                println!("Name                     ID    Cars  ");
                println!("SomePlayer               0     1     ");
                println!("John Doe Racing          12    3     ");
            }
            "exit" => {
                println!("[25/10/26 21:01:00] [INFO] Shutting down...");
                std::thread::sleep(Duration::from_millis(300));
                std::process::exit(0);
            }
            "crash" => {
                eprintln!("fatal error (fake)");
                std::process::exit(3);
            }
            "hang" => loop {
                std::thread::sleep(Duration::from_secs(60));
            },
            "spam" => {
                for i in 0..20_000 {
                    println!("[25/10/26 21:00:00] [INFO] spam {i}");
                }
            }
            other => match other.strip_prefix("say ") {
                Some(msg) => println!("[25/10/26 21:00:00] [CHAT] Server: {msg}"),
                None => println!("[25/10/26 21:00:00] [INFO] Unknown command: {other}"),
            },
        }
    }
}
