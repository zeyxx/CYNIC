//! Consumer loop for the supervision watch pattern.
//!
//! The producer (`parent::run_role`) publishes; this loop reads the status
//! file, decides via `cynic_protocol::supervision::decide_watch`, and
//! appends ledger entries. It never restarts, never signals, never spawns:
//! a consumer that acts is a second producer. Restart belongs to the outer
//! ring (systemd); this loop only records.
//!
//! Write policy bounds volume: a decision change always records; a repeated
//! `fresh` records at most once per `confirm_interval_secs` (an hour by
//! default — a healthy month is ~720 lines, not 500k). Terminal decisions
//! record once per heartbeat counter; a republished terminal envelope with
//! a new counter records again.

use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_protocol::supervision::{
    ParentStatusEnvelope, WatchDecision, WatchInput, WatchLedgerEntry, WatchVerdict, decide_watch,
};

/// Consumer configuration. All paths are caller-owned; the loop creates the
/// ledger file but never parent directories — a missing directory is a loud
/// startup error, not a silent default.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub role_id: String,
    pub status_file: PathBuf,
    pub ledger_file: PathBuf,
    pub poll_interval_secs: u64,
    pub stale_after_ms: u128,
    pub confirm_interval_secs: u64,
}

/// How the watch loop ended. The loop never ends on what it observes —
/// `stale` and terminal decisions are recorded, not acted on. Only a
/// process signal or an I/O failure ends it; the outer ring owns restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchRunOutcome {
    Signaled,
}

/// Mutable poll state: what was last recorded, so repeats stay quiet.
#[derive(Debug, Clone, Default)]
pub struct WatchPollState {
    last_decision: Option<WatchDecision>,
    last_counter: Option<u64>,
    last_write_ms: u128,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_millis())
        .unwrap_or(0)
}

/// Owned outcome of one status file read. Corrupt carries the reason, never
/// the bytes.
#[derive(Debug)]
enum StatusRead {
    Live(ParentStatusEnvelope),
    Missing,
    Corrupt(String),
}

fn read_status(status_file: &Path) -> StatusRead {
    match std::fs::read(status_file) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => StatusRead::Missing,
        Err(e) => StatusRead::Corrupt(format!("status_file_unreadable:{}", e.kind())),
        Ok(raw) => match serde_json::from_slice::<ParentStatusEnvelope>(&raw) {
            Ok(envelope) => StatusRead::Live(envelope),
            Err(_) => StatusRead::Corrupt("status_file_unparseable".to_string()),
        },
    }
}

/// Append one JSON line to the ledger (O_APPEND single write).
fn append_entry(ledger_file: &Path, entry: &WatchLedgerEntry) -> io::Result<()> {
    let mut line =
        serde_json::to_vec(entry).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_file)?;
    file.write_all(&line)
}

/// One poll: read, decide, maybe record. Returns the entry when one was
/// appended. I/O failures propagate — the outer ring owns restart, and a
/// silent watcher is worse than a dead one.
pub fn watch_once(
    cfg: &WatchConfig,
    state: &mut WatchPollState,
    now: u128,
) -> io::Result<Option<WatchLedgerEntry>> {
    let read = read_status(&cfg.status_file);
    let envelope = match &read {
        StatusRead::Live(env) => Some(env),
        _ => None,
    };
    let input = match &read {
        StatusRead::Live(env) => WatchInput::Envelope(env),
        StatusRead::Missing => WatchInput::Missing,
        StatusRead::Corrupt(reason) => WatchInput::Corrupt(reason.clone()),
    };
    let verdict: WatchVerdict = decide_watch(now, input, cfg.stale_after_ms);
    let entry = WatchLedgerEntry::new(&cfg.role_id, now, &verdict, envelope, now);

    let counter = envelope.map(|env| env.heartbeat_counter);
    let changed =
        Some(&entry.decision) != state.last_decision.as_ref() || counter != state.last_counter;
    let confirm_due = entry.decision == WatchDecision::Fresh
        && now.saturating_sub(state.last_write_ms)
            >= (cfg.confirm_interval_secs as u128).saturating_mul(1000);
    if changed || confirm_due {
        append_entry(&cfg.ledger_file, &entry)?;
        state.last_decision = Some(entry.decision.clone());
        state.last_counter = counter;
        state.last_write_ms = now;
        Ok(Some(entry))
    } else {
        Ok(None)
    }
}

