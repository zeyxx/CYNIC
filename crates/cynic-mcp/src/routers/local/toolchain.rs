// ── MCP tool: cynic_toolchain_reconcile ─────────────────────────
// Projects the Rust toolchain lifecycle compass as a read-back proof surface.
// Read-only observation; mutation surfaces require CYNIC_MCP_TRANSPORT authority.
// Delegates to the cynic-host toolchain port (observe → reconcile → decision).

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct ToolchainReconcileParams {
    /// Target crate identifier (default: host = "cynic-host").
    crate_name: Option<String>,
    /// Operator consent to apply the bounded action (rustup override set).
    /// Without approve: dry-run projection only.
    approve: Option<bool>,
}

#[tool_router(router = tool_router_local_toolchain, vis = "pub(crate)")]
impl CynicMcpProxy {
    /// Project the Rust toolchain reconciliation state for the host crate.
    /// Observes the local toolchain (rustc version, ICE artifacts), reconciles
    /// against the seed registry in infra/rust-toolchain.json and proof history
    /// in target/proofs/toolchain, then returns the pure decision + planned
    /// bounded action. With approve=true, applies the bounded argv (rustup override
    /// set) and read back the proof.
    #[tool(
        name = "cynic_toolchain_reconcile",
        description = "Project Rust toolchain reconciliation (observe → reconcile → decision) for a crate. Optional approve applies the bounded action (rustup override set) with operator consent; without approve this is a dry-run projection."
    )]
    async fn toolchain_reconcile(
        &self,
        params: Parameters<ToolchainReconcileParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = self.require_auth()?;
        let _ = self.rate_limit.check_other()?;

        let p = params.0;
        let crate_name = p.crate_name.unwrap_or_else(|| "cynic-host".to_string());
        let approve = p.approve.unwrap_or(false);

        let seed_path = std::path::Path::new(&self.project_root).join("infra/rust-toolchain.json");
        let history_dir = std::path::Path::new(&self.project_root).join("target/proofs/toolchain");

        let outcome = run_toolchain_lifecycle(&crate_name, &seed_path, &history_dir, approve);
        let json = serde_json::to_string_pretty(&outcome)
            .unwrap_or_else(|_| "{\"status\":\"serialize_error\"}".to_string());
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

/// Reads back the toolchain lifecycle port as a projection surface.
/// With approve, applies the bounded argv (rustup override set); otherwise
/// returns the observe/reconcile decision only (dry-run).
fn run_toolchain_lifecycle(
    crate_name: &str,
    seed_path: &std::path::Path,
    history_dir: &std::path::Path,
    approve: bool,
) -> serde_json::Value {
    // 1. Load seed registry (declarative desired state).
    let seed = match std::fs::read_to_string(seed_path) {
        Ok(s) => serde_json::from_str::<cynic_host::toolchain::ToolchainRegistry>(&s)
            .unwrap_or_default(),
        Err(_) => cynic_host::toolchain::ToolchainRegistry::default(),
    };

    // 2. Observe substrate + history projection.
    let observer = cynic_host::toolchain::observe::LocalToolchainObserver::new(seed.clone());
    let observed = cynic_host::toolchain::ToolchainObservationPort::observe(
        &observer,
        crate_name,
        &cynic_host::update_lifecycle::LocalArgvRunner,
    );
    let history = cynic_host::toolchain::ToolchainProofHistory::load(history_dir)
        .unwrap_or_default();

    if !approve {
        return serde_json::json!({
            "schema": "cynic.toolchain_reconcile.v1",
            "status": "dry_run",
            "crate": crate_name,
            "seed_path": seed_path.to_string_lossy(),
            "seed_loaded": !seed.crates.is_empty(),
            "observed": observed,
            "last_ok": history.last_ok(crate_name).map(|s| s.to_string()),
            "mutation": false,
        });
    }

    // 3. With operator consent: apply bounded argv via the lifecycle port.
    let result = cynic_host::toolchain::ToolchainLifecycle::new(observer)
        .run(crate_name, seed_path, history_dir, &cynic_host::update_lifecycle::LocalArgvRunner);

    match result {
        Ok(report) => serde_json::json!({
            "schema": "cynic.toolchain_reconcile.v1",
            "status": "applied",
            "crate": crate_name,
            "observed": report.observed,
            "decision": report.decision,
            "applied": report.applied,
            "proof": report.proof,
            "residual_risk": report.residual_risk,
            "mutation": true,
        }),
        Err(e) => serde_json::json!({
            "schema": "cynic.toolchain_reconcile.v1",
            "status": "error",
            "crate": crate_name,
            "error": e,
            "mutation": false,
        }),
    }
}
