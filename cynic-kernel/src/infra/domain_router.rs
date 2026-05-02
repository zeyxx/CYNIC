//! Domain-aware Dog routing — selects suitable Dogs based on domain hints.
//!
//! Initialized at boot from backend_configs. Maps domain name → suitable Dog IDs.
//! If a domain has no suitable Dogs configured, returns all available Dogs (fallback).

use crate::infra::config::BackendConfig;
use std::collections::HashMap;

/// Routes inferences to suitable Dogs based on domain hint.
/// Built from backend configurations at boot time.
#[derive(Debug, Clone)]
pub struct DomainRouter {
    /// Map: domain name → vec of suitable Dog names
    /// Empty vec for a domain = use all Dogs (default)
    domain_to_dogs: HashMap<String, Vec<String>>,
    /// All available Dog names (for fallback)
    all_dogs: Vec<String>,
}

impl DomainRouter {
    /// Build router from backend configs.
    /// Dogs with empty suitable_for_domains are always available (fallback for all domains).
    /// Dogs with specific suitable_for_domains are only included for those domains.
    /// Always includes deterministic-dog (the built-in fast heuristic Dog).
    pub fn from_backends(backend_configs: &[BackendConfig]) -> Self {
        let mut all_dogs: Vec<String> =
            backend_configs.iter().map(|cfg| cfg.name.clone()).collect();
        // Always include deterministic-dog (built-in, fast, free)
        all_dogs.insert(0, "deterministic-dog".to_string());
        let mut domain_to_dogs: HashMap<String, Vec<String>> = HashMap::new();

        // Pass 1: Collect dogs suitable for each domain
        for cfg in backend_configs {
            if cfg.suitable_for_domains.is_empty() {
                // Dog with empty suitable_for_domains is a fallback (available for all domains)
                continue;
            }
            // Dog is suitable for listed domains
            for domain in &cfg.suitable_for_domains {
                domain_to_dogs
                    .entry(domain.clone())
                    .or_default()
                    .push(cfg.name.clone());
            }
        }

        Self {
            domain_to_dogs,
            all_dogs,
        }
    }

    /// Get suitable Dogs for a domain hint.
    /// If domain has configured Dogs, return those.
    /// Otherwise return all available Dogs (fallback).
    pub fn dogs_for_domain(&self, domain_hint: &str) -> Vec<String> {
        self.domain_to_dogs
            .get(domain_hint)
            .cloned()
            .unwrap_or_else(|| self.all_dogs.clone())
    }

    /// Check if a domain has specific Dog constraints.
    /// False = use all Dogs (default/fallback behavior).
    pub fn has_constraints(&self, domain_hint: &str) -> bool {
        self.domain_to_dogs.contains_key(domain_hint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::{AuthStyle, BackendType, PromptTier};

    fn make_config(name: &str, domains: Vec<String>) -> BackendConfig {
        BackendConfig {
            name: name.to_string(),
            backend_type: BackendType::OpenAi,
            base_url: "http://localhost:8080/v1".to_string(),
            api_key: None,
            model: "test".to_string(),
            auth_style: AuthStyle::None,
            context_size: 4096,
            timeout_secs: 30,
            max_tokens: 2048,
            temperature: 0.3,
            disable_thinking: false,
            json_mode: false,
            prompt_tier: PromptTier::Full,
            cost_input_per_mtok: 0.0,
            cost_output_per_mtok: 0.0,
            health_url: None,
            remediation: None,
            fleet_node: None,
            cli_extra_args: vec![],
            latency_ms: 0,
            suitable_for_domains: domains,
        }
    }

    #[test]
    fn test_empty_config() {
        let router = DomainRouter::from_backends(&[]);
        // Even with no backend config, deterministic-dog is always available
        let dogs = router.dogs_for_domain("token");
        assert_eq!(dogs, vec!["deterministic-dog".to_string()]);
    }

    #[test]
    fn test_fallback_for_unconfigured_domain() {
        let configs = vec![
            make_config("dog1", vec!["token".to_string()]),
            make_config("dog2", vec![]),
        ];
        let router = DomainRouter::from_backends(&configs);
        // domain "token" has explicit config → should return ["dog1"]
        assert_eq!(router.dogs_for_domain("token"), vec!["dog1"]);
        // domain "chess" has no config → fallback to all Dogs (including deterministic-dog)
        let chess_dogs = router.dogs_for_domain("chess");
        assert_eq!(chess_dogs.len(), 3); // deterministic-dog + dog1 + dog2
        assert!(chess_dogs.contains(&"deterministic-dog".to_string()));
        assert!(chess_dogs.contains(&"dog1".to_string()));
        assert!(chess_dogs.contains(&"dog2".to_string()));
    }

    #[test]
    fn test_multiple_domains_per_dog() {
        let configs = vec![make_config(
            "powerful",
            vec!["chess".to_string(), "reasoning".to_string()],
        )];
        let router = DomainRouter::from_backends(&configs);
        assert_eq!(router.dogs_for_domain("chess"), vec!["powerful"]);
        assert_eq!(router.dogs_for_domain("reasoning"), vec!["powerful"]);
        // unconfigured domain → fallback to all (deterministic-dog + powerful)
        let token_dogs = router.dogs_for_domain("token");
        assert_eq!(token_dogs.len(), 2);
        assert!(token_dogs.contains(&"deterministic-dog".to_string()));
        assert!(token_dogs.contains(&"powerful".to_string()));
    }
}
