//! Node-owned, opt-in network recovery.
//!
//! This is deliberately narrower than an operator network rollout.  It may
//! only invoke an already-installed bounded helper, once per tick, and it
//! records every attempt before a later tick can retry.  A successful process
//! exit is not recovery: only a post-action `mesh_active` observation is.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_host::runtime_layout::current_runtime_layout;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::adapters::local_network_command_runner::LocalNetworkCommandRunner;
use crate::domain::node_observation::{NodeRuntimeAction, NodeRuntimeSeverity, NodeRuntimeStatus};
use crate::network_actuator::CommandExecution;
use crate::network_actuator_status::activation_status;
use crate::network_command_surface::argv_command;
use crate::observe::{NodeObservation, observe_payload};
use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

pub const NODE_NETWORK_RECOVERY_TICK_SCHEMA: &str = "cynic.node_network_recovery_tick.v1";
const RECOVERY_STATE_SCHEMA: &str = "cynic.node_network_recovery_state.v1";

#[derive(Debug, Clone, Serialize)]
pub struct NodeNetworkRecoveryTickReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub action: Option<NodeRuntimeAction>,
    pub profile: Option<&'static str>,
    pub execution: Option<CommandExecution>,
    pub post_action_status: Option<NodeRuntimeStatus>,
    pub blockers: Vec<String>,
    pub state_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecoveryState {
    schema: String,
    consecutive_attempts: u32,
    last_attempt_unix_s: Option<u64>,
    last_action: Option<NodeRuntimeAction>,
}

impl RecoveryState {
    fn fresh() -> Self {
        Self {
            schema: RECOVERY_STATE_SCHEMA.to_string(),
            consecutive_attempts: 0,
            last_attempt_unix_s: None,
            last_action: None,
        }
    }
}

