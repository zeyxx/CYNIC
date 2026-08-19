use super::{FleetNodeObservation, ProviderError};
#[path = "kubernetes.rs"]
mod kubernetes;
#[path = "proxmox.rs"]
mod proxmox;

pub(crate) use kubernetes::KubernetesInventoryProvider;
pub(crate) use proxmox::ProxmoxInventoryProvider;

pub(super) enum FleetInventoryProvider {
    Proxmox(ProxmoxInventoryProvider),
    Kubernetes(KubernetesInventoryProvider),
}

impl FleetInventoryProvider {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Proxmox(_) => "proxmox",
            Self::Kubernetes(_) => "kubernetes",
        }
    }

    pub(super) async fn collect_nodes(
        &self,
        declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        match self {
            Self::Proxmox(provider) => provider.collect_nodes(declared_nodes).await,
            Self::Kubernetes(provider) => provider.collect_nodes(declared_nodes).await,
        }
    }
}

pub(super) fn build_providers(
    registry_providers: &[String],
) -> (Vec<FleetInventoryProvider>, Vec<(String, String)>) {
    let provider_spec = std::env::var("CYNIC_FLEET_PROVIDERS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_lowercase())
        .unwrap_or_else(|| registry_providers.join(","));
    let enable_k8s = super::bool_flag("CYNIC_FLEET_K8S_ENABLED").unwrap_or_default();
    let providers = provider_spec
        .split(',')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    let mut list = Vec::new();
    let mut unavailable = Vec::new();
    for provider in providers {
        match provider.as_str() {
            "proxmox" | "pm" | "proxmox_api" => {
                if let Some(p) = ProxmoxInventoryProvider::from_env() {
                    list.push(FleetInventoryProvider::Proxmox(p));
                } else {
                    unavailable.push((
                        "proxmox".to_string(),
                        "provider_control_unconfigured".to_string(),
                    ));
                }
            }
            "k8s" | "kubernetes" | "kubernetes_api" => {
                if enable_k8s {
                    if let Some(p) = KubernetesInventoryProvider::from_env() {
                        list.push(FleetInventoryProvider::Kubernetes(p));
                    } else {
                        unavailable.push((
                            "kubernetes".to_string(),
                            "provider_configuration_unavailable".to_string(),
                        ));
                    }
                } else {
                    unavailable.push((
                        "kubernetes".to_string(),
                        "provider_disabled_by_policy".to_string(),
                    ));
                }
            }
            other => unavailable.push((
                other.to_string(),
                "provider_adapter_unsupported".to_string(),
            )),
        }
    }
    (list, unavailable)
}
