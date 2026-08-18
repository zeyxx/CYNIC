// ── Build runner tools (distributed build trigger) ──────────

use crate::types::*;
use crate::types_split::core_params::BuildTriggerParams;

use cynic_node::ports::build_runner::{
    BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode,
};
use cynic_node::ports::build_runner_adapter::{LocalCommandRunner, SshBuildRunner};

#[tool_router(router = tool_router_build, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_build_trigger",
        description = "Trigger a distributed build on the network runner (10.10.10.119) for an exact source commit and target phase. Returns the signed pipeline_build_proof bundle. No local cargo, no stack hacks — the runner owns toolchain dispatch."
    )]
    async fn cynic_build_trigger(
        &self,
        params: Parameters<BuildTriggerParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        let phase = match p.phase.as_deref() {
            Some("build") => BuildPhase::Build,
            Some("activate") => BuildPhase::Activate,
            Some(other) if other != "check" => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": format!("invalid phase '{other}' (expected check|build|activate)")
                    }))
                    .unwrap_or_default(),
                )]));
            }
            _ => BuildPhase::Check,
        };
        let network_mode = match p.network_mode.as_deref() {
            Some("online") => NetworkMode::Online,
            _ => NetworkMode::Offline,
        };
        let host = p.host.unwrap_or_else(|| "10.10.10.119".to_string());
        let user = p.user.unwrap_or_else(|| "cynic".to_string());

        let trigger = BuildTrigger {
            source_commit: p.commit,
            phase,
            network_mode,
        };
        let runner = SshBuildRunner {
            host,
            user,
            runner: Box::new(LocalCommandRunner),
        };

        match runner.trigger(&trigger) {
            Ok(proof) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&proof).unwrap_or_default(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "error": format!("{e}")
                }))
                .unwrap_or_default(),
            )])),
        }
    }
}
