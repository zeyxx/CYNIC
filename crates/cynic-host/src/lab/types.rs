use serde::Serialize;
use serde_json::Value;

use crate::runtime_harness::RuntimeHarnessContext;

#[derive(Debug, Clone, Serialize)]
pub struct LabPlanReport {
    pub(crate) schema: &'static str,
    pub(crate) context: RuntimeHarnessContext,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) can_apply: bool,
    pub(crate) registry_path: String,
    pub(crate) provider: LabProviderPlan,
    pub(crate) isolation: LabIsolationPlan,
    pub(crate) vm_count: usize,
    pub(crate) vms: Vec<LabVmPlan>,
    pub(crate) execution_suite: Vec<LabExecutionStep>,
    pub(crate) blockers: Vec<&'static str>,
    pub(crate) safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabProvisionReport {
    pub(crate) schema: &'static str,
    pub(crate) context: RuntimeHarnessContext,
    pub status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) can_apply: bool,
    pub(crate) registry_path: String,
    pub(crate) provider: LabProviderPlan,
    pub(crate) isolation: LabIsolationPlan,
    pub(crate) image: LabImagePreflight,
    pub(crate) guest_bootstrap: LabGuestBootstrapPreflight,
    pub(crate) vms: Vec<LabVmPlan>,
    pub(crate) planned_mutations: Vec<LabExecutionStep>,
    pub(crate) blockers: Vec<&'static str>,
    pub(crate) safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabBundleReport {
    pub(crate) schema: &'static str,
    pub(crate) context: RuntimeHarnessContext,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) registry_path: String,
    pub(crate) output_dir: String,
    pub(crate) files: Vec<String>,
    pub(crate) network_name: &'static str,
    pub(crate) vm_count: usize,
    pub(crate) safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabImageReport {
    pub(crate) schema: &'static str,
    pub(crate) context: RuntimeHarnessContext,
    pub status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) requested_apply: bool,
    pub(crate) can_fetch: bool,
    pub(crate) image: LabImagePreflight,
    pub(crate) commands: Vec<LabImageCommand>,
    pub(crate) executions: Vec<LabImageExecution>,
    pub(crate) observed_sha256: Option<String>,
    pub(crate) checksum_matches: bool,
    pub(crate) blockers: Vec<&'static str>,
    pub(crate) safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabImageResolveReport {
    pub(crate) schema: &'static str,
    pub(crate) context: RuntimeHarnessContext,
    pub status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) image_name: String,
    pub(crate) sha256_url: Option<String>,
    pub(crate) resolved_sha256: Option<String>,
    pub(crate) registry_sha256: Option<String>,
    pub(crate) registry_matches_resolved: bool,
    pub(crate) proposed_registry_patch: Option<Value>,
    pub(crate) blockers: Vec<&'static str>,
    pub(crate) safety_rules: Vec<&'static str>,
}

pub use crate::framework_bootstrap::FrameworkBootstrapBundleReport as LabGuestBootstrapBundleReport;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabProviderPlan {
    pub(crate) selected: &'static str,
    pub(crate) qemu_available: bool,
    pub(crate) virsh_available: bool,
    pub(crate) kvm_available: bool,
    pub(crate) rationale: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabIsolationPlan {
    pub(crate) network_name: &'static str,
    pub(crate) network_mode: &'static str,
    pub(crate) host_routing_mutation: bool,
    pub(crate) cidr: Option<String>,
    pub(crate) canonical_mesh_cidr: Option<String>,
    pub(crate) dns_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabVmPlan {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) local_ip: Option<String>,
    pub(crate) mesh_ip: Option<String>,
    pub(crate) services: Vec<String>,
    pub(crate) image_policy: &'static str,
    pub(crate) provision_state: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabExecutionStep {
    pub(crate) order: usize,
    pub(crate) name: &'static str,
    pub(crate) mutates: bool,
    pub(crate) status: &'static str,
    pub(crate) command: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabImagePreflight {
    pub(crate) declared: bool,
    pub(crate) path: Option<String>,
    pub(crate) source_url: Option<String>,
    pub(crate) sha256_url: Option<String>,
    pub(crate) sha256: Option<String>,
    pub(crate) checksum_declared: bool,
    pub(crate) checksum_valid_shape: bool,
    pub(crate) exists_on_host: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabGuestBootstrapPreflight {
    pub(crate) declared: bool,
    pub(crate) transport: Option<String>,
    pub(crate) ssh_user: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabImageCommand {
    pub(crate) role: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LabImageExecution {
    pub(crate) role: String,
    pub(crate) argv: Vec<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}
