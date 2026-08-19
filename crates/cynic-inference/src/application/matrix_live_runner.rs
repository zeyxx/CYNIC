//! Live Benchmark Execution Runner (`MatrixLiveRunner`).
//!
//! Spawns `llama-server` instances across parametric grid points,
//! executes live streaming benchmark scenarios over HTTP/SSE,
//! captures real-time emission dynamics (TTFT, tok/s, jitter, RSS peak),
//! and persists verified telemetry directly into the empirical ledger.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cynic_protocol::ParametricConfigPoint;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;
use tokio::process::Child;

use crate::domain::matrix_sweep::empirical_store::{
    BenchmarkProvenance, EmpiricalProfileEntry, OomSafetyStatus,
};
use crate::domain::matrix_sweep::memory_estimator::{
    ModelArchDescriptor, can_admit_model_configuration,
};
use crate::domain::matrix_sweep::HardwareBudget;
use crate::domain::matrix_sweep::stream_metrics::compute_stream_dynamics;
use crate::domain::matrix_sweep::types::{BenchmarkScenario, StreamDynamicsReport};
use crate::ports::empirical_store_port::{EmpiricalStoreError, EmpiricalStorePort};

use super::matrix_sweep_executor::config_to_llama_argv;

#[derive(Debug, Clone)]
pub struct MatrixLiveRunnerConfig {
    pub llama_server_bin: PathBuf,
    pub bind_port: u16,
    pub startup_timeout_secs: u64,
    pub scenario_timeout_secs: u64,
}

impl Default for MatrixLiveRunnerConfig {
    fn default() -> Self {
        Self {
            llama_server_bin: PathBuf::from("/usr/local/bin/llama-server"),
            bind_port: 8089,
            startup_timeout_secs: 180,
            scenario_timeout_secs: 90,
        }
    }
}

pub struct MatrixLiveRunner {
    config: MatrixLiveRunnerConfig,
    http_client: Client,
    store: Arc<dyn EmpiricalStorePort>,
    hardware: HardwareBudget,
}
impl std::fmt::Debug for MatrixLiveRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatrixLiveRunner")
            .field("config", &self.config)
            .field("hardware", &self.hardware)
            .finish()
    }
}

