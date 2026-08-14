use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostSubstrateDomain {
    Principal,
    Privilege,
    Packages,
    Filesystem,
    Services,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceStatus {
    Planned,
    Active,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Missing,
    PrincipalMismatch,
    ConnectionRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleDisposition {
    NotDiscovered,
    RequiresPrincipalAdoption,
    RequiresConnection,
    ObservableCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    Observe,
    Adopt,
    Connect,
    RepairService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleProjection {
    pub status: LifecycleStatus,
    pub disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
}

impl LifecycleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleStatus::Missing => "missing",
            LifecycleStatus::PrincipalMismatch => "principal_mismatch",
            LifecycleStatus::ConnectionRequired => "connection_required",
            LifecycleStatus::Ready => "ready",
        }
    }

    pub fn default_disposition(self) -> LifecycleDisposition {
        match self {
            LifecycleStatus::Missing => LifecycleDisposition::NotDiscovered,
            LifecycleStatus::PrincipalMismatch => LifecycleDisposition::RequiresPrincipalAdoption,
            LifecycleStatus::ConnectionRequired => LifecycleDisposition::RequiresConnection,
            LifecycleStatus::Ready => LifecycleDisposition::ObservableCandidate,
        }
    }
}

impl LifecycleDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleDisposition::NotDiscovered => "not_discovered",
            LifecycleDisposition::RequiresPrincipalAdoption => "requires_principal_adoption",
            LifecycleDisposition::RequiresConnection => "requires_connection",
            LifecycleDisposition::ObservableCandidate => "observable_candidate",
        }
    }
}

impl BoundaryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BoundaryKind::Observe => "observe",
            BoundaryKind::Adopt => "adopt",
            BoundaryKind::Connect => "connect",
            BoundaryKind::RepairService => "repair_service",
        }
    }
}

impl LifecycleProjection {
    pub fn new(
        status: LifecycleStatus,
        next_boundary_kind: BoundaryKind,
        next_boundary_reason: impl Into<String>,
        next_command: Option<String>,
    ) -> Self {
        Self {
            status,
            disposition: status.default_disposition(),
            next_boundary_kind,
            next_boundary_reason: next_boundary_reason.into(),
            next_command,
        }
    }

    pub fn ready_observe(next_boundary_reason: impl Into<String>) -> Self {
        Self::new(
            LifecycleStatus::Ready,
            BoundaryKind::Observe,
            next_boundary_reason,
            None,
        )
    }

    pub fn missing_adopt(
        next_boundary_reason: impl Into<String>,
        next_command: Option<String>,
    ) -> Self {
        Self::new(
            LifecycleStatus::Missing,
            BoundaryKind::Adopt,
            next_boundary_reason,
            next_command,
        )
    }

    pub fn principal_mismatch_adopt(
        next_boundary_reason: impl Into<String>,
        next_command: Option<String>,
    ) -> Self {
        Self::new(
            LifecycleStatus::PrincipalMismatch,
            BoundaryKind::Adopt,
            next_boundary_reason,
            next_command,
        )
    }

    pub fn connection_required_observe(next_boundary_reason: impl Into<String>) -> Self {
        Self::new(
            LifecycleStatus::ConnectionRequired,
            BoundaryKind::Observe,
            next_boundary_reason,
            None,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceContract {
    pub surface: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateEvidence {
    pub id: String,
    pub status: SurfaceStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateBundleFile {
    pub logical_name: String,
    pub install_path: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateDomainContract {
    pub domain: HostSubstrateDomain,
    pub status: SurfaceStatus,
    pub required_surfaces: Vec<SurfaceContract>,
    pub evidences: Vec<SubstrateEvidence>,
}
