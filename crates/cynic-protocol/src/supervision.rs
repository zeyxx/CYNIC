use serde::Serialize;

pub const SERVICE_SUPERVISOR_OBSERVATION_SCHEMA: &str = "cynic.service_supervisor_observation.v1";
pub const SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID: &str = "service_supervisor_observation";

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorObservation {
    pub schema: &'static str,
    pub evidence_id: &'static str,
    pub status: &'static str,
    pub subject: String,
    pub supervisor_kind: String,
    pub supervisor_scope: String,
    pub target_source: String,
    pub observed_at_unix_ms: u128,
    pub service_count: usize,
    pub active_count: usize,
    pub degraded_count: usize,
    pub unknown_count: usize,
    pub services: Vec<ServiceSupervisorUnitObservation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorUnitObservation {
    pub service_id: String,
    pub unit_name: String,
    pub health_port: Option<u16>,
    pub health_kind: Option<String>,
    pub desired_provider: Option<String>,
    pub selected_provider: String,
    pub selected_scope: String,
    pub target_source: String,
    pub desired_state: String,
    pub restart_policy: String,
    pub probe_status: &'static str,
    pub probe_detail: String,
    pub provider_attempts: Vec<ServiceSupervisorProviderAttempt>,
    pub load_state: String,
    pub unit_file_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub restart_count: Option<u64>,
    pub active_enter_timestamp_monotonic: Option<u64>,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorProviderAttempt {
    pub provider: String,
    pub scope: String,
    pub probe_status: &'static str,
    pub probe_detail: String,
}

pub fn service_supervisor_status(
    units: &[ServiceSupervisorUnitObservation],
) -> (&'static str, usize, usize, usize) {
    let active_count = units.iter().filter(|unit| unit.status == "active").count();
    let degraded_count = units
        .iter()
        .filter(|unit| unit.status == "degraded")
        .count();
    let unknown_count = units.iter().filter(|unit| unit.status == "unknown").count();
    let status = if units.is_empty() || unknown_count > 0 {
        "unknown"
    } else if degraded_count > 0 {
        "degraded"
    } else {
        "active"
    };
    (status, active_count, degraded_count, unknown_count)
}

/// Wire schema for supervisor observations (`cynic supervise observe` emits these).
pub const SUPERVISOR_OBSERVATION_SCHEMA: &str = "cynic.supervisor_observation.v1";
/// Declared supervisor roles from `contracts/supervisor-contract.json`
/// `lifecycle.roles`. Mirrored here as the materialised contract (Layer-0);
/// a coherence test pins this list against the JSON source.
pub const SUPERVISED_ROLE_IDS: &[&str] = &["node", "kernel", "mcp"];
/// Backoff defaults from the contract `lifecycle.restart.backoff` section.
pub const SUPERVISOR_BACKOFF_INITIAL_SECS: u64 = 5;
pub const SUPERVISOR_BACKOFF_MAX_SECS: u64 = 60;
pub const SUPERVISOR_MIN_UPTIME_SECS: u64 = 10;
/// Stop grace from the contract `lifecycle.signals.stop` section.
pub const SUPERVISOR_STOP_GRACE_SECS: u64 = 15;

/// Generic supervisor lifecycle state. Node-specific outcomes (such as
/// `ExitReason` in `cynic-node`) map onto these states at the adapter edge;
/// they are not moved here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorState {
    Planned,
    Starting,
    Active,
    Draining,
    Stopped,
    Failed,
    Unknown,
}

impl SupervisorState {
    /// Contract state name, byte-stable for observations and records.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Starting => "starting",
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a contract state name. Returns `None` for anything outside the
    /// 7 declared names; callers map `None` to `Unknown` with an explicit
    /// residual unknown rather than coercing silently.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "planned" => Self::Planned,
            "starting" => Self::Starting,
            "active" => Self::Active,
            "draining" => Self::Draining,
            "stopped" => Self::Stopped,
            "failed" => Self::Failed,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }
}

