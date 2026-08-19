use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};

/// Boot-time inference proxy routing table.
///
/// Only sovereign HTTP backends are exposed through the proxy surface. The
/// kernel may transport requests to selected targets, but selection law belongs
/// to cynic-inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyTargets {
    targets: BTreeMap<String, ProxyTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyTarget {
    pub base_url: String,
    pub api_key: Option<String>,
    pub supports_tool_use: bool,
    pub model_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedProxyTarget {
    pub backend_id: String,
    pub target: ProxyTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationIntent {
    FastChat,
    StructuredJson,
    CoTReasoning,
    CodeAudit,
    DeepSynthesis,
}

impl OperationIntent {
    pub fn preferred_backend_alias(&self) -> &'static str {
        match self {
            Self::FastChat | Self::StructuredJson => "minicpm-1b",
            Self::CoTReasoning => "vibethinker-3b",
            Self::CodeAudit => "qwythos-9b",
            Self::DeepSynthesis => "bonsai-27b",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyTargetSelectionError {
    BackendNotFound {
        requested: String,
        available: Vec<String>,
    },
    NoSovereignBackends,
}

impl ProxyTargets {
    pub fn from_fleet_meta(
        fleet_meta: &HashMap<String, (String, u32, String, Option<String>, bool)>,
        sovereign_flags: &HashMap<String, bool>,
    ) -> Self {
        let mut targets = BTreeMap::new();
        for (backend_id, (base_url, _ctx, model_name, api_key, supports_tool_use)) in fleet_meta {
            if sovereign_flags.get(backend_id).copied().unwrap_or(false) {
                targets.insert(
                    backend_id.clone(),
                    ProxyTarget {
                        base_url: base_url.clone(),
                        api_key: api_key.clone(),
                        supports_tool_use: *supports_tool_use,
                        model_name: model_name.clone(),
                    },
                );
            }
        }
        Self { targets }
    }

    pub fn available(&self) -> Vec<String> {
        self.targets.keys().cloned().collect()
    }

    pub fn entries(&self) -> Vec<(String, ProxyTarget)> {
        self.targets
            .iter()
            .map(|(backend_id, target)| (backend_id.clone(), target.clone()))
            .collect()
    }

    pub fn select_by_intent(
        &self,
        intent: OperationIntent,
        requires_tools: bool,
    ) -> Result<SelectedProxyTarget, ProxyTargetSelectionError> {
        let preferred = intent.preferred_backend_alias();
        if let Ok(target) = self.select(Some(preferred), None, requires_tools) {
            return Ok(target);
        }
        self.select(None, None, requires_tools)
    }

    pub fn select(
        &self,
        requested_backend: Option<&str>,
        requested_model: Option<&str>,
        requires_tools: bool,
    ) -> Result<SelectedProxyTarget, ProxyTargetSelectionError> {
        if let Some(requested) = requested_backend {
            return self
                .targets
                .get(requested)
                .cloned()
                .map(|target| SelectedProxyTarget {
                    backend_id: requested.to_string(),
                    target,
                })
                .ok_or_else(|| ProxyTargetSelectionError::BackendNotFound {
                    requested: requested.to_string(),
                    available: self.available(),
                });
        }

        if let Some(requested) = requested_model {
            let requested_model = requested.to_ascii_lowercase();
            return self
                .targets
                .iter()
                .find(|(_, target)| target.model_name.eq_ignore_ascii_case(&requested_model))
                .map(|(backend_id, target)| SelectedProxyTarget {
                    backend_id: backend_id.clone(),
                    target: target.clone(),
                })
                .ok_or_else(|| ProxyTargetSelectionError::BackendNotFound {
                    requested: requested.to_string(),
                    available: self.available(),
                });
        }

        let selected = if requires_tools {
            self.first_with_tools().or_else(|| self.first())
        } else {
            self.first()
        };

        selected
            .map(|(backend_id, target)| SelectedProxyTarget {
                backend_id: backend_id.to_string(),
                target: target.clone(),
            })
            .ok_or(ProxyTargetSelectionError::NoSovereignBackends)
    }

    fn first(&self) -> Option<(&str, &ProxyTarget)> {
        self.targets
            .iter()
            .next()
            .map(|(backend_id, target)| (backend_id.as_str(), target))
    }

    fn first_with_tools(&self) -> Option<(&str, &ProxyTarget)> {
        self.targets
            .iter()
            .find(|(_, target)| target.supports_tool_use)
            .map(|(backend_id, target)| (backend_id.as_str(), target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet_entry(
        base_url: &str,
        model_name: &str,
        supports_tool_use: bool,
    ) -> (String, u32, String, Option<String>, bool) {
        (
            base_url.to_string(),
            32768,
            model_name.to_string(),
            None,
            supports_tool_use,
        )
    }

    #[test]
    fn proxy_targets_select_by_intent_prefers_calibrated_backend() {
        let mut fleet = HashMap::new();
        fleet.insert("minicpm-1b".to_string(), fleet_entry("http://127.0.0.1:8082/v1", "MiniCPM", true));
        fleet.insert("vibethinker-3b".to_string(), fleet_entry("http://127.0.0.1:8087/v1", "VibeThinker", true));
        fleet.insert("qwythos-9b".to_string(), fleet_entry("http://127.0.0.1:8080/v1", "Qwythos", true));

        let mut sovereign = HashMap::new();
        sovereign.insert("minicpm-1b".to_string(), true);
        sovereign.insert("vibethinker-3b".to_string(), true);
        sovereign.insert("qwythos-9b".to_string(), true);

        let targets = ProxyTargets::from_fleet_meta(&fleet, &sovereign);

        let fast_target = targets.select_by_intent(OperationIntent::FastChat, false).unwrap();
        assert_eq!(fast_target.backend_id, "minicpm-1b");

        let cot_target = targets.select_by_intent(OperationIntent::CoTReasoning, false).unwrap();
        assert_eq!(cot_target.backend_id, "vibethinker-3b");

        let audit_target = targets.select_by_intent(OperationIntent::CodeAudit, false).unwrap();
        assert_eq!(audit_target.backend_id, "qwythos-9b");
    }

    #[test]
    fn proxy_targets_from_fleet_meta_keeps_only_sovereign_backends() {
        let mut fleet = HashMap::new();
        fleet.insert(
            "qwen25-7b-core".to_string(),
            fleet_entry("http://10.0.0.1:8080/v1", "Qwen2.5-7B-Instruct", true),
        );
        fleet.insert(
            "cloud-dog".to_string(),
            (
                "https://api.cloud.com/v1".to_string(),
                128000,
                "gpt-4".to_string(),
                Some("sk-xxx".to_string()),
                false,
            ),
        );

        let mut sovereign = HashMap::new();
        sovereign.insert("qwen25-7b-core".to_string(), true);
        sovereign.insert("cloud-dog".to_string(), false);

        let targets = ProxyTargets::from_fleet_meta(&fleet, &sovereign);

        assert_eq!(targets.available(), vec!["qwen25-7b-core"]);
        assert_eq!(
            targets
                .select(Some("cloud-dog"), None, false)
                .expect_err("cloud backend must not be selectable"),
            ProxyTargetSelectionError::BackendNotFound {
                requested: "cloud-dog".to_string(),
                available: vec!["qwen25-7b-core".to_string()]
            }
        );
    }
}
