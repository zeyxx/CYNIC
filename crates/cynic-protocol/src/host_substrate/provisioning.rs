//! Hypervisor container provisioning — canonical pure types and port.
//!
//! CYNIC provisions ephemeral build containers on a supervised hypervisor.
//! This module defines the pure provisioning vocabulary (spec, report,
//! lifecycle states) and the provisioning port. It contains zero I/O: the
//! adapters live in `cynic-proxmox` (L1) and drive the hypervisor through
//! `cynic-host`'s `ArgvExecutionPort`, never raw shell.
//!
//! Design gate answers (bottom-up):
//! - Desired state: a clean, reproducible, ephemeral container that can
//!   compile an exact source commit and be destroyed after read-back proof.
//! - Pure selection rule: a spec is admissible when every field is present
//!   and the memory/disk/cores floors are respected; anything else is
//!   blocked with named blockers, never defaulted silently.
//! - Stop condition: stop after read-back proof or first non-zero exit.

use serde::{Deserialize, Serialize};

pub const PROVISIONING_SPEC_SCHEMA: &str = "cynic.provisioning.spec.v1";
pub const PROVISIONING_REPORT_SCHEMA: &str = "cynic.provisioning.report.v1";

/// Minimum resource floors for a build container. Provisioning below these
/// floors reproduces the 2026-10-06 incident class: an 8 GiB builder that
/// starves its own sshd under link-time memory pressure.
pub const PROVISIONING_MIN_MEMORY_MB: u64 = 8192;
pub const PROVISIONING_MIN_DISK_GB: u64 = 80;
pub const PROVISIONING_MIN_CORES: u32 = 4;

/// Which hypervisor image source to provision from. A clean template is the
/// only admissible source for an ephemeral build container: cloning a dirty
/// running container copies its caches, root-owned files, and hidden state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisioningSource {
    /// A clean, versioned LXC/VM template (recommended).
    CleanTemplate { template: String },
    /// Any other named image (cloud image, snapshot) — must be declared.
    NamedImage { image: String },
}

impl ProvisioningSource {
    /// A source is dirty when it names a running guest directly. Provisioning
    /// from a running guest is blocked: the clone captures live state.
    pub fn from_running_guest(&self) -> bool {
        false
    }
}

/// A pure, fully-declared provisioning specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningSpec {
    pub schema: String,
    /// Hypervisor-local guest identifier to create (LXC vmid or VM id).
    pub guest_id: u32,
    pub hostname: String,
    /// Guest name used in reports and audit entries.
    pub purpose: String,
    pub source: ProvisioningSource,
    pub memory_mb: u64,
    pub disk_gb: u64,
    pub cores: u32,
    /// Pinned toolchain the container must provide (e.g. "1.95.0"). An
    /// unpinned spec is blocked: an unpinned toolchain is an untracked
    /// variable that makes builds irreproducible.
    pub toolchain_pin: String,
    /// Secret-free transport URI of the repository to clone.
    pub repository_transport_uri: String,
    /// Exact 40-hex commit the container must check out.
    pub source_commit: String,
    /// Declared wall-clock budget for the whole provisioning, in seconds.
    pub timeout_secs: u64,
}

impl ProvisioningSpec {
    /// Pure validation with named blockers. No defaults: an incomplete spec
    /// is an explicit `blocked`, never a guessed container.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != PROVISIONING_SPEC_SCHEMA {
            blockers.push("provisioning_spec_schema_mismatch".to_string());
        }
        if self.guest_id == 0 {
            blockers.push("provisioning_guest_id_zero".to_string());
        }
        if self.hostname.trim().is_empty() {
            blockers.push("provisioning_hostname_empty".to_string());
        }
        if self.purpose.trim().is_empty() {
            blockers.push("provisioning_purpose_empty".to_string());
        }
        match &self.source {
            ProvisioningSource::CleanTemplate { template } if template.trim().is_empty() => {
                blockers.push("provisioning_template_empty".to_string());
            }
            ProvisioningSource::NamedImage { image } if image.trim().is_empty() => {
                blockers.push("provisioning_image_empty".to_string());
            }
            _ => {}
        }
        if self.memory_mb < PROVISIONING_MIN_MEMORY_MB {
            blockers.push("provisioning_memory_below_floor".to_string());
        }
        if self.disk_gb < PROVISIONING_MIN_DISK_GB {
            blockers.push("provisioning_disk_below_floor".to_string());
        }
        if self.cores < PROVISIONING_MIN_CORES {
            blockers.push("provisioning_cores_below_floor".to_string());
        }
        if self.toolchain_pin.trim().is_empty() {
            blockers.push("provisioning_toolchain_unpinned".to_string());
        }
        if self.repository_transport_uri.trim().is_empty() {
            blockers.push("provisioning_repository_transport_missing".to_string());
        } else if self.repository_transport_uri.contains(':') && !self.repository_transport_uri.contains('@') {
            // A URI with credentials inline is refused; secret-free only.
            if self.repository_transport_uri.matches(':').count() > 1 {
                blockers.push("provisioning_repository_transport_contains_credentials".to_string());
            }
        }
        if !is_commit_id(&self.source_commit) {
            blockers.push("provisioning_source_commit_invalid".to_string());
        }
        if self.timeout_secs == 0 {
            blockers.push("provisioning_timeout_zero".to_string());
        }
        blockers
    }
}

