use std::fmt;
use std::sync::Arc;

use crate::backends;
use crate::domain;
use crate::infra;
use cynic_inference::application::proxy_routing::ProxyTargets;

pub struct RuntimeServices {
    pub domain_curations: Arc<domain::wisdom::DomainCurations>,
    pub enricher: Option<Arc<dyn domain::enrichment::TokenEnricherPort>>,
    pub domain_router: Arc<infra::domain_router::DomainRouter>,
    pub dog_perf_collector: Arc<infra::dog_performance::DogPerformanceCollector>,
    pub proxy_targets: Arc<ProxyTargets>,
    pub event_tx: tokio::sync::broadcast::Sender<domain::events::KernelEvent>,
    pub mail: Option<Arc<dyn domain::mail::MailPort>>,
    pub zones: Arc<domain::zones::ZoneConfig>,
}

impl fmt::Debug for RuntimeServices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeServices")
            .field("domain_curations", &self.domain_curations)
            .field("has_enricher", &self.enricher.is_some())
            .field("domain_router", &self.domain_router)
            .field("dog_perf_collector", &self.dog_perf_collector)
            .field("proxy_targets", &self.proxy_targets)
            .field("event_tx", &"broadcast::Sender<KernelEvent>")
            .field("has_mail", &self.mail.is_some())
            .field("zones", &self.zones)
            .finish()
    }
}

pub fn build_runtime_services(
    backend_configs: &[infra::config::BackendConfig],
    dog_thresholds: &Arc<infra::config::DogThresholds>,
) -> RuntimeServices {
    let curation_dir = selected_curation_dir();
    let domain_curations = match domain::wisdom::DomainCurations::load_from_path(&curation_dir) {
        Ok(curations) => {
            crate::klog!(
                "[Boot] Domain curations loaded from {:?} ({} domains)",
                curation_dir,
                curations.loaded_domains.len()
            );
            Arc::new(curations)
        }
        Err(error) => {
            crate::klog!(
                "[Boot] Domain curations failed to load: {} — wisdom enrichment will be empty",
                error
            );
            Arc::new(domain::wisdom::DomainCurations::new())
        }
    };

    let enricher: Option<Arc<dyn domain::enrichment::TokenEnricherPort>> =
        match backends::helius::HeliusEnricher::from_env() {
            Some(helius) => {
                let helius = helius.with_kscore_config(dog_thresholds.kscore.clone());
                crate::klog!(
                    "[Boot] Helius enricher configured — token-analysis with K-Score behavioral analysis"
                );
                Some(Arc::new(helius))
            }
            None => {
                crate::klog!(
                    "[Boot] Helius enricher not configured — using lo-fi DexScreener/RugCheck fallback"
                );
                Some(Arc::new(backends::lofi::LofiEnricher::new()))
            }
        };

    let domain_router = Arc::new(infra::domain_router::DomainRouter::from_backends(
        backend_configs,
    ));
    let dog_perf_collector = Arc::new(infra::dog_performance::DogPerformanceCollector::new());

    let mut fleet_meta = std::collections::HashMap::new();
    let mut sovereign_flags = std::collections::HashMap::new();
    for config in backend_configs {
        sovereign_flags.insert(config.name.clone(), config.sovereign);
        fleet_meta.insert(
            config.name.clone(),
            (
                config.base_url.clone(),
                config.context_size,
                config.model.clone(),
                config.api_key.clone(),
                config.supports_tool_use,
            ),
        );
    }

    let proxy_targets = Arc::new(ProxyTargets::from_fleet_meta(&fleet_meta, &sovereign_flags));
    if !proxy_targets.available().is_empty() {
        crate::klog!(
            "[Ring 2] Soma L4 proxy: {} sovereign backend(s): {:?}",
            proxy_targets.available().len(),
            proxy_targets.available()
        );
    }

    let (event_tx, _) = tokio::sync::broadcast::channel::<domain::events::KernelEvent>(256);

    let mail = match backends::mail::AgentmailConfig::from_env("MAIL_USERNAME") {
        Some(config) => {
            let backend = backends::mail::AgentmailBackend::new(config);
            crate::klog!("[Ring 2] Mail service: agentmail.to configured");
            Some(Arc::new(backend) as Arc<dyn domain::mail::MailPort>)
        }
        None => {
            crate::klog!(
                "[Ring 2] Mail service: AGENTMAIL_API_KEY or MAIL_USERNAME not configured — disabled"
            );
            None
        }
    };

    let zones = {
        match selected_zones_path() {
            Some(path) => {
                let zones_str = path.display().to_string();
                match domain::zones::ZoneConfig::from_file(&zones_str) {
                    Ok(zones) => {
                        crate::klog!("[Ring 2] Zones: loaded {} zone(s)", zones.zone_count());
                        Arc::new(zones)
                    }
                    Err(error) => {
                        crate::klog!("[Ring 2] Zones: {error} — zone activity disabled");
                        Arc::new(domain::zones::ZoneConfig::default())
                    }
                }
            }
            None => {
                crate::klog!(
                    "[Ring 2] Zones: no configured zone manifest — zone activity disabled"
                );
                Arc::new(domain::zones::ZoneConfig::default())
            }
        }
    };

    RuntimeServices {
        domain_curations,
        enricher,
        domain_router,
        dog_perf_collector,
        proxy_targets,
        event_tx,
        mail,
        zones,
    }
}

fn selected_curation_dir() -> std::path::PathBuf {
    std::env::var("CYNIC_CURATION_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/etc/cynic/curation"))
}

fn selected_zones_path() -> Option<std::path::PathBuf> {
    std::env::var("CYNIC_ZONES_PATH")
        .ok()
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            let fallback = std::path::PathBuf::from("/etc/cynic/zones.json");
            if fallback.is_file() {
                Some(fallback)
            } else {
                None
            }
        })
}
