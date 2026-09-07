//! Read-only tmux session supervision for CYNIC.
//!
//! Observes tmux sessions running on the host (Pi coding agent, build shells,
//! operator sessions) without runtime mutation authority. Captures pane content,
//! process metrics, and activity signals to produce machine-readable evidence.
//!
//! This module extends the existing `transport` tmux observation with deeper
//! per-session telemetry while preserving the read-only boundary.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::transport::TmuxSessionObservation;

/// Schema version for session supervision evidence.
pub const TMUX_SESSION_SUPERVISION_SCHEMA: &str = "cynic.tmux_session_supervision.v1";

/// Process-level metrics captured for a supervised session.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SessionProcessMetrics {
    /// PID of the process group leader in the pane.
    pub pid: u32,
    /// Process name (comm).
    pub comm: String,
    /// Resident memory in bytes.
    pub rss_bytes: u64,
    /// Virtual memory in bytes.
    pub vms_bytes: u64,
    /// CPU usage fraction (0-100).
    pub cpu_percent: f32,
    /// Process state (R/S/D/Z/T).
    pub state: String,
    /// Number of threads.
    pub threads: u64,
    /// Process uptime in seconds (since process start).
    pub uptime_secs: u64,
}

/// Activity signal derived from pane content sampling.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SessionActivity {
    /// Total lines in the pane scrollback buffer.
    pub total_lines: u32,
    /// Lines in the visible viewport.
    pub visible_lines: u32,
    /// Last non-empty line (truncated to 200 chars).
    pub last_line: String,
    /// Whether the pane shows a shell prompt (heuristic).
    pub appears_at_prompt: bool,
    /// Whether the pane shows a running process (heuristic: no prompt, cursor at end).
    pub appears_running: bool,
}

/// Full observation of a single tmux session.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SupervisedTmuxSession {
    /// Session name.
    pub name: String,
    /// Number of windows.
    pub windows: u32,
    /// Number of attached clients.
    pub attached_clients: u32,
    /// Per-pane observations (keyed by pane index).
    pub panes: Vec<SupervisedTmuxPane>,
    /// Oldest process uptime in the session (seconds).
    pub session_uptime_secs: u64,
    /// Aggregated CPU across all panes.
    pub total_cpu_percent: f32,
    /// Aggregated RSS across all panes (bytes).
    pub total_rss_bytes: u64,
}

/// Observation of a single pane within a session.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SupervisedTmuxPane {
    /// Pane index.
    pub index: u32,
    /// Pane PID (process group leader).
    pub pid: u32,
    /// Current command running in the pane.
    pub current_command: String,
    /// Process metrics (None if PID unavailable).
    pub metrics: Option<SessionProcessMetrics>,
    /// Activity signal.
    pub activity: SessionActivity,
}

/// Top-level supervision report.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TmuxSessionSupervisionReport {
    /// Schema identifier.
    pub schema: &'static str,
    /// Observation timestamp (unix ms).
    pub observed_at_unix_ms: u128,
    /// Total sessions observed.
    pub session_count: usize,
    /// Total panes observed.
    pub pane_count: usize,
    /// Sessions with at least one attached client.
    pub active_sessions: usize,
    /// Aggregated CPU across all sessions.
    pub total_cpu_percent: f32,
    /// Aggregated RSS across all sessions (bytes).
    pub total_rss_bytes: u64,
    /// Supervised sessions.
    pub sessions: Vec<SupervisedTmuxSession>,
    /// Detail when tmux is unavailable.
    pub detail: Option<String>,
}

/// Port for tmux session supervision adapters.
pub trait TmuxSessionSupervisionPort {
    /// Observe all tmux sessions with full telemetry.
    fn observe_sessions(&self) -> TmuxSessionSupervisionReport;
}

/// Default observer using tmux CLI + /proc.
#[derive(Debug, Clone, Default)]
pub struct LocalTmuxSessionSupervisor;

impl LocalTmuxSessionSupervisor {
    pub fn new() -> Self {
        Self
    }
}

impl TmuxSessionSupervisionPort for LocalTmuxSessionSupervisor {
    fn observe_sessions(&self) -> TmuxSessionSupervisionReport {
        let observed_at = observed_at_unix_ms();

        let output = std::process::Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}\t#{session_windows}\t#{session_attached}",
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let sessions = parse_session_list(&output.stdout);
                let supervised: Vec<SupervisedTmuxSession> =
                    sessions.into_iter().map(|s| observe_session(&s)).collect();

                let pane_count = supervised.iter().map(|s| s.panes.len()).sum();
                let active_sessions = supervised.iter().filter(|s| s.attached_clients > 0).count();
                let total_cpu = supervised.iter().map(|s| s.total_cpu_percent).sum();
                let total_rss = supervised.iter().map(|s| s.total_rss_bytes).sum();