impl Default for SupervisorState {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Canonical restart policy. Delay values default from the contract; the
/// attempt bound stays caller-supplied because the contract declares no
/// `max_attempts` (node config carries its own bound, which diverges and is
/// migrated in a later slice, not here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestartPolicy {
    pub max_attempts: u32,
    pub initial_delay_secs: u64,
    pub max_delay_secs: u64,
    pub min_uptime_secs: u64,
}

impl RestartPolicy {
    /// Contract delays with a caller-supplied attempt bound.
    pub fn from_contract(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            initial_delay_secs: SUPERVISOR_BACKOFF_INITIAL_SECS,
            max_delay_secs: SUPERVISOR_BACKOFF_MAX_SECS,
            min_uptime_secs: SUPERVISOR_MIN_UPTIME_SECS,
        }
    }
}

/// Pure backoff counter. Time enters only as `u64` seconds supplied by the
/// caller; this type never reads a clock, sleeps, or spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackoffState {
    pub attempt: u32,
}

impl BackoffState {
    /// Increment the attempt counter and return the next delay in seconds,
    /// capped at `policy.max_delay_secs`.
    ///
    /// Delay formula mirrors the node supervisor: `initial * 2^(attempt-1)`
    /// after increment, with the shift clamped to avoid overflow.
    pub fn next_delay_secs(&mut self, policy: &RestartPolicy) -> u64 {
        self.attempt = self.attempt.saturating_add(1);
        delay_for_attempt(policy, self.attempt)
    }

    /// Unconditionally reset the attempt counter.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Reset only when the process lived at least `min_uptime_secs`.
    pub fn reset_if_stable(&mut self, uptime_secs: u64, min_uptime_secs: u64) {
        if uptime_secs >= min_uptime_secs {
            self.attempt = 0;
        }
    }

    /// Returns `true` when `attempt >= policy.max_attempts`.
    pub fn exhausted(&self, policy: &RestartPolicy) -> bool {
        self.attempt >= policy.max_attempts
    }
}

/// Pure delay for a 1-based attempt number, without mutating any state.
pub fn delay_for_attempt(policy: &RestartPolicy, attempt: u32) -> u64 {
    if attempt == 0 {
        return policy.initial_delay_secs.min(policy.max_delay_secs);
    }
    let shift = (attempt - 1).min(30);
    let multiplier = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
    policy
        .initial_delay_secs
        .saturating_mul(multiplier)
        .min(policy.max_delay_secs)
}

/// Generic restart decision. The caller owns clocks, sleeps, and spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDecision {
    /// Restart after `delay_secs`.
    Restart { delay_secs: u64 },
    /// Do not restart; the failure bound is reached.
    GiveUp { reason: &'static str },
    /// Do not restart; there was no failure.
    NoRestart,
}

/// Pure restart choice: no failure never restarts, stable uptime resets the
/// counter before the bound check, an exhausted bound gives up.
pub fn decide_restart(
    policy: &RestartPolicy,
    attempt: u32,
    uptime_secs: u64,
    failed: bool,
) -> RestartDecision {
    if !failed {
        return RestartDecision::NoRestart;
    }
    let effective_attempt = if uptime_secs >= policy.min_uptime_secs {
        0
    } else {
        attempt
    };
    if effective_attempt >= policy.max_attempts {
        return RestartDecision::GiveUp {
            reason: "max_restart_attempts_reached",
        };
    }
    RestartDecision::Restart {
        delay_secs: delay_for_attempt(policy, effective_attempt.saturating_add(1)),
    }
}

/// Read-only supervisor observation for one role, per
/// `contracts/supervisor-contract.json` `observation.required_fields`.
/// Emitted by `cynic supervise observe`; never constructed by mutation paths.
/// `state` is `Unknown` with a non-empty `residual_unknowns` whenever no
/// supervisor daemon holds live state for the role.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupervisorObservation {
    pub schema: String,
    pub role_id: String,
    pub state: SupervisorState,
    pub restart_count: u32,
    pub last_exit_reason: Option<String>,
    pub observed_at_unix_ms: u128,
    pub residual_unknowns: Vec<String>,
}

