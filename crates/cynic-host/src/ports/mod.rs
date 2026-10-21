//! Ports: the domain's own interface.
//!
//! ARCHITECTURE INVARIANT — `domain <-> ports` is INTRA-LAYER, not a cycle.
//!
//! This module and `domain` are one layer, not two. `domain/shared.rs` declares
//! `SubstrateDomainContract`; `domain/{services,privilege,packages,principal}.rs`
//! implement it. A module cannot implement a trait without the trait in scope, so
//! the imports necessarily run both ways:
//!
//!   ports/mod.rs            imports `domain` (this file, 10 times)
//!   domain/services.rs:10   imports `ports::HostOperation`
//!   domain/privilege.rs:12  imports `ports::HostOperation`
//!   domain/packages.rs:14   imports `ports::HostOperation`
//!   domain/principal.rs:11  imports `ports::{HostOperation, PrincipalLoginShell}`
//!
//! No file layout removes this, and it is not a defect to remove. Many Rust
//! hexagonal codebases carry it. It is recorded here because a module-level
//! cycle analysis will report `domain <-> ports` as an entanglement, and without
//! this note the next reader spends a day trying to fix a language constraint.
//!
//! What IS a defect, and is fixed rather than documented:
//!   ports -> adapters   a port naming an implementation (ports depend on
//!                       abstractions, never the reverse)
//!   domain -> adapters  the domain knowing which client it uses
//! Both were present in this crate and are being removed.

pub mod argv_execution;
pub mod causal_epoch_index;
pub mod evidence_store;
// provider_substrate ports moved to cynic-protocol::provider::ports (pure, zero I/O).
// remote_execution port moved to cynic-protocol::host_substrate::remote_execution (pure, zero I/O).
use crate::bridge::{PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState};
use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackageObservedState;
use crate::domain::packages::PackagePolicy;
use crate::domain::principal::PrincipalObservedState;
use crate::domain::principal::PrincipalPolicy;
use crate::domain::privilege::PrivilegeObservedState;
use crate::domain::privilege::PrivilegePolicy;
use crate::domain::services::ServiceObservedState;
use crate::domain::services::ServicePolicy;
use crate::domain::shared::SubstrateDomainContract;
use crate::execution::AdmittedHostOperation;
use crate::hardware_conformance::HardwareObservation;
use cynic_node_protocol::PreOsPlatformObservation;
// Re-exported from cynic-protocol (canonical home of the pure ports).
pub use cynic_protocol::provider::{NodeSubstrateControlPort, NodeSubstrateObservationPort};
// Re-exported from cynic-protocol (canonical home of the pure port).
pub use cynic_protocol::host_substrate::remote_execution::RemoteExecutionPort;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostOperation {
    EnsureSystemGroup {
        group: String,
    },
    EnsureSystemUser {
        user: String,
        group: String,
        home: String,
        shell: PrincipalLoginShell,
    },
    EnsureSupplementaryGroupMember {
        user: String,
        group: String,
    },
    EnsureOwnedDir {
        path: String,
        user: String,
        group: String,
        mode: u32,
    },
    InstallOwnedFile {
        source: String,
        target: String,
        user: String,
        group: String,
        mode: u32,
    },
    ValidatePrivilegeProjection {
        provider: String,
        path: String,
    },
    EnsureDir {
        path: String,
        mode: u32,
    },
    WritePackageSourceProjection {
        path: String,
        approved_sources: Vec<String>,
    },
    InstallBootstrapPackages {
        approved_sources: Vec<String>,
        packages: Vec<String>,
    },
    ServiceManagerDaemonReload,
    ServiceEnableNow {
        unit: String,
    },
    ServiceStart {
        unit: String,
    },
    /// Retire a unit this host must no longer run.
    ///
    /// Bounded by `ServicePolicy::retired_units`: a unit that is not
    /// named there is never disabled, stopped or removed. The operation
    /// is the destructive inverse of `ServiceEnableNow`, so it is a
    /// separate variant rather than a flag: an admit/plan digest names
    /// exactly which verbs it carries.
    ServiceRetire {
        unit: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalLoginShell {
    NoLogin,
}

pub trait HostPrincipalPort {
    fn desired_principal(&self) -> PrincipalPolicy;
}

pub trait HostPrincipalObservationPort {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState;
}

pub trait HostPrivilegePort {
    fn desired_privilege(&self) -> PrivilegePolicy;
}

pub trait HostPrivilegeObservationPort {
    fn observe_privilege(&self, policy: &PrivilegePolicy) -> PrivilegeObservedState;
}

pub trait HostPrivilegeBridgeObservationPort {
    fn observe_privilege_bridge(
        &self,
        spec: &PrivilegeBridgeInstallSpec,
    ) -> PrivilegeBridgeObservedState;
}

pub trait HostPackagePort {
    fn desired_packages(&self) -> PackagePolicy;
}

pub trait HostPackageObservationPort {
    fn observe_packages(&self, policy: &PackagePolicy) -> PackageObservedState;
}

pub trait HostFilesystemPort {
    fn desired_filesystem(&self) -> FilesystemPolicy;
}

pub trait HostServicePort {
    fn desired_services(&self) -> ServicePolicy;
}

pub trait HostServiceObservationPort {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState;
}

pub trait HostHardwareObservationPort {
    fn observe_hardware(&self) -> HardwareObservation;
}

pub trait PreOsPlatformObservationPort {
    fn observe_pre_os_platform(&self) -> PreOsPlatformObservation;
}

pub trait PreOsArtifactObservationPort {
    fn observe_pre_os_artifacts(&self) -> Vec<crate::pre_os_control::PreOsArtifactObservation>;
}

pub trait PreOsAttestationObservationPort {
    fn observe_pre_os_attestation_sink(
        &self,
    ) -> crate::pre_os_control::PreOsAttestationSinkObservation;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait HostCommandRunnerPort {
    fn run_operation(&self, operation: &AdmittedHostOperation) -> HostCommandResult;
}

pub trait HostContractPort:
    HostPrincipalPort + HostPrivilegePort + HostPackagePort + HostFilesystemPort + HostServicePort
{
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract>;
}
