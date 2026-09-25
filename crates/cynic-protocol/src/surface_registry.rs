use serde::{Deserialize, Serialize};

pub const SURFACE_REGISTRY_SCHEMA: &str = "cynic.surface_registry.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceRegistryDocument {
    pub schema: String,
    pub surfaces: Vec<CanonicalCliSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCliSurface {
    pub id: String,
    pub layer: String,
    pub owner_crate: String,
    pub adapter_module: String,
    pub domain_owner: String,
    pub owner_modules: Vec<String>,
    pub owner_call_markers: Vec<String>,
    pub responsibility: String,
    pub mutation_policy: SurfaceMutationPolicy,
    pub canonical_entrypoints: Vec<String>,
    pub evidence: Vec<String>,
    pub anti_responsibilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<SurfaceArchitectureContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMutationPolicy {
    ReadOnly,
    ReadOnlyTempArtifacts,
    ArtifactGeneration,
    ExplicitApplyOnly,
    ExplicitApplyOnlyAfterSignatureDigestEpochNodeAndSlotPreflight,
    ReadOnlyByDefaultExplicitApplyBoundedRestart,
    ReadOnlyByDefaultExplicitApplyForActuators,
    AdmittedDispatchOrExplicitApplyOnly,
    ReadOnlyByDefaultExplicitApplyCreateOnce,
    ExplicitApplyOrAdmittedNodeDesiredStateAfterSignatureDigestEpochNodeAndSlotPreflight,
    ExplicitApplyOnlyWithBootstrapHumanAdminSecretOnStdinAndOwnershipProvenCompensation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SurfaceMutationDimensions {
    pub read_only_default: bool,
    pub requires_explicit_apply: bool,
    pub allows_admitted_dispatch: bool,
    pub allows_artifact_generation: bool,
    pub allows_temporary_files: bool,
    pub allows_process_execution: bool,
    pub create_once: bool,
    pub human_admin_stdin: bool,
    pub requires_compensation_proof: bool,
    pub requires_signature_digest_epoch_node_slot_preflight: bool,
}

impl SurfaceMutationPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::ReadOnlyTempArtifacts => "read_only_temp_artifacts",
            Self::ArtifactGeneration => "artifact_generation",
            Self::ExplicitApplyOnly => "explicit_apply_only",
            Self::ExplicitApplyOnlyAfterSignatureDigestEpochNodeAndSlotPreflight => {
                "explicit_apply_only_after_signature_digest_epoch_node_and_slot_preflight"
            }
            Self::ReadOnlyByDefaultExplicitApplyBoundedRestart => {
                "read_only_by_default_explicit_apply_bounded_restart"
            }
            Self::ReadOnlyByDefaultExplicitApplyForActuators => {
                "read_only_by_default_explicit_apply_for_actuators"
            }
            Self::AdmittedDispatchOrExplicitApplyOnly => {
                "admitted_dispatch_or_explicit_apply_only"
            }
            Self::ReadOnlyByDefaultExplicitApplyCreateOnce => {
                "read_only_by_default_explicit_apply_create_once"
            }
            Self::ExplicitApplyOrAdmittedNodeDesiredStateAfterSignatureDigestEpochNodeAndSlotPreflight => {
                "explicit_apply_or_admitted_node_desired_state_after_signature_digest_epoch_node_and_slot_preflight"
            }
            Self::ExplicitApplyOnlyWithBootstrapHumanAdminSecretOnStdinAndOwnershipProvenCompensation => {
                "explicit_apply_only_with_bootstrap_human_admin_secret_on_stdin_and_ownership_proven_compensation"
            }
        }
    }

    pub const fn dimensions(self) -> SurfaceMutationDimensions {
        let read_only_default = matches!(
            self,
            Self::ReadOnly
                | Self::ReadOnlyTempArtifacts
                | Self::ReadOnlyByDefaultExplicitApplyBoundedRestart
                | Self::ReadOnlyByDefaultExplicitApplyForActuators
                | Self::ReadOnlyByDefaultExplicitApplyCreateOnce
        );
        let requires_explicit_apply = !matches!(
            self,
            Self::ReadOnly | Self::ReadOnlyTempArtifacts | Self::ArtifactGeneration
        );
        let allows_admitted_dispatch = matches!(
            self,
            Self::AdmittedDispatchOrExplicitApplyOnly
                | Self::ExplicitApplyOrAdmittedNodeDesiredStateAfterSignatureDigestEpochNodeAndSlotPreflight
        );
        let allows_artifact_generation = !matches!(self, Self::ReadOnly);
        let allows_temporary_files = !matches!(self, Self::ReadOnly);
        let allows_process_execution = !matches!(self, Self::ArtifactGeneration);
        SurfaceMutationDimensions {
            read_only_default,
            requires_explicit_apply,
            allows_admitted_dispatch,
            allows_artifact_generation,
            allows_temporary_files,
            allows_process_execution,
            create_once: matches!(self, Self::ReadOnlyByDefaultExplicitApplyCreateOnce),
            human_admin_stdin: matches!(
                self,
                Self::ExplicitApplyOnlyWithBootstrapHumanAdminSecretOnStdinAndOwnershipProvenCompensation
            ),
            requires_compensation_proof: matches!(
                self,
                Self::ExplicitApplyOnlyWithBootstrapHumanAdminSecretOnStdinAndOwnershipProvenCompensation
            ),
            requires_signature_digest_epoch_node_slot_preflight: matches!(
                self,
                Self::ExplicitApplyOnlyAfterSignatureDigestEpochNodeAndSlotPreflight
                    | Self::ExplicitApplyOrAdmittedNodeDesiredStateAfterSignatureDigestEpochNodeAndSlotPreflight
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceArchitectureContract {
    pub owned_ports: Vec<String>,
    pub owned_decisions: Vec<String>,
    pub allowed_adapters: Vec<String>,
    pub projection_modules: Vec<String>,
    pub forbidden_projection_markers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_policy_is_typed_and_projects_operational_dimensions() {
        let policy: SurfaceMutationPolicy = serde_json::from_str(
            "\"explicit_apply_only_with_bootstrap_human_admin_secret_on_stdin_and_ownership_proven_compensation\"",
        )
        .expect("known policy");

        let dimensions = policy.dimensions();
        assert!(dimensions.requires_explicit_apply);
        assert!(dimensions.human_admin_stdin);
        assert!(dimensions.requires_compensation_proof);
        assert!(!dimensions.allows_admitted_dispatch);
    }

    #[test]
    fn admitted_runtime_seed_policy_is_distinct_from_human_bootstrap() {
        let dimensions = SurfaceMutationPolicy::ExplicitApplyOrAdmittedNodeDesiredStateAfterSignatureDigestEpochNodeAndSlotPreflight
            .dimensions();

        assert!(dimensions.requires_explicit_apply);
        assert!(dimensions.allows_admitted_dispatch);
        assert!(dimensions.requires_signature_digest_epoch_node_slot_preflight);
        assert!(!dimensions.human_admin_stdin);
    }
}
