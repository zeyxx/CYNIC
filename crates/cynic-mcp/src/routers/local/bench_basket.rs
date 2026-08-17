// ── MCP tool: cynic_bench_basket_launch ─────────────────────────
// Projects the consolidated empirical benchmark matrix → model launch basket.
//
// Reads the empirical ledger (FsEmpiricalStore), applies Pareto-optimal
// selection across all models, and launches the resulting basket through
// the injected LocalInferenceRuntimePort — no hardcoded substrate.
//
// Implements the user workflow:
//   "un basket de modèles selon les résultats émergents des matrices
//    de benchmarks qu'on consolide"

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct BenchBasketParams {
    /// Optimization goal: max_throughput | min_latency | min_jitter |
    /// max_context | balanced (default).
    #[serde(default = "default_goal")]
    goal: String,
    /// Optional max TTFT in ms (SLA floor filter).
    max_ttft_ms: Option<f32>,
    /// Optional min generation tokens/sec (performance floor filter).
    min_gen_tokens_per_sec: Option<f32>,
    /// Optional min context tier (e.g. "4k", "32k").
    min_context: Option<String>,
    /// Optional max RAM bytes per model.
    max_ram_bytes: Option<u64>,
    /// Require Flash Attention enabled.
    require_flash_attn: Option<bool>,
    /// Require GPU offload (>0 layers).
    require_gpu_offload: Option<bool>,
    /// Launch the selected basket (requires operator consent).
    /// Without approve: dry-run projection only.
    #[serde(default)]
    approve: bool,
}

fn default_goal() -> String {
    "balanced".to_string()
}

fn parse_goal(s: &str) -> cynic_inference::OptimizationGoal {
    match s.to_lowercase().as_str() {
        "max_throughput" => cynic_inference::OptimizationGoal::MaxThroughput,
        "min_latency"    => cynic_inference::OptimizationGoal::MinLatency,
        "min_jitter"     => cynic_inference::OptimizationGoal::MinJitter,
        "max_context"    => cynic_inference::OptimizationGoal::MaxContext,
        _                => cynic_inference::OptimizationGoal::Balanced,
    }
}

fn parse_context_tier(s: &str) -> Option<cynic_protocol::ContextTier> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "4k"   => Some(cynic_protocol::ContextTier::Ctx4k),
        "8k"   => Some(cynic_protocol::ContextTier::Ctx8k),
        "16k"  => Some(cynic_protocol::ContextTier::Ctx16k),
        "32k"  => Some(cynic_protocol::ContextTier::Ctx32k),
        "64k"  => Some(cynic_protocol::ContextTier::Ctx64k),
        "128k" => Some(cynic_protocol::ContextTier::Ctx128k),
        _      => None,
    }
}

