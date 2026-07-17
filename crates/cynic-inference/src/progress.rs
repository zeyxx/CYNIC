//! Operator-facing progress on stderr (JSON CLIs stay silent on stdout until done).
//!
//! Enabled by default. Disable with `CYNIC_PROGRESS=0` for pure machine pipes.

use std::io::{self, Write};
use std::time::Instant;

/// Whether progress lines should be emitted.
pub fn progress_enabled() -> bool {
    match std::env::var("CYNIC_PROGRESS") {
        Ok(v) => {
            let v = v.to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "off" || v == "no")
        }
        Err(_) => true,
    }
}

/// Emit a single progress line to stderr and flush (best-effort).
#[allow(clippy::print_stderr)]
pub fn progress(stage: &str, detail: impl AsRef<str>) {
    if !progress_enabled() {
        return;
    }
    let detail = detail.as_ref();
    let line = if detail.is_empty() {
        format!("cynic: {stage}\n")
    } else {
        format!("cynic: {stage}: {detail}\n")
    };
    let mut err = io::stderr().lock();
    let _ = err.write_all(line.as_bytes());
    let _ = err.flush();
}

/// Periodic wait reporter (e.g. model HTTP readiness).
pub struct WaitProgress {
    label: String,
    started: Instant,
    last_emit: Instant,
    interval_secs: u64,
}

impl WaitProgress {
    pub fn new(label: impl Into<String>, interval_secs: u64) -> Self {
        let now = Instant::now();
        Self {
            label: label.into(),
            started: now,
            last_emit: now
                .checked_sub(std::time::Duration::from_secs(interval_secs))
                .unwrap_or(now),
            interval_secs,
        }
    }

    pub fn tick(&mut self) {
        if !progress_enabled() {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last_emit).as_secs() < self.interval_secs {
            return;
        }
        self.last_emit = now;
        let elapsed = now.duration_since(self.started).as_secs();
        progress(
            "waiting",
            format!("{} ({}s elapsed)", self.label, elapsed),
        );
    }

    pub fn done(&self, ok: bool) {
        let elapsed = self.started.elapsed().as_secs();
        if ok {
            progress("ready", format!("{} after {}s", self.label, elapsed));
        } else {
            progress(
                "timeout",
                format!("{} not ready after {}s", self.label, elapsed),
            );
        }
    }
}
