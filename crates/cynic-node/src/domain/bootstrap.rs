//! Bootstrap domain language for a node becoming part of the organism.

use serde::Serialize;

/// Ordered bootstrap stages emitted by the L0 watchdog.
#[derive(Debug, Clone, Serialize)]
pub struct BootstrapStage {
    pub name: &'static str,
    pub status: BootstrapStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapStatus {
    Observed,
    Degraded,
}

/// Pure domain projection: summarize the minimum birth sequence for a node.
pub fn l0_bootstrap_stages(kernel_reachable: bool, services_sampled: bool) -> Vec<BootstrapStage> {
    vec![
        BootstrapStage {
            name: "binary_alive",
            status: BootstrapStatus::Observed,
        },
        BootstrapStage {
            name: "local_identity_observed",
            status: BootstrapStatus::Observed,
        },
        BootstrapStage {
            name: "kernel_reachable",
            status: if kernel_reachable {
                BootstrapStatus::Observed
            } else {
                BootstrapStatus::Degraded
            },
        },
        BootstrapStage {
            name: "services_sampled",
            status: if services_sampled {
                BootstrapStatus::Observed
            } else {
                BootstrapStatus::Degraded
            },
        },
    ]
}