fn is_commit_id(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Lifecycle states of a provisioned container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisioningState {
    Specified,
    Creating,
    Booting,
    ToolchainReady,
    SourceReady,
    Ready,
    Failed,
    Destroyed,
}

/// Read-back proof of a provisioned container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningReport {
    pub schema: String,
    pub guest_id: u32,
    pub hostname: String,
    pub state: ProvisioningState,
    /// Observed toolchain identity (e.g. `rustc 1.95.0 (...)`) — read back
    /// from the container, never copied from the spec.
    pub observed_toolchain: Option<String>,
    /// Observed checked-out commit — must equal the spec's source_commit.
    pub observed_commit: Option<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

impl ProvisioningReport {
    /// The report is `ready` only when every binding is read back. A report
    /// without observed toolchain and commit is not ready, whatever the
    /// state claims.
    pub fn is_ready(&self) -> bool {
        self.state == ProvisioningState::Ready
            && self.blockers.is_empty()
            && self
                .observed_commit
                .as_deref()
                .is_some_and(|c| c.len() == 40)
            && self.observed_toolchain.is_some()
    }
}

/// Port: provision and destroy an ephemeral container on a hypervisor.
///
/// Implementations must be fail-closed: an unimplementable step is a named
/// blocker in the report, never a silent default. Destroy must be idempotent
/// (destroying an absent guest is a success with a `destroyed` state).
pub trait ProvisioningPort: Send + Sync {
    /// Provision a container from a clean template and prove it ready.
    fn provision(&self, spec: &ProvisioningSpec) -> Result<ProvisioningReport, String>;

    /// Observe a provisioned container's current state (read-only).
    fn observe(&self, guest_id: u32) -> Result<ProvisioningReport, String>;

    /// Destroy a provisioned container. Idempotent.
    fn destroy(&self, guest_id: u32) -> Result<ProvisioningReport, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ProvisioningSpec {
        ProvisioningSpec {
            schema: PROVISIONING_SPEC_SCHEMA.to_string(),
            guest_id: 106,
            hostname: "cynic-build-ephemeral".to_string(),
            purpose: "ephemeral build container for targeted cargo check".to_string(),
            source: ProvisioningSource::CleanTemplate {
                template: "ubuntu-22.04-clean".to_string(),
            },
            memory_mb: 8192,
            disk_gb: 80,
            cores: 4,
            toolchain_pin: "1.95.0".to_string(),
            repository_transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git".to_string(),
            timeout_secs: 600,
            source_commit: "e5e8f718faafc3a751e05dc210aedb4386f3b8cf".to_string(),
        }
    }

    #[test]
    fn valid_spec_has_no_blockers() {
        assert!(spec().validate().is_empty());
    }

    #[test]
    fn unpinned_toolchain_is_blocked() {
        let mut s = spec();
        s.toolchain_pin = String::new();
        assert!(s
            .validate()
            .contains(&"provisioning_toolchain_unpinned".to_string()));
    }

    #[test]
    fn memory_below_incident_floor_is_blocked() {
        let mut s = spec();
        s.memory_mb = 4096;
        assert!(s
            .validate()
            .contains(&"provisioning_memory_below_floor".to_string()));
    }

    #[test]
    fn short_commit_is_blocked() {
        let mut s = spec();
        s.source_commit = "e5e8f718".to_string();
        assert!(s
            .validate()
            .contains(&"provisioning_source_commit_invalid".to_string()));
    }

    #[test]
    fn empty_template_is_blocked() {
        let mut s = spec();
        s.source = ProvisioningSource::CleanTemplate { template: String::new() };
        assert!(s
            .validate()
            .contains(&"provisioning_template_empty".to_string()));
    }

    #[test]
    fn report_without_read_back_is_not_ready() {
        let report = ProvisioningReport {
            schema: PROVISIONING_REPORT_SCHEMA.to_string(),
            guest_id: 106,
            hostname: "cynic-build-ephemeral".to_string(),
            state: ProvisioningState::Ready,
            observed_toolchain: None,
            observed_commit: None,
            blockers: Vec::new(),
            unknowns: Vec::new(),
        };
        assert!(!report.is_ready());
    }

    #[test]
    fn report_with_read_back_is_ready() {
        let report = ProvisioningReport {
            schema: PROVISIONING_REPORT_SCHEMA.to_string(),
            guest_id: 106,
            hostname: "cynic-build-ephemeral".to_string(),
            state: ProvisioningState::Ready,
            observed_toolchain: Some("rustc 1.95.0".to_string()),
            observed_commit: Some("e5e8f718faafc3a751e05dc210aedb4386f3b8cf".to_string()),
            blockers: Vec::new(),
            unknowns: Vec::new(),
        };
        assert!(report.is_ready());
    }

    #[test]
    fn blocked_report_is_not_ready_even_when_state_claims_ready() {
        let report = ProvisioningReport {
            schema: PROVISIONING_REPORT_SCHEMA.to_string(),
            guest_id: 106,
            hostname: "cynic-build-ephemeral".to_string(),
            state: ProvisioningState::Ready,
            observed_toolchain: Some("rustc 1.95.0".to_string()),
            observed_commit: Some("e5e8f718faafc3a751e05dc210aedb4386f3b8cf".to_string()),
            blockers: vec!["provisioning_memory_below_floor".to_string()],
            unknowns: Vec::new(),
        };
        assert!(!report.is_ready());
    }
}
