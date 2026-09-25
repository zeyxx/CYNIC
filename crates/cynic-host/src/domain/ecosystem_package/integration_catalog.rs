//! Stable integration identities and capability ownership.

use super::integration_types::{
    IntegrationCapability, IntegrationDependencySpec, IntegrationKind, IntegrationPrincipal,
    IntegrationSpec,
};
use crate::domain::ecosystem_package::{python_cli_policy, types::ToolingAuthorityFamily};

const GITHUB_CAPABILITIES: &[IntegrationCapability] = &[IntegrationCapability::PublicRelay];
const HF_CAPABILITIES: &[IntegrationCapability] = &[IntegrationCapability::ModelHub];
const GITHUB_DEPENDENCY: IntegrationDependencySpec = IntegrationDependencySpec {
    id: "github-cli",
    install_principal: IntegrationPrincipal::OperatorLocal,
    tooling_authority_family: None,
    expected_tooling_provenance: None,
};
pub fn integration_specs() -> Vec<IntegrationSpec> {
    let hf_policy = python_cli_policy("huggingface-cli").expect("hf python cli policy");
    vec![
        IntegrationSpec {
            id: "github",
            kind: IntegrationKind::ExternalCli,
            desired_principal: IntegrationPrincipal::OperatorLocal,
            dependency: Some(GITHUB_DEPENDENCY),
            connection_required: true,
            capabilities: GITHUB_CAPABILITIES,
        },
        IntegrationSpec {
            id: "huggingface",
            kind: IntegrationKind::ExternalCli,
            desired_principal: IntegrationPrincipal::RuntimeUser,
            dependency: Some(IntegrationDependencySpec {
                id: "huggingface-cli",
                install_principal: IntegrationPrincipal::RuntimeUser,
                tooling_authority_family: Some(ToolingAuthorityFamily::Python),
                expected_tooling_provenance: Some(hf_policy.expected_tooling_provenance),
            }),
            connection_required: true,
            capabilities: HF_CAPABILITIES,
        },
    ]
}
