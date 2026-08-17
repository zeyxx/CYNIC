//! Filesystem JSONL-backed Empirical Performance Store.
//!
//! Provides durable, human-readable, append-only telemetry persistence
//! with an in-memory cached index for sub-millisecond constraint solving.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use cynic_protocol::ContextTier;
use tokio::sync::RwLock;

use crate::domain::matrix_sweep::empirical_store::{
    ConstraintFilter, EmpiricalProfileEntry, ModelEmpiricalScorecard, OptimizationGoal,
    aggregate_model_scorecard, solve_optimal_configuration,
};
use crate::domain::matrix_sweep::HardwareBudget;
use crate::ports::empirical_store_port::{EmpiricalStoreError, EmpiricalStorePort};

/// Default location for the empirical performance ledger.
pub const DEFAULT_EMPIRICAL_LEDGER_PATH: &str = "/var/lib/cynic/evidence/inference_profiles.jsonl";

#[derive(Debug, Clone)]
pub struct FsEmpiricalStore {
    ledger_path: PathBuf,
    entries: Arc<RwLock<Vec<EmpiricalProfileEntry>>>,
}

impl FsEmpiricalStore {
    /// Open or initialize the empirical store at the specified ledger path.
    pub fn open(ledger_path: impl Into<PathBuf>) -> Result<Self, EmpiricalStoreError> {
        let path = ledger_path.into();
        let mut loaded = Vec::new();

        if path.exists() {
            let file = File::open(&path).map_err(|e| EmpiricalStoreError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            let reader = BufReader::new(file);

            for (line_no, line_res) in reader.lines().enumerate() {
                let line = line_res.map_err(|e| EmpiricalStoreError::Io {
                    path: path.display().to_string(),
                    source: e,
                })?;

                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                let entry: EmpiricalProfileEntry =
                    serde_json::from_str(trimmed).map_err(|e| EmpiricalStoreError::CorruptedRecord {
                        details: format!("Line {} in {}: {}", line_no + 1, path.display(), e),
                    })?;

                loaded.push(entry);
            }
        }

        Ok(Self {
            ledger_path: path,
            entries: Arc::new(RwLock::new(loaded)),
        })
    }

    /// Open with the default system evidence path or fallback to user configuration directory.
    pub fn open_default() -> Result<Self, EmpiricalStoreError> {
        let default_path = Path::new(DEFAULT_EMPIRICAL_LEDGER_PATH);
        if default_path.exists() || default_path.parent().map(|p| p.exists()).unwrap_or(false) {
            Self::open(default_path)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let user_fallback = PathBuf::from(home).join(".cynic/evidence/inference_profiles.jsonl");
            Self::open(user_fallback)
        }
    }

    /// Internal helper to append raw lines to the file.
    fn append_to_file(&self, serialized_lines: &[String]) -> Result<(), EmpiricalStoreError> {
        if let Some(parent) = self.ledger_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| EmpiricalStoreError::Io {
                    path: parent.display().to_string(),
                    source: e,
                })?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|e| EmpiricalStoreError::Io {
                path: self.ledger_path.display().to_string(),
                source: e,
            })?;

        for line in serialized_lines {
            writeln!(file, "{line}").map_err(|e| EmpiricalStoreError::Io {
                path: self.ledger_path.display().to_string(),
                source: e,
            })?;
        }

        file.flush().map_err(|e| EmpiricalStoreError::Io {
            path: self.ledger_path.display().to_string(),
            source: e,
        })?;

        Ok(())
    }
}

#[async_trait]
impl EmpiricalStorePort for FsEmpiricalStore {
    async fn record_entry(&self, entry: EmpiricalProfileEntry) -> Result<(), EmpiricalStoreError> {
        let json_line = serde_json::to_string(&entry)?;
        let mut guard = self.entries.write().await;
        self.append_to_file(&[json_line])?;
        guard.push(entry);
        Ok(())
    }

    async fn record_batch(&self, entries: Vec<EmpiricalProfileEntry>) -> Result<(), EmpiricalStoreError> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut lines = Vec::with_capacity(entries.len());
        for entry in &entries {
            lines.push(serde_json::to_string(entry)?);
        }

