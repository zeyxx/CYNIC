use std::process::ExitCode;

use cynic_inference::{
    HardwareProfile, HfDiscoveryService, HfDownloadCommand, ModelStoreService, category_catalog,
};

use super::emit_json;

pub(crate) struct HfDownloadArgs<'a> {
    pub(crate) revision: Option<&'a str>,
    pub(crate) include: &'a [String],
    pub(crate) exclude: &'a [String],
    pub(crate) repo_type: Option<&'a str>,
    pub(crate) cache_only: bool,
    pub(crate) alias: Option<&'a str>,
    pub(crate) register_profile: bool,
}

pub(crate) fn execute_inference_hf_status(json: bool) -> ExitCode {
    let service = ModelStoreService::from_env();
    match service.hf_status() {
        Ok(status) => {
            if json {
                emit_json(&status)
            } else {
                println!("HF Hub CLI status:");
                println!("  Bin path:       {}", status.hf_bin);
                println!("  Available:      {}", status.available);
                println!("  Authenticated:  {}", status.authenticated);
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("hf status error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_download(
    repo: &str,
    json: bool,
    args: &HfDownloadArgs<'_>,
) -> ExitCode {
    let service = HfDiscoveryService::from_env();
    match service.download_and_register(
        repo,
        HfDownloadCommand::new(
            args.revision,
            args.include,
            args.exclude,
            args.repo_type,
            args.cache_only,
            args.alias,
            args.register_profile,
        ),
    ) {
        Ok(report) => {
            if report.ok {
                let _ = ModelStoreService::from_env().scan_and_persist();
            }
            let code = if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
            if json {
                let _ = emit_json(&report);
            } else {
                println!("HF Download Report: ok={}", report.ok);
                if let Some(msg) = &report.message {
                    println!("  Message: {}", msg);
                }
            }
            code
        }
        Err(error) => {
            eprintln!("hf download error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_categories(json: bool) -> ExitCode {
    let body = serde_json::json!({
        "schema": "cynic.inference.hf_categories.v1",
        "surface": "hf_cli",
        "categories": category_catalog(),
    });
    if json {
        emit_json(&body)
    } else {
        println!("Available HF Categories: {:?}", category_catalog());
        ExitCode::SUCCESS
    }
}

pub(crate) fn execute_inference_hardware(json: bool) -> ExitCode {
    let probe = HardwareProfile::probe();
    if json {
        emit_json(&probe)
    } else {
        println!("Hardware Profile Probe:");
        println!("  CPU Threads:      {}", probe.cpu_threads);
        println!(
            "  Total RAM:        {} GB",
            probe.total_ram_bytes / (1024 * 1024 * 1024)
        );
        println!("  GPU Class:        {:?}", probe.gpu.class);
        println!("  GPU Backends:     {:?}", probe.backends);
        ExitCode::SUCCESS
    }
}

pub(crate) fn execute_inference_hf_search(
    json: bool,
    category: Option<&str>,
    query: Option<&str>,
    author: Option<&str>,
    limit: u32,
) -> ExitCode {
    let service = HfDiscoveryService::from_env();
    match service.search(category, query, limit, author) {
        Ok(report) => {
            if json {
                emit_json(&report)
            } else {
                println!("HF Search Report (found {} hits):", report.hits.len());
                for h in &report.hits {
                    println!("  - Repo:  {}", h.repo_id);
                    println!("    Likes: {}", h.likes.unwrap_or(0));
                }
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("hf search error: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_inference_hf_suggest(json: bool, category: &str, limit: u32) -> ExitCode {
    let service = HfDiscoveryService::from_env();
    match service.suggest(Some(category), limit) {
        Ok(report) => {
            if json {
                emit_json(&report)
            } else {
                println!("HF Suggestions (found {} suggestions):", report.hits.len());
                for h in &report.hits {
                    println!("  - Repo:  {}", h.repo_id);
                    println!("    Likes: {}", h.likes.unwrap_or(0));
                }
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("hf suggest error: {error}");
            ExitCode::FAILURE
        }
    }
}