impl SupervisorObservation {
    /// Explicit-unknown observation: no daemon holds this role yet.
    pub fn unknown_role(role_id: &str, observed_at_unix_ms: u128, reason: &str) -> Self {
        Self {
            schema: SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
            role_id: role_id.to_string(),
            state: SupervisorState::Unknown,
            restart_count: 0,
            last_exit_reason: None,
            observed_at_unix_ms,
            residual_unknowns: vec![reason.to_string()],
        }
    }
}

/// Map a unit health status onto the generic lifecycle state (G3 decision:
/// `degraded` blocks like `failed`; `unknown` stays unknown; `draining` is
/// reserved for commanded stops and never produced here).
///
/// Total by construction: the `_` arm preserves today's `else` branch —
/// unnormalised provider output (raw systemd states such as `activating`
/// or `inactive`) maps to `Unknown`, which callers pair with an explicit
/// residual unknown. Nothing is coerced to active.
pub fn unit_status_state(status: &str) -> SupervisorState {
    match status {
        "active" => SupervisorState::Active,
        "degraded" => SupervisorState::Failed,
        "unknown" => SupervisorState::Unknown,
        _ => SupervisorState::Unknown,
    }
}

/// Wire schema for consumer ledger entries (`cynic supervise watch` appends
/// these, one JSON object per line).
pub const WATCH_LEDGER_ENTRY_SCHEMA: &str = "cynic.supervisor_watch_ledger.v1";

/// What the consumer read from the status file. The caller owns file I/O;
/// this enum carries only the outcome so `decide_watch` stays pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchInput<'a> {
    /// A parsed envelope.
    Envelope(&'a ParentStatusEnvelope),
    /// No file at the status path.
    Missing,
    /// A file that does not parse; carries why, not the bytes.
    Corrupt(String),
}

/// Consumer decision. The consumer observes and records only: `Stale` never
/// implies the child is dead (a silent parent says nothing about its child),
/// and `Unknown` never becomes `Failed` without a terminal envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchDecision {
    Fresh,
    Stale,
    Unknown,
    TerminalExpected,
    TerminalFailed,
}

impl WatchDecision {
    /// Ledger-stable decision name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
            Self::TerminalExpected => "terminal_expected",
            Self::TerminalFailed => "terminal_failed",
        }
    }
}

/// One consumer verdict: the decision plus the reason that produced it.
/// Reasons are reader-facing strings; the decision is the machine contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchVerdict {
    pub decision: WatchDecision,
    pub reason: String,
}

/// Pure consumer choice. Terminal envelopes win over age (a `stopped` file
/// that keeps aging is still an expected end, not a missed beat); anything
/// else is fresh within `stale_after_ms`, stale past it. Missing and corrupt
/// inputs are `Unknown` with the reason preserved, never coerced.
pub fn decide_watch(
    now_unix_ms: u128,
    input: WatchInput<'_>,
    stale_after_ms: u128,
) -> WatchVerdict {
    match input {
        WatchInput::Missing => WatchVerdict {
            decision: WatchDecision::Unknown,
            reason: "status_file_missing".to_string(),
        },
        WatchInput::Corrupt(reason) => WatchVerdict {
            decision: WatchDecision::Unknown,
            reason,
        },
        WatchInput::Envelope(envelope) => match envelope.observation.state {
            SupervisorState::Stopped => WatchVerdict {
                decision: WatchDecision::TerminalExpected,
                reason: "parent_reported_stopped".to_string(),
            },
            SupervisorState::Failed => WatchVerdict {
                decision: WatchDecision::TerminalFailed,
                reason: "parent_reported_failed".to_string(),
            },
            _ => {
                let age_ms = now_unix_ms.saturating_sub(envelope.observation.observed_at_unix_ms);
                if age_ms <= stale_after_ms {
                    WatchVerdict {
                        decision: WatchDecision::Fresh,
                        reason: "heartbeat_within_bound".to_string(),
                    }
                } else {
                    WatchVerdict {
                        decision: WatchDecision::Stale,
                        reason: "heartbeat_age_exceeded_bound".to_string(),
                    }
                }
            }
        },
    }
}

