use crate::console::Console;
use anyhow::{Result, anyhow};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const SERVER_EXE: &str = if cfg!(windows) {
    "BeamMP-Server.exe"
} else {
    "BeamMP-Server"
};

pub struct ServerProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    /// Number of output streams (stdout/stderr) still open.
    open_streams: Arc<AtomicUsize>,
}

impl ServerProcess {
    pub fn start(server_path: &Path, console: Arc<Console>) -> Result<Self> {
        let exe_path = server_path.join(SERVER_EXE);
        if !exe_path.is_file() {
            return Err(anyhow!(
                "BeamMP server executable not found: {}",
                exe_path.display()
            ));
        }

        let mut command = Command::new(&exe_path);
        command
            .current_dir(server_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Prevent a console window from appearing on Windows
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn()?;

        #[cfg(windows)]
        job::assign(&child);

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to capture stdin"))?;

        let open_streams = Arc::new(AtomicUsize::new(2));
        spawn_reader(stdout, false, Arc::clone(&console), Arc::clone(&open_streams));
        spawn_reader(stderr, true, console, Arc::clone(&open_streams));

        Ok(Self {
            child,
            stdin: Some(stdin),
            open_streams,
        })
    }

    pub fn send_command(&mut self, command: &str) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Server console input is closed"))?;
        let result = writeln!(stdin, "{}", command).and_then(|_| stdin.flush());
        if result.is_err() {
            // Broken pipe: the server is going away; stop trying to write.
            self.stdin = None;
        }
        Ok(result?)
    }

    /// Immediately kill the process. Graceful shutdown is requested by the
    /// caller via `send_command("exit")`; this is the timeout fallback.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// The exit status once the process has exited, `None` while it runs.
    pub fn try_wait(&mut self) -> Option<ExitStatus> {
        match self.child.try_wait() {
            Ok(status) => status,
            // Can't query it any more: treat it as gone.
            Err(_) => Some(ExitStatus::default()),
        }
    }

    /// True once both output pipes have closed, which normally means the
    /// process has exited (or is about to).
    pub fn output_closed(&self) -> bool {
        self.open_streams.load(Ordering::Acquire) == 0
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        // Don't leave an orphaned (hidden) server process behind. Normally the
        // app has already stopped every server before getting here.
        if self.try_wait().is_none() {
            let _ = self.send_command("exit");
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline && self.try_wait().is_none() {
                thread::sleep(Duration::from_millis(50));
            }
            self.kill();
        }
    }
}

fn spawn_reader(
    stream: impl Read + Send + 'static,
    from_stderr: bool,
    console: Arc<Console>,
    open_streams: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::with_capacity(256);
        loop {
            buf.clear();
            // read_until + lossy decoding instead of lines(): one invalid
            // UTF-8 byte (e.g. a player name) must not stop the reader, or
            // the pipe fills and the server blocks on its next log write.
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => console.push_output(&String::from_utf8_lossy(&buf), from_stderr),
            }
        }
        open_streams.fetch_sub(1, Ordering::AcqRel);
        console.wake();
    });
}

/// Every server is put in a job object that kills its members when the
/// panel's handle closes, so hidden servers can't outlive a crashed panel.
#[cfg(windows)]
mod job {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    struct Job(HANDLE);
    // The handle is only used for AssignProcessToJobObject, which is thread-safe.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    fn job() -> Option<&'static Job> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            // Deliberately never closed: the OS closes it when the panel exits.
            (ok != 0).then_some(Job(handle))
        })
        .as_ref()
    }

    pub fn assign(child: &Child) {
        if let Some(job) = job() {
            unsafe {
                AssignProcessToJobObject(job.0, child.as_raw_handle() as HANDLE);
            }
        }
    }
}