#[tool_router(router = tool_router_local_bench_basket, vis = "pub(crate)")]
impl CynicMcpProxy {
    /// Build a model launch basket from the consolidated benchmark matrix
    /// (empirical ledger). With approve=false (default): dry-run projection
    /// of the Pareto-optimal basket. With approve=true: launches each selected
    /// config through the local inference runtime (requires kernel transport
    /// authority).
    #[tool(
        name = "cynic_bench_basket_launch",
        description = "Build a model launch basket from the consolidated empirical benchmark matrix (Pareto-optimal configurations) and optionally launch it. Dry-run by default; set approve=true with operator consent to launch models through the local inference runtime."
    )]
    async fn bench_basket_launch(
        &self,
        params: Parameters<BenchBasketParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = self.require_auth()?;
        let _ = self.rate_limit.check_other()?;

        let p = params.0;

        // Build constraint filter from optional params
        let constraints = cynic_inference::ConstraintFilter {
            max_ttft_ms: p.max_ttft_ms,
            min_gen_tokens_per_sec: p.min_gen_tokens_per_sec,
            min_context: p.min_context.as_deref().and_then(parse_context_tier),
            max_ram_bytes: p.max_ram_bytes,
            require_flash_attn: p.require_flash_attn.unwrap_or(false),
            require_gpu_offload: p.require_gpu_offload.unwrap_or(false),
        };

        let goal = parse_goal(&p.goal);
        let hardware = cynic_inference::domain::matrix_sweep::HardwareBudget::from_env();

        // Build basket plan from the empirical ledger
        let fs_store = match cynic_inference::adapters::empirical_store::FsEmpiricalStore::open_default() {
            Ok(store) => store,
            Err(e) => {
                return Err(rmcp::model::ErrorData::new(
                    rmcp::model::ErrorCode(-32603),
                    format!("failed to open empirical store: {e}"),
                    None,
                ));
            }
        };

        let basket_plan = cynic_inference::build_basket_from_ledger(
            &fs_store,
            hardware,
            hardware.total_ram_bytes / 10,
            &constraints,
            goal,
        )
        .await;

        match basket_plan {
            Ok(plan) => {
                if !p.approve {
                    let result = serde_json::json!({
                        "schema": "cynic.bench_basket_launch.v1",
                        "status": "dry_run",
                        "mutation": false,
                        "goal": p.goal,
                        "constraints": constraints,
                        "hardware": plan.hardware,
                        "candidates_considered": plan.candidates_considered,
                        "dominated_excluded": plan.dominated_excluded,
                        "basket": plan.items,
                        "item_count": plan.items.len(),
                        "message": format!("basket of {} model configs ready; pass approve=true to launch", plan.items.len()),
                    });
                    let json = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "{\"status\":\"serialize_error\"}".to_string());
                    return Ok(CallToolResult::success(vec![Content::text(json)]));
                }

                // Launch the basket through the local runtime port
                let runtime = cynic_inference::LocalRuntimeServiceAny::from_env();
                let launch_result = cynic_inference::launch_basket(&runtime, plan.clone()).await;

                match launch_result {
                    Ok(report) => {
                        let result = serde_json::json!({
                            "schema": "cynic.bench_basket_launch.v1",
                            "status": "launched",
                            "mutation": true,
                            "goal": p.goal,
                            "plan": plan,
                            "launched": report.launched,
                            "failed": report.failed,
                            "per_item_results": report.per_item_results,
                            "message": report.message,
                        });
                        let json = serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| "{\"status\":\"serialize_error\"}".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    }
                    Err(e) => {
                        let result = serde_json::json!({
                            "schema": "cynic.bench_basket_launch.v1",
                            "status": "error",
                            "mutation": false,
                            "error": format!("launch failed: {e}"),
                            "plan": plan,
                        });
                        let json = serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| "{\"status\":\"serialize_error\"}".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    }
                }
            }
            Err(e) => {
                let result = serde_json::json!({
                    "schema": "cynic.bench_basket_launch.v1",
                    "status": "error",
                    "mutation": false,
                    "error": format!("basket build failed: {e}"),
                });
                let json = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|_| "{\"status\":\"serialize_error\"}".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_goal_is_balanced() {
        assert_eq!(default_goal(), "balanced");
    }

    #[test]
    fn parse_goal_handles_all_variants() {
        assert_eq!(parse_goal("max_throughput"), cynic_inference::OptimizationGoal::MaxThroughput);
        assert_eq!(parse_goal("min_latency"), cynic_inference::OptimizationGoal::MinLatency);
        assert_eq!(parse_goal("min_jitter"), cynic_inference::OptimizationGoal::MinJitter);
        assert_eq!(parse_goal("max_context"), cynic_inference::OptimizationGoal::MaxContext);
        assert_eq!(parse_goal("balanced"), cynic_inference::OptimizationGoal::Balanced);
        assert_eq!(parse_goal("unknown"), cynic_inference::OptimizationGoal::Balanced);
    }

    #[test]
    fn parse_goal_is_case_insensitive() {
        assert_eq!(parse_goal("MAX_THROUGHPUT"), cynic_inference::OptimizationGoal::MaxThroughput);
        assert_eq!(parse_goal("Min_Latency"), cynic_inference::OptimizationGoal::MinLatency);
    }

    #[test]
    fn parse_context_tier_handles_all_sizes() {
        assert_eq!(parse_context_tier("4k"), Some(cynic_protocol::ContextTier::Ctx4k));
        assert_eq!(parse_context_tier("32k"), Some(cynic_protocol::ContextTier::Ctx32k));
        assert_eq!(parse_context_tier("128k"), Some(cynic_protocol::ContextTier::Ctx128k));
        assert_eq!(parse_context_tier("invalid"), None);
        assert_eq!(parse_context_tier("64K"), Some(cynic_protocol::ContextTier::Ctx64k));
    }
}
