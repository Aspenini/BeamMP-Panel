use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// Coalesce bursts of output into at most ~20 repaints per second.
const REPAINT_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    Normal,
    Warn,
    Error,
    Debug,
    Chat,
    /// A command typed into the panel
    Input,
    /// A message from the panel itself (started, exited, ...)
    System,
}

impl LineKind {
    /// Classify a line of BeamMP server output by its `[LEVEL]` tag.
    fn classify(text: &str) -> Self {
        // The level tag follows the timestamp, so only look near the start.
        let head = &text[..text.floor_char_boundary(48)];
        if head.contains("[ERROR]") || head.contains("[FATAL]") {
            LineKind::Error
        } else if head.contains("[WARN]") {
            LineKind::Warn
        } else if head.contains("[DEBUG]") {
            LineKind::Debug
        } else if head.contains("[CHAT]") {
            LineKind::Chat
        } else {
            LineKind::Normal
        }
    }
}

pub struct ConsoleLine {
    pub text: String,
    pub kind: LineKind,
}

/// A bounded, thread-safe console buffer shared between a server's output
/// reader threads and the UI. Readers never block on the UI: once full, the
/// oldest lines are dropped.
pub struct Console {
    inner: Mutex<Inner>,
    ctx: egui::Context,
    /// Whether new output should wake the UI (its console is on screen, or
    /// the app is waiting for command output).
    notify: AtomicBool,
}

pub struct Inner {
    pub lines: VecDeque<ConsoleLine>,
    /// Sequence number of `lines[0]`; increases monotonically as lines are dropped.
    first_seq: u64,
    max_lines: usize,
}

impl Inner {
    pub fn end_seq(&self) -> u64 {
        self.first_seq + self.lines.len() as u64
    }
}

impl Console {
    pub fn new(ctx: egui::Context, max_lines: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                lines: VecDeque::with_capacity(max_lines.min(1024)),
                first_seq: 0,
                max_lines: max_lines.max(1),
            }),
            ctx,
            notify: AtomicBool::new(true),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock can't leave the buffer inconsistent
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_notify(&self, notify: bool) {
        self.notify.store(notify, Ordering::Relaxed);
    }

    /// Wake the UI regardless of `notify`, e.g. when the process exits.
    pub fn wake(&self) {
        self.ctx.request_repaint();
    }

    pub fn push(&self, text: String, kind: LineKind) {
        {
            let mut inner = self.lock();
            if inner.lines.len() >= inner.max_lines {
                inner.lines.pop_front();
                inner.first_seq += 1;
            }
            inner.lines.push_back(ConsoleLine { text, kind });
        }
        if self.notify.load(Ordering::Relaxed) {
            self.ctx.request_repaint_after(REPAINT_DELAY);
        }
    }

    /// Push a raw line of process output, stripping terminal escapes.
    pub fn push_output(&self, raw: &str, from_stderr: bool) {
        let text = crate::util::strip_ansi(raw.trim_end_matches(['\r', '\n']));
        let kind = if from_stderr {
            LineKind::Error
        } else {
            LineKind::classify(&text)
        };
        self.push(text.into_owned(), kind);
    }

    pub fn system(&self, text: impl Into<String>) {
        self.push(text.into(), LineKind::System);
    }

    pub fn end_seq(&self) -> u64 {
        self.lock().end_seq()
    }

    /// Copy the text of every line from `seq` onward (lines already dropped
    /// from the buffer are skipped). Returns the lines and the new end sequence.
    pub fn lines_since(&self, seq: u64) -> (Vec<String>, u64) {
        let inner = self.lock();
        let skip = seq.saturating_sub(inner.first_seq) as usize;
        let lines = inner
            .lines
            .iter()
            .skip(skip)
            .map(|l| l.text.clone())
            .collect();
        (lines, inner.end_seq())
    }

    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.first_seq = inner.end_seq();
        inner.lines.clear();
    }

    pub fn set_max_lines(&self, max_lines: usize) {
        let mut inner = self.lock();
        inner.max_lines = max_lines.max(1);
        while inner.lines.len() > inner.max_lines {
            inner.lines.pop_front();
            inner.first_seq += 1;
        }
    }

    pub fn to_text(&self) -> String {
        let inner = self.lock();
        let mut out = String::with_capacity(inner.lines.iter().map(|l| l.text.len() + 1).sum());
        for line in &inner.lines {
            out.push_str(&line.text);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_levels() {
        assert_eq!(LineKind::classify("[25/10/26 21:29:01] [ERROR] boom"), LineKind::Error);
        assert_eq!(LineKind::classify("[25/10/26 21:29:01] [WARN] hm"), LineKind::Warn);
        assert_eq!(LineKind::classify("[25/10/26 21:29:01] [CHAT] (1) bob: hi"), LineKind::Chat);
        assert_eq!(LineKind::classify("[25/10/26 21:29:01] [INFO] ok"), LineKind::Normal);
        // Tags far into the message body are not level tags
        let long = format!("[25/10/26 21:29:01] [INFO] {} [ERROR]", "x".repeat(40));
        assert_eq!(LineKind::classify(&long), LineKind::Normal);
    }

    #[test]
    fn ring_buffer_tracks_sequence() {
        let console = Console::new(egui::Context::default(), 3);
        for i in 0..5 {
            console.push(i.to_string(), LineKind::Normal);
        }
        let (lines, end) = console.lines_since(0);
        assert_eq!(lines, ["2", "3", "4"]);
        assert_eq!(end, 5);
        let (lines, _) = console.lines_since(4);
        assert_eq!(lines, ["4"]);
        console.clear();
        assert_eq!(console.end_seq(), 5);
        assert!(console.lines_since(0).0.is_empty());
    }
}
