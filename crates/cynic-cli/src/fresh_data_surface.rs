//! Fresh-data CLI surface — probe providers, models, quota, and freshness.
//!
//! Exposes the `cynic-protocol::fresh_data` types via `cynic fresh data --json`.
//!
//! Subcommands:
//!   data — Probe all active providers and emit fresh-data snapshot

use std::fs;
use std::process::ExitCode;
use std::time::Duration;

use clap::Subcommand;

use cynic_host::adapters::fresh_data_probe_adapter::HermesProbeAdapter;
use cynic_host::adapters::fresh_data_metadata_adapter::MetadataAdapter;

const CACHE_PATH: &str = "/home/user/.hermes/probe-cache.json";
const CACHE_TTL_SECS: u64 = 300; // 5 minutes

#[derive(Subcommand)]
pub(crate) enum FreshDataCommand {
    /// Probe all active providers and emit fresh-data snapshot (JSON).
    Data {
        /// Machine-readable JSON (default is human text).
        #[arg(long)]
        json: bool,
        /// Force re-probe even if cache is fresh.
        #[arg(long)]
        force: bool,
        /// Quick mode: probe only active providers (skip no-credit).
        #[arg(long)]
        quick: bool,
    },
}

fn cache_is_fresh() -> bool {
    if let Ok(metadata) = fs::metadata(CACHE_PATH) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed < Duration::from_secs(CACHE_TTL_SECS);
            }
        }
    }
    false
}

pub(crate) fn execute_fresh_data(command: FreshDataCommand) -> ExitCode {
    match command {
        FreshDataCommand::Data { json, force, quick: _ } => {
            // Check cache first
            if !force && cache_is_fresh() {
                if let Ok(content) = fs::read_to_string(CACHE_PATH) {
                    if json {
                        print!("{}", content);
                    } else {
                        println!("Using cached probe results (fresh < 5 min).");
                    }
                    return ExitCode::SUCCESS;
                }
            }

            // Load metadata
            let meta_adapter = MetadataAdapter::default();
            let metadata = meta_adapter.load().unwrap_or_default();

            // Probe providers (native Rust implementation)
            let probe_adapter = HermesProbeAdapter::default();
            let providers = vec!["antigravity", "nous", "opencode-go"];
            let mut probes = Vec::new();

            for provider in providers {
                let outcome = probe_adapter.probe(provider, None);
                probes.push(serde_json::json!({
                    "provider": outcome.provider,
                    "model": outcome.model,
                    "latency_ms": outcome.latency_ms,
                    "success": outcome.success,
                    "error": outcome.error,
                }));
            }

            let snapshot = serde_json::json!({
                "schema": "cynic.fresh_data.v1",
                "timestamp": "native",
                "version": "0.8.0",
                "metadata_count": metadata.len(),
                "probes": probes,
            });

            let json_str = serde_json::to_string_pretty(&snapshot).unwrap_or_default();

            // Save to cache
            let _ = fs::write(CACHE_PATH, &json_str);

            if json {
                println!("{}", json_str);
            } else {
                println!("Fresh-data probe complete. {} providers probed.", probes.len());
            }

            ExitCode::SUCCESS
        }
    }
}