                TmuxSessionSupervisionReport {
                    schema: TMUX_SESSION_SUPERVISION_SCHEMA,
                    observed_at_unix_ms: observed_at,
                    session_count: supervised.len(),
                    pane_count,
                    active_sessions,
                    total_cpu_percent: total_cpu,
                    total_rss_bytes: total_rss,
                    sessions: supervised,
                    detail: None,
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                TmuxSessionSupervisionReport {
                    schema: TMUX_SESSION_SUPERVISION_SCHEMA,
                    observed_at_unix_ms: observed_at,
                    session_count: 0,
                    pane_count: 0,
                    active_sessions: 0,
                    total_cpu_percent: 0.0,
                    total_rss_bytes: 0,
                    sessions: Vec::new(),
                    detail: Some(if stderr.is_empty() {
                        "tmux returned no sessions".to_string()
                    } else {
                        stderr
                    }),
                }
            }
            Err(error) => TmuxSessionSupervisionReport {
                schema: TMUX_SESSION_SUPERVISION_SCHEMA,
                observed_at_unix_ms: observed_at,
                session_count: 0,
                pane_count: 0,
                active_sessions: 0,
                total_cpu_percent: 0.0,
                total_rss_bytes: 0,
                sessions: Vec::new(),
                detail: Some(format!("failed to invoke tmux: {error}")),
            },
        }
    }
}

fn parse_session_list(bytes: &[u8]) -> Vec<TmuxSessionObservation> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim();
            let windows = fields.next()?.trim().parse().ok()?;
            let attached_clients = fields.next()?.trim().parse().ok()?;
            if name.is_empty() {
                return None;
            }
            Some(TmuxSessionObservation {
                name: name.to_string(),
                windows,
                attached_clients,
            })
        })
        .collect()
}

fn observe_session(session: &TmuxSessionObservation) -> SupervisedTmuxSession {
    let panes = observe_panes(&session.name);
    let session_uptime = panes
        .iter()
        .filter_map(|p| p.metrics.as_ref().map(|m| m.uptime_secs))
        .max()
        .unwrap_or(0);
    let total_cpu = panes
        .iter()
        .filter_map(|p| p.metrics.as_ref().map(|m| m.cpu_percent))
        .sum();
    let total_rss = panes
        .iter()
        .filter_map(|p| p.metrics.as_ref().map(|m| m.rss_bytes))
        .sum();

    SupervisedTmuxSession {
        name: session.name.clone(),
        windows: session.windows,
        attached_clients: session.attached_clients,
        panes,
        session_uptime_secs: session_uptime,
        total_cpu_percent: total_cpu,
        total_rss_bytes: total_rss,
    }
}