/// One ledger line: the verdict plus the envelope facts that support it.
/// `heartbeat_age_ms`, `heartbeat_counter`, `observed_state`, and
/// `restart_count` are `None` exactly when no envelope was read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WatchLedgerEntry {
    pub schema: String,
    pub role_id: String,
    pub decided_at_unix_ms: u128,
    pub decision: WatchDecision,
    pub reason: String,
    pub heartbeat_age_ms: Option<u128>,
    pub heartbeat_counter: Option<u64>,
    pub observed_state: Option<SupervisorState>,
    pub restart_count: Option<u32>,
    pub residual_unknowns: Vec<String>,
}

impl WatchLedgerEntry {
    /// Build the entry from a verdict; the caller supplies the envelope
    /// facts (or `None` when the verdict came from `Missing`/`Corrupt`).
    pub fn new(
        role_id: &str,
        decided_at_unix_ms: u128,
        verdict: &WatchVerdict,
        envelope: Option<&ParentStatusEnvelope>,
        now_unix_ms: u128,
    ) -> Self {
        let (heartbeat_age_ms, heartbeat_counter, observed_state, restart_count) = match envelope {
            Some(env) => (
                Some(now_unix_ms.saturating_sub(env.observation.observed_at_unix_ms)),
                Some(env.heartbeat_counter),
                Some(env.observation.state),
                Some(env.observation.restart_count),
            ),
            None => (None, None, None, None),
        };
        Self {
            schema: WATCH_LEDGER_ENTRY_SCHEMA.to_string(),
            role_id: role_id.to_string(),
            decided_at_unix_ms,
            decision: verdict.decision.clone(),
            reason: verdict.reason.clone(),
            heartbeat_age_ms,
            heartbeat_counter,
            observed_state,
            restart_count,
            residual_unknowns: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_status_does_not_hide_unknown_units() {
        let units = vec![ServiceSupervisorUnitObservation {
            service_id: "kernel".to_string(),
            unit_name: "cynic-kernel.service".to_string(),
            health_port: Some(3030),
            health_kind: Some("tcp".to_string()),
            desired_provider: Some("process-port".to_string()),
            selected_provider: "systemd".to_string(),
            selected_scope: "system".to_string(),
            target_source: "test".to_string(),
            desired_state: "active".to_string(),
            restart_policy: "on-failure".to_string(),
            probe_status: "ok",
            probe_detail: "test".to_string(),
            provider_attempts: vec![ServiceSupervisorProviderAttempt {
                provider: "systemd".to_string(),
                scope: "system".to_string(),
                probe_status: "ok",
                probe_detail: "test".to_string(),
            }],
            load_state: "unknown".to_string(),
            unit_file_state: "unknown".to_string(),
            active_state: "unknown".to_string(),
            sub_state: "unknown".to_string(),
            restart_count: None,
            active_enter_timestamp_monotonic: None,
            status: "unknown",
        }];
        let (status, active, degraded, unknown) = service_supervisor_status(&units);
        assert_eq!(status, "unknown");
        assert_eq!(active, 0);
        assert_eq!(degraded, 0);
        assert_eq!(unknown, 1);
    }

    #[test]
    fn contract_backoff_defaults_match_declared_policy() {
        assert_eq!(SUPERVISOR_BACKOFF_INITIAL_SECS, 5);
        assert_eq!(SUPERVISOR_BACKOFF_MAX_SECS, 60);
        assert_eq!(SUPERVISOR_MIN_UPTIME_SECS, 10);
        assert_eq!(SUPERVISOR_STOP_GRACE_SECS, 15);
        let policy = RestartPolicy::from_contract(5);
        assert_eq!(policy.initial_delay_secs, 5);
        assert_eq!(policy.max_delay_secs, 60);
        assert_eq!(policy.min_uptime_secs, 10);
    }

    #[test]
    fn backoff_delay_progression_caps_at_max() {
        let policy = RestartPolicy::from_contract(10);
        let mut state = BackoffState::default();
        assert_eq!(state.next_delay_secs(&policy), 5);
        assert_eq!(state.next_delay_secs(&policy), 10);
        assert_eq!(state.next_delay_secs(&policy), 20);
        assert_eq!(state.next_delay_secs(&policy), 40);
        assert_eq!(state.next_delay_secs(&policy), 60);
        assert_eq!(state.next_delay_secs(&policy), 60);
    }

    #[test]
    fn backoff_reset_if_stable_respects_boundary() {
        let mut state = BackoffState { attempt: 3 };
        state.reset_if_stable(9, 10);
        assert_eq!(state.attempt, 3);
        state.reset_if_stable(10, 10);
        assert_eq!(state.attempt, 0);
    }

    #[test]
    fn decide_restart_covers_branches() {
        let policy = RestartPolicy::from_contract(2);
        assert_eq!(
            decide_restart(&policy, 0, 0, false),
            RestartDecision::NoRestart
        );
        assert_eq!(
            decide_restart(&policy, 0, 0, true),
            RestartDecision::Restart { delay_secs: 5 }
        );
        assert_eq!(
            decide_restart(&policy, 2, 0, true),
            RestartDecision::GiveUp {
                reason: "max_restart_attempts_reached"
            }
        );
        // Stable uptime resets the counter before the bound check.
        assert_eq!(
            decide_restart(&policy, 2, 10, true),
            RestartDecision::Restart { delay_secs: 5 }
        );
    }

    #[test]
    fn supervisor_state_names_match_contract() {
        let cases = [
            (SupervisorState::Planned, "planned"),
            (SupervisorState::Starting, "starting"),
            (SupervisorState::Active, "active"),
            (SupervisorState::Draining, "draining"),
            (SupervisorState::Stopped, "stopped"),
            (SupervisorState::Failed, "failed"),
            (SupervisorState::Unknown, "unknown"),
        ];
        for (state, name) in cases {
            assert_eq!(state.as_str(), name);
        }
        assert_eq!(SupervisorState::default(), SupervisorState::Unknown);
    }

    #[test]
    fn supervisor_state_parse_roundtrips_all_names() {
        for name in [
            "planned",
            "starting",
            "active",
            "draining",
            "stopped",
            "failed",
            "unknown",
        ] {
            let state = SupervisorState::parse(name)
                .unwrap_or_else(|| panic!("parse({name}) should succeed"));
            assert_eq!(state.as_str(), name);
        }
        assert_eq!(SupervisorState::parse(""), None);
        assert_eq!(SupervisorState::parse("Active"), None);
        assert_eq!(SupervisorState::parse("degraded"), None);
    }

    #[test]
    fn unknown_role_observation_is_explicit() {
        let obs = SupervisorObservation::unknown_role("node", 1_700_000_000_000, "no_daemon");
        assert_eq!(obs.schema, SUPERVISOR_OBSERVATION_SCHEMA);
        assert_eq!(obs.role_id, "node");
        assert_eq!(obs.state, SupervisorState::Unknown);
        assert_eq!(obs.restart_count, 0);
        assert_eq!(obs.last_exit_reason, None);
        assert_eq!(obs.residual_unknowns, vec!["no_daemon".to_string()]);
    }

    #[test]
    fn supervised_role_ids_match_contract() {
        assert_eq!(SUPERVISED_ROLE_IDS, &["node", "kernel", "mcp"]);
    }

    #[test]
    fn unit_status_state_maps_health_to_lifecycle() {
        use SupervisorState as S;
        assert_eq!(unit_status_state("active"), S::Active);
        assert_eq!(unit_status_state("degraded"), S::Failed);
        assert_eq!(unit_status_state("unknown"), S::Unknown);
        // Unnormalised provider output never coerces to active.
        for raw in ["activating", "inactive", "failed", "", "Active"] {
            assert_eq!(unit_status_state(raw), S::Unknown, "input: {raw}");
        }
    }

    fn watch_envelope(state: SupervisorState, observed_at_unix_ms: u128) -> ParentStatusEnvelope {
        ParentStatusEnvelope {
            schema: PARENT_STATUS_FILE_SCHEMA.to_string(),
            heartbeat_counter: 7,
            observation: SupervisorObservation {
                schema: SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
                role_id: "node".to_string(),
                state,
                restart_count: 1,
                last_exit_reason: None,
                observed_at_unix_ms,
                residual_unknowns: Vec::new(),
            },
        }
    }

    #[test]
    fn decide_watch_covers_all_branches() {
        use SupervisorState as S;
        use WatchDecision as D;
        // Fresh within the bound, stale past it.
        let env = watch_envelope(S::Active, 1_000);
        let v = decide_watch(1_000 + 9_999, WatchInput::Envelope(&env), 10_000);
        assert_eq!(v.decision, D::Fresh);
        let v = decide_watch(1_000 + 10_001, WatchInput::Envelope(&env), 10_000);
        assert_eq!(v.decision, D::Stale);
        // Terminal wins over age: an old stopped file is an expected end.
        let env = watch_envelope(S::Stopped, 1_000);
        let v = decide_watch(1_000 + 999_999, WatchInput::Envelope(&env), 10_000);
        assert_eq!(v.decision, D::TerminalExpected);
        let env = watch_envelope(S::Failed, 1_000);
        let v = decide_watch(1_000, WatchInput::Envelope(&env), 10_000);
        assert_eq!(v.decision, D::TerminalFailed);
        // Missing and corrupt never coerce.
        let v = decide_watch(5_000, WatchInput::Missing, 10_000);
        assert_eq!(v.decision, D::Unknown);
        assert_eq!(v.reason, "status_file_missing");
        let v = decide_watch(5_000, WatchInput::Corrupt("not_json".to_string()), 10_000);
        assert_eq!(v.decision, D::Unknown);
        assert_eq!(v.reason, "not_json");
    }

    #[test]
    fn watch_ledger_entry_binds_envelope_facts() {
        use SupervisorState as S;
        use WatchDecision as D;
        let env = watch_envelope(S::Active, 1_000);
        let verdict = decide_watch(1_500, WatchInput::Envelope(&env), 10_000);
        assert_eq!(verdict.decision, D::Fresh);
        let entry = WatchLedgerEntry::new("node", 1_500, &verdict, Some(&env), 1_500);
        assert_eq!(entry.schema, WATCH_LEDGER_ENTRY_SCHEMA);
        assert_eq!(entry.heartbeat_age_ms, Some(500));
        assert_eq!(entry.heartbeat_counter, Some(7));
        assert_eq!(entry.observed_state, Some(S::Active));
        assert_eq!(entry.restart_count, Some(1));
        // No envelope, no facts — never a zero-filled pretence.
        let verdict = decide_watch(1_500, WatchInput::Missing, 10_000);
        let entry = WatchLedgerEntry::new("node", 1_500, &verdict, None, 1_500);
        assert_eq!(entry.heartbeat_age_ms, None);
        assert_eq!(entry.heartbeat_counter, None);
        assert_eq!(entry.observed_state, None);
        assert_eq!(entry.restart_count, None);
        // Byte-stable decision names for the ledger.
        assert_eq!(D::Fresh.as_str(), "fresh");
        assert_eq!(D::TerminalExpected.as_str(), "terminal_expected");
    }
}