pub async fn run_node_network_recovery_loop(
    enabled: bool,
    registry_path: String,
    state_path: String,
    cooldown: Duration,
    max_attempts: u32,
    shutdown: CancellationToken,
) {
    let mut ticker = tokio::time::interval(cooldown.min(Duration::from_secs(30)));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let runner = LocalNetworkCommandRunner;
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                let report = reconcile_node_network_recovery_tick(
                    enabled,
                    &registry_path,
                    Path::new(&state_path),
                    cooldown,
                    max_attempts,
                    &runner,
                    observe_payload,
                );
                match report.status {
                    "proved_rejoined" => tracing::info!(action = ?report.action, "node network recovery proved fleet mesh return"),
                    "inactive" | "converged" | "cooldown" => tracing::debug!(status = report.status, "node network recovery idle"),
                    _ => tracing::warn!(status = report.status, blockers = ?report.blockers, "node network recovery did not converge"),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn reconcile_node_network_recovery_tick<R, O>(
    enabled: bool,
    registry_path: &str,
    state_path: &Path,
    cooldown: Duration,
    max_attempts: u32,
    runner: &R,
    observe: O,
) -> NodeNetworkRecoveryTickReport
where
    R: NetworkCommandRunnerPort,
    O: Fn(Option<&str>) -> NodeObservation,
{
    if !enabled {
        return report("inactive", None, None, None, None, Vec::new(), state_path);
    }
    let before = observe(Some(registry_path));
    if mesh_rejoined(&before) {
        let state = RecoveryState::fresh();
        return match persist_state(state_path, &state) {
            Ok(()) => report(
                "converged",
                None,
                None,
                None,
                Some(before.status),
                Vec::new(),
                state_path,
            ),
            Err(error) => report(
                "blocked",
                None,
                None,
                None,
                Some(before.status),
                vec![format!("network_recovery_state_write_failed:{error}")],
                state_path,
            ),
        };
    }

    let activation = activation_status(&before);
    if activation.status != "active" || !activation.root_authority.sudo_noninteractive_available {
        return report(
            "blocked",
            None,
            None,
            None,
            Some(before.status),
            vec!["bounded_network_actuator_unproved".to_string()],
            state_path,
        );
    }
    let state = match load_state(state_path) {
        Ok(state) => state,
        Err(error) => {
            return report(
                "blocked",
                None,
                None,
                None,
                Some(before.status),
                vec![format!("network_recovery_state_unreadable:{error}")],
                state_path,
            );
        }
    };
    let now = unix_seconds();
    if state.consecutive_attempts >= max_attempts {
        return report(
            "blocked",
            None,
            None,
            None,
            Some(before.status),
            vec!["network_recovery_attempt_budget_exhausted".to_string()],
            state_path,
        );
    }
    if state
        .last_attempt_unix_s
        .is_some_and(|last| now.saturating_sub(last) < cooldown.as_secs())
    {
        return report(
            "cooldown",
            None,
            None,
            None,
            Some(before.status),
            Vec::new(),
            state_path,
        );
    }
    let Some((action, profile)) = before.remediation.iter().find_map(|remediation| {
        profile_for_action(&remediation.action).map(|profile| (remediation.action.clone(), profile))
    }) else {
        return report(
            "blocked",
            None,
            None,
            None,
            Some(before.status),
            vec!["network_recovery_requires_an_uninstalled_or_non_network_adapter".to_string()],
            state_path,
        );
    };

    let mut next_state = state;
    next_state.schema = RECOVERY_STATE_SCHEMA.to_string();
    next_state.consecutive_attempts = next_state.consecutive_attempts.saturating_add(1);
    next_state.last_attempt_unix_s = Some(now);
    next_state.last_action = Some(action.clone());
    if let Err(error) = persist_state(state_path, &next_state) {
        return report(
            "blocked",
            Some(action),
            Some(profile),
            None,
            Some(before.status),
            vec![format!("network_recovery_state_write_failed:{error}")],
            state_path,
        );
    }

    let layout = current_runtime_layout();
    let command = argv_command(
        "node_network_recovery_apply",
        vec![
            layout.sudo_bin(),
            "-n".to_string(),
            layout.network_actuator_helper_path(),
            "apply".to_string(),
            "--profile".to_string(),
            profile.to_string(),
            "--apply".to_string(),
        ],
    );
    let execution = runner.run_command(&command);
    if execution.exit_code != Some(0) {
        return report(
            "failed",
            Some(action),
            Some(profile),
            Some(execution),
            Some(before.status),
            vec!["bounded_network_actuator_execution_failed".to_string()],
            state_path,
        );
    }
    let after = observe(Some(registry_path));
    if mesh_rejoined(&after) {
        if let Err(error) = persist_state(state_path, &RecoveryState::fresh()) {
            return report(
                "blocked",
                Some(action),
                Some(profile),
                Some(execution),
                Some(after.status),
                vec![format!("network_recovery_state_reset_failed:{error}")],
                state_path,
            );
        }
        return report(
            "proved_rejoined",
            Some(action),
            Some(profile),
            Some(execution),
            Some(after.status),
            Vec::new(),
            state_path,
        );
    }
    report(
        "unproved",
        Some(action),
        Some(profile),
        Some(execution),
        Some(after.status),
        vec!["network_recovery_post_action_mesh_readback_unproved".to_string()],
        state_path,
    )
}

fn mesh_rejoined(observation: &NodeObservation) -> bool {
    observation.status == NodeRuntimeStatus::MeshActive
        && observation.severity != NodeRuntimeSeverity::Critical
}

fn profile_for_action(action: &NodeRuntimeAction) -> Option<&'static str> {
    match action {
        NodeRuntimeAction::RestoreProviderFallbackRoute => Some("provider-fallback-route"),
        NodeRuntimeAction::ActivateSynapseRoute => Some("synapse-route"),
        NodeRuntimeAction::ActivateWireguardMesh => Some("wireguard-mesh"),
        NodeRuntimeAction::RestoreCriticalDns
        | NodeRuntimeAction::RestoreCanonicalDnsProjection
        | NodeRuntimeAction::RestoreCriticalTcp => None,
    }
}

fn load_state(path: &Path) -> Result<RecoveryState, String> {
    if !path.exists() {
        return Ok(RecoveryState::fresh());
    }
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let state: RecoveryState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))?;
    if state.schema != RECOVERY_STATE_SCHEMA {
        return Err("network recovery state schema mismatch".to_string());
    }
    Ok(state)
}

fn persist_state(path: &Path, state: &RecoveryState) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "network recovery state has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|error| format!("open {}: {error}", temporary.display()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("activate {}: {error}", path.display()))?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync {}: {error}", parent.display()))
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn report(
    status: &'static str,
    action: Option<NodeRuntimeAction>,
    profile: Option<&'static str>,
    execution: Option<CommandExecution>,
    post_action_status: Option<NodeRuntimeStatus>,
    blockers: Vec<String>,
    state_path: &Path,
) -> NodeNetworkRecoveryTickReport {
    NodeNetworkRecoveryTickReport {
        schema: NODE_NETWORK_RECOVERY_TICK_SCHEMA,
        status,
        action,
        profile,
        execution,
        post_action_status,
        blockers,
        state_path: state_path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::profile_for_action;
    use crate::domain::node_observation::NodeRuntimeAction;

    #[test]
    fn only_installed_bounded_profiles_are_eligible_for_autonomous_recovery() {
        assert_eq!(
            profile_for_action(&NodeRuntimeAction::ActivateWireguardMesh),
            Some("wireguard-mesh")
        );
        assert_eq!(
            profile_for_action(&NodeRuntimeAction::RestoreCriticalDns),
            None
        );
    }
}
