//! Provider metadata registry (channel + principal per dep id).
//!
//! Recipe logic lives in `providers/`; this table is identity + policy only.

use super::types::{InstallPrincipalKind, ProviderChannel};

/// Built-in provider metadata (universal surface every dep exposes).
#[derive(Debug, Clone, Copy)]
pub struct EcosystemProviderSpec {
    pub id: &'static str,
    pub channel: ProviderChannel,
    pub principal: InstallPrincipalKind,
    pub role: &'static str,
}

/// Canonical provider table — extend here instead of ad-hoc recipes.
pub fn ecosystem_provider_specs() -> &'static [EcosystemProviderSpec] {
    &[
        EcosystemProviderSpec {
            id: "cynic",
            channel: ProviderChannel::Scripted,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "organism_cli",
        },
        EcosystemProviderSpec {
            id: "llama.cpp",
            channel: ProviderChannel::Scripted,
            principal: InstallPrincipalKind::RuntimeUser,
            role: "native_inference_engine",
        },
        EcosystemProviderSpec {
            id: "huggingface-cli",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "model_hub_download",
        },
        EcosystemProviderSpec {
            id: "ods",
            channel: ProviderChannel::Scripted,
            principal: InstallPrincipalKind::RuntimeUser,
            role: "ai_environment_bootstrap",
        },
        EcosystemProviderSpec {
            id: "ollama",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "compat_inference_peer",
        },
        EcosystemProviderSpec {
            id: "codex-cli",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "agent_coding",
        },
        EcosystemProviderSpec {
            id: "claude-code",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "agent_coding",
        },
        EcosystemProviderSpec {
            id: "grok-build",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "agent_coding",
        },
        EcosystemProviderSpec {
            id: "hermes-agent",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "agent_runtime",
        },
        EcosystemProviderSpec {
            id: "antigravity-cli",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "agent_cli_legacy",
        },
        EcosystemProviderSpec {
            id: "docker",
            channel: ProviderChannel::DistroManaged,
            principal: InstallPrincipalKind::SystemManaged,
            role: "container_runtime",
        },
        EcosystemProviderSpec {
            id: "rustc",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "build_toolchain",
        },
        EcosystemProviderSpec {
            id: "cargo",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "build_toolchain",
        },
        EcosystemProviderSpec {
            id: "node",
            channel: ProviderChannel::External,
            principal: InstallPrincipalKind::OperatorLocal,
            role: "js_toolchain",
        },
        EcosystemProviderSpec {
            id: "systemd",
            channel: ProviderChannel::DistroManaged,
            principal: InstallPrincipalKind::SystemManaged,
            role: "service_manager",
        },
    ]
}

pub fn provider_spec(id: &str) -> Option<&'static EcosystemProviderSpec> {
    ecosystem_provider_specs().iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ecosystem_package::types::ProviderChannel;

    #[test]
    fn provider_table_covers_llama_and_ods_as_scripted() {
        let llama = provider_spec("llama.cpp").unwrap();
        let ods = provider_spec("ods").unwrap();
        assert_eq!(llama.channel, ProviderChannel::Scripted);
        assert_eq!(ods.channel, ProviderChannel::Scripted);
        assert_eq!(llama.principal, InstallPrincipalKind::RuntimeUser);
        assert!(!matches!(
            provider_spec("systemd").unwrap().channel,
            ProviderChannel::Scripted
        ));
    }
}