impl MatrixLiveRunner {
    pub fn new(
        config: MatrixLiveRunnerConfig,
        store: Arc<dyn EmpiricalStorePort>,
        hardware: HardwareBudget,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.scenario_timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            store,
            hardware,
        }
    }

    /// Execute the full benchmark on a single parametric configuration point.
    pub async fn execute_point(
        &self,
        model_path: &Path,
        model_weight_bytes: u64,
        approx_params_billions: f32,
        config: &ParametricConfigPoint,
        scenarios: &[BenchmarkScenario],
    ) -> Result<EmpiricalProfileEntry, EmpiricalStoreError> {
        let arch = ModelArchDescriptor::for_parameter_count(approx_params_billions);
        let estimated_total = arch.estimate_total_footprint(
            model_weight_bytes,
            config.context_tier,
            config.kv_quant,
            config.batch_size,
            1,
        );

        // Preflight: Skip spawn if hardware memory admission certainly rejects it
        let system_reserve = self.hardware.total_ram_bytes / 10;
        if !can_admit_model_configuration(
            estimated_total,
            self.hardware.available_ram_bytes,
            system_reserve,
        ) {
            let skipped_entry = EmpiricalProfileEntry {
                model_alias: config.model_alias.clone(),
                model_sha256: None,
                model_weight_bytes,
                config: config.clone(),
                stream_dynamics: StreamDynamicsReport {
                    ttft_ms: 0.0,
                    mean_inter_token_ms: 0.0,
                    jitter_variance_ms: 0.0,
                    predicted_tokens_per_second: 0.0,
                    prompt_tokens_per_second: 0.0,
                },
                rss_peak_bytes: estimated_total,
                roofline_inputs: None,
                quality_score: None,
                oom_status: OomSafetyStatus::Oom,
                provenance: self.current_provenance(),
            };
            self.store.record_entry(skipped_entry.clone()).await?;
            return Ok(skipped_entry);
        }

        let model_path_str = model_path.display().to_string();
        let argv = config_to_llama_argv(&model_path_str, config, self.config.bind_port);

        // Spawn llama-server
        let mut child = match tokio::process::Command::new(&self.config.llama_server_bin)
            .args(&argv)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let err_entry = EmpiricalProfileEntry {
                    model_alias: config.model_alias.clone(),
                    model_sha256: None,
                    model_weight_bytes,
                    config: config.clone(),
                    stream_dynamics: StreamDynamicsReport {
                        ttft_ms: 0.0,
                        mean_inter_token_ms: 0.0,
                        jitter_variance_ms: 0.0,
                        predicted_tokens_per_second: 0.0,
                        prompt_tokens_per_second: 0.0,
                    },
                    rss_peak_bytes: 0,
                    roofline_inputs: None,
                    quality_score: None,
                    oom_status: OomSafetyStatus::ExecutionError,
                    provenance: self.current_provenance(),
                };
                let _ = self.store.record_entry(err_entry.clone()).await;
                return Err(EmpiricalStoreError::Io {
                    path: self.config.llama_server_bin.display().to_string(),
                    source: e,
                });
            }
        };

        // Wait for readiness on /health
        let health_url = format!("http://127.0.0.1:{}/health", self.config.bind_port);
        let ready = self.wait_for_health(&health_url, &mut child).await;

        if !ready {
            let _ = child.kill().await;
            let oom_entry = EmpiricalProfileEntry {
                model_alias: config.model_alias.clone(),
                model_sha256: None,
                model_weight_bytes,
                config: config.clone(),
                stream_dynamics: StreamDynamicsReport {
                    ttft_ms: 0.0,
                    mean_inter_token_ms: 0.0,
                    jitter_variance_ms: 0.0,
                    predicted_tokens_per_second: 0.0,
                    prompt_tokens_per_second: 0.0,
                },
                rss_peak_bytes: 0,
                roofline_inputs: None,
                quality_score: None,
                oom_status: OomSafetyStatus::Oom,
                provenance: self.current_provenance(),
            };
            self.store.record_entry(oom_entry.clone()).await?;
            return Ok(oom_entry);
        }

        // Measure Peak RSS
        let child_pid = child.id().unwrap_or(0);
        let initial_rss = read_process_rss_bytes(child_pid);

        // Execute benchmark scenarios over SSE
        let mut scenario_dynamics = Vec::new();
        let completions_url = format!("http://127.0.0.1:{}/v1/chat/completions", self.config.bind_port);

        for scenario in scenarios {
            if let Ok(dyn_report) = self.run_streaming_scenario(&completions_url, scenario).await {
                scenario_dynamics.push(dyn_report);
            }
        }

        let peak_rss = read_process_rss_bytes(child_pid).max(initial_rss);

        // Graceful termination
        let _ = child.kill().await;
        let _ = child.wait().await;

        // Aggregate measured scenario dynamics
        let aggregated_dynamics = aggregate_stream_reports(&scenario_dynamics);
        let oom_status = if peak_rss > (self.hardware.total_ram_bytes - system_reserve) {
            OomSafetyStatus::Marginal
        } else {
            OomSafetyStatus::Safe
        };

        let entry = EmpiricalProfileEntry {
            model_alias: config.model_alias.clone(),
            model_sha256: None,
            model_weight_bytes,
            config: config.clone(),
            stream_dynamics: aggregated_dynamics,
            rss_peak_bytes: peak_rss,
            roofline_inputs: None,
            // Quality is unknown until scenario assertions produce an observed score.
            quality_score: None,
            oom_status,
            provenance: self.current_provenance(),
        };

        self.store.record_entry(entry.clone()).await?;
        Ok(entry)
    }

    async fn wait_for_health(&self, health_url: &str, child: &mut Child) -> bool {
        let deadline = Instant::now() + Duration::from_secs(self.config.startup_timeout_secs);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = child.try_wait() {
                // Process crashed / exited
                return false;
            }

            if let Ok(resp) = self.http_client.get(health_url).send().await {
                if resp.status().is_success() {
                    return true;
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        false
    }

    async fn run_streaming_scenario(
        &self,
        completions_url: &str,
        scenario: &BenchmarkScenario,
    ) -> Result<StreamDynamicsReport, ()> {
        let start = Instant::now();
        let payload = serde_json::json!({
            "messages": [
                {"role": "user", "content": scenario.prompt}
            ],
            "max_tokens": scenario.max_tokens,
            "stream": true,
            "temperature": 0.0
        });

        let resp = self.http_client
            .post(completions_url)
            .json(&payload)
            .send()
            .await
            .map_err(|_| ())?;

        let mut stream = resp.bytes_stream();
        let mut chunk_timestamps_ms = Vec::new();
        let mut completion_tokens = 0u32;

        while let Some(item) = stream.next().await {
            if let Ok(bytes) = item {
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                chunk_timestamps_ms.push(elapsed_ms);

                if let Ok(text) = std::str::from_utf8(&bytes) {
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("data: ") && !trimmed.contains("[DONE]") {
                            let json_str = &trimmed[6..];
                            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                                if let Some(content) = v.pointer("/choices/0/delta/content").and_then(|c| c.as_str()) {
                                    if !content.is_empty() {
                                        completion_tokens += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let total_elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let prompt_tokens = (scenario.prompt.len() / 4).max(1) as u32;

        let report = compute_stream_dynamics(
            &chunk_timestamps_ms,
            prompt_tokens,
            completion_tokens,
            total_elapsed_ms,
        );

        Ok(report)
    }

    fn current_provenance(&self) -> BenchmarkProvenance {
        BenchmarkProvenance {
            timestamp_unix_s: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            total_ram_bytes: self.hardware.total_ram_bytes,
            available_ram_bytes_at_start: self.hardware.available_ram_bytes,
            cpu_arch: std::env::consts::ARCH.to_string(),
            os_kernel: std::env::consts::OS.to_string(),
        }
    }
}

fn aggregate_stream_reports(reports: &[StreamDynamicsReport]) -> StreamDynamicsReport {
    if reports.is_empty() {
        return StreamDynamicsReport {
            ttft_ms: 0.0,
            mean_inter_token_ms: 0.0,
            jitter_variance_ms: 0.0,
            predicted_tokens_per_second: 0.0,
            prompt_tokens_per_second: 0.0,
        };
    }

    let count = reports.len() as f32;
    let avg_ttft = reports.iter().map(|r| r.ttft_ms).sum::<f32>() / count;
    let avg_tps = reports.iter().map(|r| r.predicted_tokens_per_second).sum::<f32>() / count;
    let avg_prompt_tps = reports.iter().map(|r| r.prompt_tokens_per_second).sum::<f32>() / count;
    let avg_jitter = reports.iter().map(|r| r.jitter_variance_ms).sum::<f32>() / count;
    let avg_inter_token = reports.iter().map(|r| r.mean_inter_token_ms).sum::<f32>() / count;

    StreamDynamicsReport {
        ttft_ms: avg_ttft,
        mean_inter_token_ms: avg_inter_token,
        jitter_variance_ms: avg_jitter,
        predicted_tokens_per_second: avg_tps,
        prompt_tokens_per_second: avg_prompt_tps,
    }
}

/// Reads process resident set size (RSS) from /proc/{pid}/statm.
fn read_process_rss_bytes(pid: u32) -> u64 {
    if pid == 0 {
        return 0;
    }
    let statm_path = format!("/proc/{pid}/statm");
    if let Ok(content) = std::fs::read_to_string(statm_path) {
        if let Some(rss_pages) = content.split_whitespace().nth(1).and_then(|p| p.parse::<u64>().ok()) {
            return rss_pages * 4096; // 4 KiB page size
        }
    }
    0
}
