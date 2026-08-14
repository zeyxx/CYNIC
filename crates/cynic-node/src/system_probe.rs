#![allow(dead_code)]
//! SystemProbe concern: collect hardware metrics (CPU, RAM, Disk)
//! and POST them as observations to the kernel.

use cynic_protocol::{KernelEndpointPolicy, OBSERVE_PATH, kernel_url as build_kernel_url};
use reqwest::Client;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::adapters::local_system_probe::{LinuxPlatformProbe, SysinfoMetricsSampler};
use crate::domain::system_observation::{system_metrics_payload, system_observation_payload};
use crate::ports::system_probe_port::{SystemMetricsPort, SystemProbePlatformPort};

pub(crate) async fn run_system_probe(
    client: Client,
    kernel_url: String,
    api_key: String,
    node_name: String,
    interval_secs: u64,
) {
    run_system_probe_with_platform(
        client,
        kernel_url,
        api_key,
        node_name,
        interval_secs,
        SysinfoMetricsSampler::new(),
        LinuxPlatformProbe,
    )
    .await;
}

async fn run_system_probe_with_platform(
    client: Client,
    kernel_url: String,
    api_key: String,
    node_name: String,
    interval_secs: u64,
    mut metrics_sampler: impl SystemMetricsPort,
    platform: impl SystemProbePlatformPort,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    // Skip first tick (instant)
    interval.tick().await;

    info!(node = %node_name, interval = interval_secs, "System probe active");

    loop {
        interval.tick().await;

        let sample = metrics_sampler.sample_metrics();
        let platform_observation = platform.observe_platform();
        for vmid in &platform_observation.critical_vmids_to_restart {
            warn!(
                "L0 Watchdog: Critical node cynic-vm-v2 is stopped. Requesting platform restart for resource {}.",
                vmid
            );
            platform.restart_critical_vmid(vmid);
        }

        let metrics = system_metrics_payload(sample, platform_observation);

        let payload = system_observation_payload(&node_name, metrics);

        let url = match build_kernel_url(
            &kernel_url,
            OBSERVE_PATH,
            KernelEndpointPolicy {
                allow_loopback: true,
                ..KernelEndpointPolicy::default()
            },
        ) {
            Ok(url) => url,
            Err(e) => {
                error!(error = %e, "Error building kernel observation URL");
                continue;
            }
        };

        match client
            .post(&url)
            .bearer_auth(&api_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                // Success - silent or debug log
            }
            Ok(resp) => {
                warn!(
                    status = resp.status().as_u16(),
                    "Failed to send system observation"
                );
            }
            Err(e) => {
                error!(error = %e, "Error sending system observation");
            }
        }
    }
}
