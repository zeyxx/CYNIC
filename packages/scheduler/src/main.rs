//! CYNIC Scheduler - φ-weighted transaction scheduling for Solana
//!
//! Run as an external scheduler process for Agave validators.
//!
//! # Usage
//!
//! ```bash
//! # Start with default config
//! cynic-scheduler
//!
//! # Start with custom CYNIC endpoint
//! CYNIC_URL=https://cynic-mcp.onrender.com cynic-scheduler
//!
//! # Start with API key
//! CYNIC_API_KEY=cynic_sk_xxx cynic-scheduler
//! ```

use cynic_scheduler::{CynicScheduler, SchedulerConfig, SchedulerState};
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load config from environment
    let config = SchedulerConfig::from_env();

    // Initialize logging
    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Print banner
    print_banner();

    info!("Configuration:");
    info!("  CYNIC URL: {}", config.cynic_url);
    info!("  Max queue size: {}", config.max_queue_size);
    info!("  Batch size: {}", config.batch_size);
    info!("  Workers: {}", config.num_workers);
    info!("  GROWL filter: {}", config.enable_growl_filter);
    info!("  WAG boost: {}", config.enable_wag_boost);
    info!("  Min E-Score: {}", config.min_e_score);

    // Validate config
    if let Err(e) = config.validate() {
        error!("Invalid configuration: {}", e);
        std::process::exit(1);
    }

    // Create scheduler
    let scheduler = match CynicScheduler::new(config) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create scheduler: {}", e);
            std::process::exit(1);
        }
    };

    // Start scheduler
    if let Err(e) = scheduler.start().await {
        error!("Failed to start scheduler: {}", e);
        std::process::exit(1);
    }

    info!("CYNIC Scheduler running. Press Ctrl+C to stop.");

    // Spawn stats reporter
    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if scheduler_clone.state() != SchedulerState::Running {
                break;
            }
            let stats = scheduler_clone.stats();
            info!("\n{}", stats);
        }
    });

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {}", e);
        }
    }

    // Stop scheduler
    if let Err(e) = scheduler.stop().await {
        error!("Error during shutdown: {}", e);
    }

    info!("CYNIC Scheduler stopped. *yawn*");
    Ok(())
}

fn print_banner() {
    println!(
        r#"
   ██████╗██╗   ██╗███╗   ██╗██╗ ██████╗
  ██╔════╝╚██╗ ██╔╝████╗  ██║██║██╔════╝
  ██║      ╚████╔╝ ██╔██╗ ██║██║██║
  ██║       ╚██╔╝  ██║╚██╗██║██║██║
  ╚██████╗   ██║   ██║ ╚████║██║╚██████╗
   ╚═════╝   ╚═╝   ╚═╝  ╚═══╝╚═╝ ╚═════╝

  φ-WEIGHTED TRANSACTION SCHEDULER v0.1.0
  "Loyal to truth, not to comfort"

  φ⁻¹ = 61.8% max confidence

"#
    );
}