        let mut guard = self.entries.write().await;
        self.append_to_file(&lines)?;
        guard.extend(entries);
        Ok(())
    }

    async fn query_entries(
        &self,
        model_alias: Option<&str>,
        context_tier: Option<ContextTier>,
    ) -> Result<Vec<EmpiricalProfileEntry>, EmpiricalStoreError> {
        let guard = self.entries.read().await;
        let mut results = Vec::new();

        for entry in guard.iter() {
            if let Some(alias) = model_alias {
                if !entry.model_alias.eq_ignore_ascii_case(alias) {
                    continue;
                }
            }
            if let Some(ctx) = context_tier {
                if entry.config.context_tier != ctx {
                    continue;
                }
            }
            results.push(entry.clone());
        }

        Ok(results)
    }

    async fn resolve_optimal(
        &self,
        model_alias: &str,
        live_hardware: HardwareBudget,
        system_reserve_bytes: u64,
        constraints: &ConstraintFilter,
        goal: OptimizationGoal,
    ) -> Result<Option<EmpiricalProfileEntry>, EmpiricalStoreError> {
        let guard = self.entries.read().await;
        let optimal = solve_optimal_configuration(
            &guard,
            model_alias,
            live_hardware,
            system_reserve_bytes,
            constraints,
            goal,
        );
        Ok(optimal)
    }

    async fn get_scorecard(&self, model_alias: &str) -> Result<ModelEmpiricalScorecard, EmpiricalStoreError> {
        let guard = self.entries.read().await;
        let scorecard = aggregate_model_scorecard(&guard, model_alias);
        Ok(scorecard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{HarnessProtocol, KvQuantization, ParametricConfigPoint, RopeScaling, TopologyStrategy};
    use tempfile::NamedTempFile;
    use crate::domain::matrix_sweep::empirical_store::{BenchmarkProvenance, OomSafetyStatus};
    use crate::domain::matrix_sweep::types::StreamDynamicsReport;

    fn mock_entry(model: &str, ctx: ContextTier, tps: f32) -> EmpiricalProfileEntry {
        EmpiricalProfileEntry {
            model_alias: model.into(),
            model_sha256: Some("sha256_mock_123".into()),
            model_weight_bytes: 4 * 1024 * 1024 * 1024,
            config: ParametricConfigPoint {
                model_alias: model.into(),
                model_file: format!("{model}.gguf"),
                context_tier: ctx,
                kv_quant: KvQuantization::Q4_0,
                threads_gen: 8,
                threads_batch: 16,
                batch_size: 1024,
                ubatch_size: 512,
                flash_attn: true,
                gpu_layers: 24,
                rope_scaling: RopeScaling::None,
                topology: TopologyStrategy::DedicatedSingle,
                target_harness: HarnessProtocol::OpenAiToolCall,
            },
            stream_dynamics: StreamDynamicsReport {
                ttft_ms: 120.0,
                mean_inter_token_ms: 1000.0 / tps,
                jitter_variance_ms: 1.5,
                predicted_tokens_per_second: tps,
                prompt_tokens_per_second: 200.0,
            },
            rss_peak_bytes: 5 * 1024 * 1024 * 1024,
            roofline_inputs: None,
            quality_score: Some(0.98),
            oom_status: OomSafetyStatus::Safe,
            provenance: BenchmarkProvenance {
                timestamp_unix_s: 1786830000,
                total_ram_bytes: 32 * 1024 * 1024 * 1024,
                available_ram_bytes_at_start: 24 * 1024 * 1024 * 1024,
                cpu_arch: "x86_64".into(),
                os_kernel: "Linux".into(),
            },
        }
    }

    #[tokio::test]
    async fn test_fs_empirical_store_persistence_and_query() {
        let temp = NamedTempFile::new().unwrap();
        let store = FsEmpiricalStore::open(temp.path()).unwrap();

        let e1 = mock_entry("qwen-2b", ContextTier::Ctx4k, 28.0);
        let e2 = mock_entry("qwen-2b", ContextTier::Ctx64k, 19.5);
        let e3 = mock_entry("bonsai-27b", ContextTier::Ctx32k, 14.0);

        store.record_batch(vec![e1.clone(), e2.clone(), e3.clone()]).await.unwrap();

        // Query by model
        let qwen_entries = store.query_entries(Some("qwen-2b"), None).await.unwrap();
        assert_eq!(qwen_entries.len(), 2);

        // Re-open from disk and verify persistence
        let reopened = FsEmpiricalStore::open(temp.path()).unwrap();
        let all = reopened.query_entries(None, None).await.unwrap();
        assert_eq!(all.len(), 3);

        // Solve optimal on reopened store
        let hw = HardwareBudget {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 24 * 1024 * 1024 * 1024,
            gpu_vram_bytes: 8 * 1024 * 1024 * 1024,
        };

        let optimal = reopened.resolve_optimal(
            "qwen-2b",
            hw,
            2 * 1024 * 1024 * 1024,
            &ConstraintFilter {
                min_context: Some(ContextTier::Ctx64k),
                ..Default::default()
            },
            OptimizationGoal::MaxThroughput,
        ).await.unwrap();

        assert!(optimal.is_some());
        assert_eq!(optimal.unwrap().config.context_tier, ContextTier::Ctx64k);
    }
}