/// Run the consumer until a process signal. Records per the write policy;
/// signals end the loop without a farewell entry (the ledger records
/// observations, not the observer's biography).
pub async fn watch_role(cfg: &WatchConfig) -> io::Result<WatchRunOutcome> {
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    #[cfg(unix)]
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut state = WatchPollState::default();
    loop {
        watch_once(cfg, &mut state, now_ms())?;
        for _ in 0..cfg.poll_interval_secs.max(1) {
            #[cfg(unix)]
            {
                if signaled(&mut sigterm, &mut sigint).await {
                    return Ok(WatchRunOutcome::Signaled);
                }
            }
            #[cfg(not(unix))]
            {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

#[cfg(unix)]
async fn signaled(
    sigterm: &mut tokio::signal::unix::Signal,
    sigint: &mut tokio::signal::unix::Signal,
) -> bool {
    use std::future::poll_fn;
    use std::pin::Pin;
    use std::task::Poll;
    poll_fn(|cx| {
        for sig in [sigterm, sigint] {
            if let Poll::Ready(signal) = Pin::new(sig).poll_recv(cx) {
                if signal.is_some() {
                    return Poll::Ready(true);
                }
            }
        }
        Poll::Ready(false)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::supervision::{
        PARENT_STATUS_FILE_SCHEMA, SUPERVISOR_OBSERVATION_SCHEMA, SupervisorObservation,
        SupervisorState,
    };

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cynic-watch-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_cfg(dir: &Path) -> WatchConfig {
        WatchConfig {
            role_id: "node".to_string(),
            status_file: dir.join("node.status.json"),
            ledger_file: dir.join("watch.jsonl"),
            poll_interval_secs: 1,
            stale_after_ms: 10_000,
            confirm_interval_secs: 3_600,
        }
    }

    fn write_envelope(dir: &Path, state: SupervisorState, counter: u64, observed_at: u128) {
        let envelope = ParentStatusEnvelope {
            schema: PARENT_STATUS_FILE_SCHEMA.to_string(),
            heartbeat_counter: counter,
            observation: SupervisorObservation {
                schema: SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
                role_id: "node".to_string(),
                state,
                restart_count: 0,
                last_exit_reason: None,
                observed_at_unix_ms: observed_at,
                residual_unknowns: Vec::new(),
            },
        };
        let payload = serde_json::to_vec_pretty(&envelope).unwrap();
        std::fs::write(dir.join("node.status.json"), payload).unwrap();
    }

    fn ledger_entries(dir: &Path) -> Vec<WatchLedgerEntry> {
        let raw = std::fs::read(dir.join("watch.jsonl")).unwrap();
        raw.split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }

    #[test]
    fn watch_once_records_change_and_stays_quiet_on_repeat() {
        let dir = test_dir("change-quiet");
        let cfg = test_cfg(&dir);
        let mut state = WatchPollState::default();
        // Missing file records unknown once, then stays quiet.
        assert!(watch_once(&cfg, &mut state, 1_000).unwrap().is_some());
        assert!(watch_once(&cfg, &mut state, 2_000).unwrap().is_none());
        assert_eq!(ledger_entries(&dir).len(), 1);
        // Fresh envelope records the change...
        write_envelope(&dir, SupervisorState::Active, 1, 3_000);
        assert!(watch_once(&cfg, &mut state, 3_100).unwrap().is_some());
        // ...then repeats stay quiet until the hourly confirmation.
        assert!(watch_once(&cfg, &mut state, 3_200).unwrap().is_none());
        assert!(watch_once(&cfg, &mut state, 3_600_000).unwrap().is_none());
        assert!(
            watch_once(&cfg, &mut state, 3_100 + 3_600_000)
                .unwrap()
                .is_some()
        );
        assert_eq!(ledger_entries(&dir).len(), 3);
    }

    #[test]
    fn watch_once_records_stale_and_terminal_once_per_counter() {
        let dir = test_dir("stale-terminal");
        let cfg = test_cfg(&dir);
        let mut state = WatchPollState::default();
        // Stale heartbeat records, then repeats stay quiet (not fresh).
        write_envelope(&dir, SupervisorState::Active, 1, 1_000);
        assert!(watch_once(&cfg, &mut state, 100_000).unwrap().is_some());
        assert!(watch_once(&cfg, &mut state, 200_000).unwrap().is_none());
        // Terminal records once per counter, even as the file ages.
        write_envelope(&dir, SupervisorState::Stopped, 2, 200_000);
        assert!(watch_once(&cfg, &mut state, 900_000).unwrap().is_some());
        assert!(watch_once(&cfg, &mut state, 9_900_000).unwrap().is_none());
        let entries = ledger_entries(&dir);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].decision, WatchDecision::Stale);
        assert_eq!(entries[2].decision, WatchDecision::TerminalExpected);
    }

    #[test]
    fn watch_once_marks_corrupt_unknown_without_bytes() {
        let dir = test_dir("corrupt");
        let cfg = test_cfg(&dir);
        std::fs::write(dir.join("node.status.json"), b"{not json").unwrap();
        let mut state = WatchPollState::default();
        let entry = watch_once(&cfg, &mut state, 1_000).unwrap().unwrap();
        assert_eq!(entry.decision, WatchDecision::Unknown);
        assert_eq!(entry.heartbeat_counter, None);
        assert!(!entry.reason.contains("{not json"));
    }
}
