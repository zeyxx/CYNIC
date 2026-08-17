//! Data-Centric Inference Compliance Audit Surface (`cynic dev data-centric-audit`).
//!
//! Evaluates whether runtime execution configurations are backed by empirical
//! telemetry proofs or unverified manual guesswork.

use std::path::PathBuf;
use std::process::ExitCode;

use cynic_inference::{EmpiricalStorePort, FsEmpiricalStore};
use serde::Serialize;
pub(super) const DATA_CENTRIC_AUDIT_SCHEMA: &str = "cynic.dev.data_centric_audit.v1";

#[derive(Debug, Serialize)]
pub(super) struct DataCentricAuditReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub ledger_path: String,
    pub ledger_present: bool,
    pub total_recorded_profiles: usize,
    pub vault_models_total: usize,
    pub vault_models_benchmarked: usize,
    pub coverage_percent: u32,
    pub benchmarked_scorecards: Vec<cynic_inference::ModelEmpiricalScorecard>,
    pub unbenchmarked_models: Vec<String>,
    pub next_action: Option<String>,
}

pub(super) fn run(json: bool, ledger: Option<PathBuf>, vault_dir: Option<PathBuf>) -> ExitCode {
    if !json {
        eprintln!("data-centric-audit requires --json");
        return ExitCode::FAILURE;
    }

    let store = match ledger {
        Some(p) => FsEmpiricalStore::open(p),
        None => FsEmpiricalStore::open_default(),
    };

    let (store_ok, store_inst) = match store {
        Ok(s) => (true, Some(s)),
        Err(_) => (false, None),
    };

    let vault = vault_dir.unwrap_or_else(|| PathBuf::from("/var/lib/cynic/vault/models"));
    let mut vault_models = Vec::new();

    if vault.exists() {
        if let Ok(entries) = std::fs::read_dir(&vault) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        vault_models.push(stem.to_string());
                    }
                }
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let (scorecards, unbenchmarked, total_profiles) = if let Some(ref s) = store_inst {
        rt.block_on(async {
            let mut sc_list = Vec::new();
            let mut unbench = Vec::new();
            let all_entries = s.query_entries(None, None).await.unwrap_or_default();
            let total_count = all_entries.len();

            for m in &vault_models {
                let sc = s.get_scorecard(m).await.unwrap_or_else(|_| cynic_inference::ModelEmpiricalScorecard {
                    model_alias: m.clone(),
                    total_benchmarked_points: 0,
                    safe_points_count: 0,
                    max_viable_context: cynic_protocol::ContextTier::Ctx4k,
                    peak_throughput_tok_sec: 0.0,
                    min_ttft_ms: 0.0,
                    best_balanced_config: None,
                });

                if sc.total_benchmarked_points > 0 {
                    sc_list.push(sc);
                } else {
                    unbench.push(m.clone());
                }
            }

            (sc_list, unbench, total_count)
        })
    } else {
        (Vec::new(), vault_models.clone(), 0)
    };

    let benchmarked_count = scorecards.len();
    let vault_total = vault_models.len();
    let coverage = if vault_total > 0 {
        (benchmarked_count * 100 / vault_total) as u32
    } else {
        100
    };

    let (status, next_action) = if !unbenchmarked.is_empty() {
        let first = &unbenchmarked[0];
        (
            "action_required",
            Some(format!(
                "cynic dev inference-matrix --model /var/lib/cynic/vault/models/{first}.gguf --execute --sample 5"
            )),
        )
    } else {
        ("ok", None)
    };

    let report = DataCentricAuditReport {
        schema: DATA_CENTRIC_AUDIT_SCHEMA,
        status,
        ledger_path: store_inst
            .map(|_| cynic_inference::DEFAULT_EMPIRICAL_LEDGER_PATH.to_string())
            .unwrap_or_else(|| "unobservable".into()),
        ledger_present: store_ok,
        total_recorded_profiles: total_profiles,
        vault_models_total: vault_total,
        vault_models_benchmarked: benchmarked_count,
        coverage_percent: coverage,
        benchmarked_scorecards: scorecards,
        unbenchmarked_models: unbenchmarked,
        next_action,
    };

    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());

    if status == "ok" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
