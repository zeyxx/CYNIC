//! Fleet epoch consensus evaluation.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

use clap::Subcommand;

pub(crate) fn execute_consensus_status(
    expected_epoch: u64,
    expected_commit: &str,
    max_skew_ms: u64,
) -> ExitCode {
    let registered_nodes = vec![
        "cynic-host-cortex".to_string(),
        "cynic-builder".to_string(),
        "cynic-host-proxmox".to_string(),
    ];
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let local_attestation = cynic_consensus::NodeEpochAttestation {
        schema: cynic_consensus::FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
        node_id: "cynic-host-cortex".to_string(),
        epoch_sequence: expected_epoch,
        commit_sha: expected_commit.to_string(),
        binary_sha256: "local-active".to_string(),
        registry_digest: "registry-v1".to_string(),
        monotonic_tick: 1,
        signature_hex: "local-sig".to_string(),
        attested_at_unix_ms: now_unix_ms,
    };

    let consensus = cynic_consensus::evaluate_fleet_consensus(
        expected_epoch,
        expected_commit,
        &registered_nodes,
        &[local_attestation],
        max_skew_ms,
        now_unix_ms,
    );

    println!(
        "{}",
        serde_json::to_string_pretty(&consensus).unwrap_or_default()
    );
    ExitCode::SUCCESS
}

/// Fleet consensus arms: evaluate epoch consensus and clock skew.
#[derive(Subcommand)]
pub enum NodeConsensusCommand {
    /// Evaluate fleet epoch consensus and clock skew across cluster nodes.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 1)]
        expected_epoch: u64,
        #[arg(long, default_value = "HEAD")]
        expected_commit: String,
        #[arg(long, default_value_t = 5000)]
        max_skew_ms: u64,
    },
}

pub(crate) fn execute_node_consensus(command: NodeConsensusCommand) -> ExitCode {
    match command {
        NodeConsensusCommand::Status {
            json: _,
            expected_epoch,
            expected_commit,
            max_skew_ms,
        } => execute_consensus_status(expected_epoch, &expected_commit, max_skew_ms),
    }
}
