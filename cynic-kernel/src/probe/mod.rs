//! Ring 0 — CYNIC Onboarding Probe
//! Runs once at first boot, writes ~/.cynic/node.toml
//! Subsequent boots: instant load from cache.
//! Daemons read config via gRPC NodeConfigService — never touch node.toml directly.

pub mod types;
pub mod hardware;
pub mod environment;
pub mod llm;

pub use types::*;

use std::path::{Path, PathBuf};

// ============================================================
// PROBE ENTRY POINT
// ============================================================

pub async fn run(force_reprobe: bool) -> NodeConfig {
    let config_path = config_file_path();

    if config_path.exists() && !force_reprobe {
        match load_config(&config_path) {
            Ok(cfg) => {
                klog!("[Ring 0] ✅ Node config loaded from cache: {}", config_path.display());
                return cfg;
            }
            Err(e) => println!("[Ring 0] ANOMALY: Config corrupt ({}). Re-probing...", e),
        }
    }

    klog!("[Ring 0] First boot — comprehensive probe running...");

    // All probes run in parallel
    let (hw, env, compute) = tokio::join!(
        hardware::probe_hardware(),
        environment::probe_environment(),
        hardware::probe_compute(),
    );

    // LLM probe uses env + compute results
    let llm = llm::probe_llm_resources(&env, &compute).await;

    let mut config = NodeConfig {
        probed_at: chrono::Utc::now().to_rfc3339(),
        hardware: hw,
        compute,
        llm,
        env,
        suggestions: vec![],
    };

    // Run Sovereignty Advisor
    config.suggestions = SovereigntyAdvisor::analyze(&config);

    if !config.suggestions.is_empty() {
        klog!("\n╔══════════════════════════════════════╗");
        klog!("║       SOVEREIGNTY SUGGESTIONS        ║");
        klog!("╚══════════════════════════════════════╝");
        for s in &config.suggestions {
            klog!(" > {}", s);
        }
    }

    match save_config(&config, &config_path) {
        Err(e) => println!("[Ring 0] ANOMALY: Could not save config: {}", e),
        Ok(_)  => println!("[Ring 0] ✅ Node config saved: {}", config_path.display()),
    }
    config
}

// ============================================================
// CONFIG PERSISTENCE
// ============================================================

fn config_file_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = home.join(".cynic");
    std::fs::create_dir_all(&dir).ok();
    dir.join("node.toml")
}

fn save_config(config: &NodeConfig, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

fn load_config(path: &Path) -> Result<NodeConfig, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