fn observe_panes(session_name: &str) -> Vec<SupervisedTmuxPane> {
    let output = std::process::Command::new("tmux")
        .args([
            "list-panes",
            "-s",
            "-t",
            session_name,
            "-F",
            "#{pane_index}\t#{pane_pid}\t#{pane_current_command}",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let index = fields.next()?.trim().parse().ok()?;
                let pid = fields.next()?.trim().parse().ok()?;
                let current_command = fields.next()?.trim().to_string();
                Some((index, pid, current_command))
            })
            .map(|(index, pid, current_command)| {
                let metrics = observe_process_metrics(pid);
                let activity = observe_pane_activity(session_name, index);
                SupervisedTmuxPane {
                    index,
                    pid,
                    current_command,
                    metrics,
                    activity,
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn observe_process_metrics(pid: u32) -> Option<SessionProcessMetrics> {
    let stat_path = format!("/proc/{pid}/stat");
    let stat_content = std::fs::read_to_string(&stat_path).ok()?;

    // Parse /proc/pid/stat: comm may contain spaces and parens, find last ')'
    let (pre_comm, post_comm) = stat_content.rsplit_once(')')?;
    let comm = pre_comm.trim_start_matches('(').to_string();
    let fields: Vec<&str> = post_comm.split_whitespace().collect();

    // Field indices after comm (0-indexed in `fields`):
    // 0=state, ..., 11=utime, 12=stime, 17=num_threads, 19=starttime, 20=vsize, 21=rss
    let state = fields.first().map(|s| s.to_string()).unwrap_or_default();
    let num_threads = fields.get(17).and_then(|s| s.parse().ok()).unwrap_or(0);
    let starttime_ticks = fields
        .get(19)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let vms_bytes = fields.get(20).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rss_pages = fields
        .get(21)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let rss_bytes = rss_pages * page_size;

    let uptime_secs = process_uptime_secs(starttime_ticks);
    let cpu_percent = estimate_cpu_usage(pid, uptime_secs);

    Some(SessionProcessMetrics {
        pid,
        comm,
        rss_bytes,
        vms_bytes,
        cpu_percent,
        state,
        threads: num_threads,
        uptime_secs,
    })
}

fn process_uptime_secs(starttime_ticks: u64) -> u64 {
    let sys_uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .unwrap_or(0.0);

    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
    if ticks_per_sec == 0 {
        return 0;
    }
    let start_secs = starttime_ticks / ticks_per_sec;
    let sys_uptime_secs = sys_uptime as u64;

    sys_uptime_secs.saturating_sub(start_secs)
}

fn estimate_cpu_usage(pid: u32, uptime_secs: u64) -> f32 {
    if uptime_secs == 0 {
        return 0.0;
    }

    let stat_path = format!("/proc/{pid}/stat");
    let Ok(content) = std::fs::read_to_string(&stat_path) else {
        return 0.0;
    };

    let Some((_, post_comm)) = content.rsplit_once(')') else {
        return 0.0;
    };
    let fields: Vec<&str> = post_comm.split_whitespace().collect();
    let utime: u64 = fields.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
    let stime: u64 = fields.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
    if ticks_per_sec == 0 {
        return 0.0;
    }

    let total_time_secs = (utime + stime) as f32 / ticks_per_sec as f32;
    (total_time_secs / uptime_secs as f32) * 100.0
}

fn observe_pane_activity(session_name: &str, pane_index: u32) -> SessionActivity {
    let output = std::process::Command::new("tmux")
        .args([
            "capture-pane",
            "-p",
            "-t",
            &format!("{session_name}:{pane_index}"),
            "-S",
            "-50",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().collect();
            let total_lines = lines.len() as u32;

            let last_line = lines
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .map(|l| {
                    let t = l.trim();
                    if t.len() > 200 { &t[..200] } else { t }
                })
                .unwrap_or("")
                .to_string();

            let appears_at_prompt = {
                let trimmed = last_line.trim_end();
                trimmed.ends_with('$')
                    || trimmed.ends_with('%')
                    || trimmed.ends_with('#')
                    || trimmed.ends_with('>')
            };

            let appears_running =
                !last_line.is_empty() && !appears_at_prompt && last_line.trim().len() < 120;

            SessionActivity {
                total_lines,
                visible_lines: total_lines.min(50),
                last_line,
                appears_at_prompt,
                appears_running,
            }
        }
        _ => SessionActivity {
            total_lines: 0,
            visible_lines: 0,
            last_line: String::new(),
            appears_at_prompt: false,
            appears_running: false,
        },
    }
}

fn observed_at_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl fmt::Display for TmuxSessionSupervisionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "tmux session supervision report")?;
        writeln!(f, "  schema: {}", self.schema)?;
        writeln!(f, "  observed_at: {}", self.observed_at_unix_ms)?;
        writeln!(f, "  sessions: {}", self.session_count)?;
        writeln!(f, "  panes: {}", self.pane_count)?;
        writeln!(f, "  active_sessions: {}", self.active_sessions)?;
        writeln!(f, "  total_cpu: {:.1}%", self.total_cpu_percent)?;
        writeln!(f, "  total_rss: {} MB", self.total_rss_bytes / 1_048_576)?;
        for session in &self.sessions {
            writeln!(
                f,
                "  session: {} (windows={}, attached={}, uptime={}s, cpu={:.1}%, rss={}MB)",
                session.name,
                session.windows,
                session.attached_clients,
                session.session_uptime_secs,
                session.total_cpu_percent,
                session.total_rss_bytes / 1_048_576,
            )?;
            for pane in &session.panes {
                let metrics_str = match &pane.metrics {
                    Some(m) => format!(
                        "cpu={:.1}% rss={}MB threads={} state={}",
                        m.cpu_percent,
                        m.rss_bytes / 1_048_576,
                        m.threads,
                        m.state,
                    ),
                    None => "no metrics".to_string(),
                };
                writeln!(
                    f,
                    "    pane[{}] pid={} cmd=\"{}\" [{}]",
                    pane.index, pane.pid, pane.current_command, metrics_str,
                )?;
                writeln!(f, "      last_line: {}", pane.activity.last_line)?;
                writeln!(
                    f,
                    "      at_prompt={} running={}",
                    pane.activity.appears_at_prompt, pane.activity.appears_running,
                )?;
            }
        }
        if let Some(detail) = &self.detail {
            writeln!(f, "  detail: {detail}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_list_output() {
        let sessions = parse_session_list(b"pi\t2\t1\nbuild\t1\t0\n");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "pi");
        assert_eq!(sessions[0].windows, 2);
        assert_eq!(sessions[0].attached_clients, 1);
        assert_eq!(sessions[1].name, "build");
        assert_eq!(sessions[1].attached_clients, 0);
    }

    #[test]
    fn parses_empty_session_list() {
        let sessions = parse_session_list(b"");
        assert!(sessions.is_empty());
    }

    #[test]
    fn report_display_includes_session_count() {
        let report = TmuxSessionSupervisionReport {
            schema: TMUX_SESSION_SUPERVISION_SCHEMA,
            observed_at_unix_ms: 0,
            session_count: 2,
            pane_count: 3,
            active_sessions: 1,
            total_cpu_percent: 12.5,
            total_rss_bytes: 104857600,
            sessions: Vec::new(),
            detail: None,
        };
        let display = format!("{report}");
        assert!(display.contains("sessions: 2"));
        assert!(display.contains("panes: 3"));
        assert!(display.contains("12.5%"));
    }

    #[test]
    fn activity_prompt_detection_heuristic() {
        let line = "user@host:~$ ";
        let trimmed = line.trim_end();
        assert!(trimmed.ends_with('$'));

        let line2 = "Compiling cynic-host v0.1.0...";
        assert!(!line2.trim_end().ends_with('$'));
    }
}
