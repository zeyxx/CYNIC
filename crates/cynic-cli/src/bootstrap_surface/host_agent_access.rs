use std::process::ExitCode;

use clap::Args;
use cynic_host::adapters::agent_access::observe_linux_uid;
use cynic_host::adapters::local_command_runner::LocalHostCommandRunner;
use cynic_host::agent_access::{
    AgentAccessPolicy, AgentAuthorityBundleRequest, AgentAuthorityProjectionSpec,
    AgentEnrollmentPolicy, DEFAULT_AGENT_SESSION_TTL_MS, DEFAULT_PEER_OBSERVATION_MAX_AGE_MS,
    apply_agent_access, apply_agent_authority_projection, default_local_agent_capabilities,
    write_agent_authority_bundle,
};

use crate::{emit_json, fail};

#[derive(Debug, Args)]
pub(crate) struct AgentAccessArgs {
    #[arg(long = "principal", required = true)]
    principals: Vec<String>,
    #[arg(long)]
    apply: bool,
}

#[derive(Debug, Args)]
pub(crate) struct AgentAuthorityBundleArgs {
    #[arg(long)]
    node_id: String,
    #[arg(long = "principal", required = true)]
    principals: Vec<String>,
    #[arg(long = "agent-type", default_value = "codex")]
    agent_types: Vec<String>,
    #[arg(long)]
    output: String,
    #[arg(long, default_value_t = 1)]
    revision: u64,
    #[arg(long, default_value_t = DEFAULT_PEER_OBSERVATION_MAX_AGE_MS)]
    peer_observation_max_age_ms: u64,
    #[arg(long, default_value_t = DEFAULT_AGENT_SESSION_TTL_MS)]
    max_ttl_ms: u64,
}

#[derive(Debug, Args)]
pub(crate) struct AgentAuthorityArgs {
    #[arg(long)]
    source: String,
    #[arg(long)]
    apply: bool,
}

pub(crate) fn execute_agent_access(args: AgentAccessArgs) -> ExitCode {
    let policy = AgentAccessPolicy::desired(args.principals);
    let report = apply_agent_access(
        &policy,
        "local-cli-admin",
        args.apply,
        &LocalHostCommandRunner,
    );
    let should_fail = args.apply && report.status != "completed";
    emit_json(&report, should_fail)
}

pub(crate) fn execute_agent_authority_bundle(args: AgentAuthorityBundleArgs) -> ExitCode {
    let mut enrollments = Vec::with_capacity(args.principals.len());
    for principal in &args.principals {
        let observed_linux_uid = match observe_linux_uid(principal) {
            Ok(uid) => uid,
            Err(error) => return fail(error),
        };
        enrollments.push(AgentEnrollmentPolicy {
            observed_linux_uid,
            agent_types: args.agent_types.clone(),
            max_ttl_ms: args.max_ttl_ms,
            capabilities: default_local_agent_capabilities(),
        });
    }
    let request = AgentAuthorityBundleRequest {
        node_id: args.node_id,
        revision: args.revision,
        peer_observation_max_age_ms: args.peer_observation_max_age_ms,
        enrollments,
    };
    match write_agent_authority_bundle(&args.output, &request) {
        Ok(report) => emit_json(&report, false),
        Err(error) => fail(error),
    }
}

pub(crate) fn execute_agent_authority(args: AgentAuthorityArgs) -> ExitCode {
    let authority = match std::fs::read(&args.source)
        .map_err(|error| format!("authority source is unreadable: {error}"))
        .and_then(|bytes| {
            serde_json::from_slice::<cynic_protocol::AgentSessionAuthority>(&bytes)
                .map_err(|error| format!("authority source is invalid: {error}"))
        }) {
        Ok(authority) => authority,
        Err(error) => return fail(error),
    };
    let spec = AgentAuthorityProjectionSpec::installed(&args.source, authority);
    let report = apply_agent_authority_projection(
        &spec,
        "local-cli-admin",
        args.apply,
        &LocalHostCommandRunner,
    );
    let should_fail = args.apply && report.status != "completed";
    emit_json(&report, should_fail)
}
