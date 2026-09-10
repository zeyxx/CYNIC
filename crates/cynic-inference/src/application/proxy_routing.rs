use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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

    /// Select a proxy target taking live slot utilization into account.
    /// Prefers targets with free slots, picking the one with lowest utilization.
    pub fn select_with_slot_tracker(
        &self,
        slot_tracker: &crate::domain::slot_tracker::SlotTracker,
        requested_backend: Option<&str>,
        requested_model: Option<&str>,
        requires_tools: bool,
    ) -> Result<SelectedProxyTarget, ProxyTargetSelectionError> {
        if requested_backend.is_some() || requested_model.is_some() {
            return self.select(requested_backend, requested_model, requires_tools);
        }

        let candidates = {
            let all = self.entries();
            if requires_tools {
                let tools: Vec<(String, ProxyTarget)> = all
                    .into_iter()
                    .filter(|(_, target)| target.supports_tool_use)
                    .collect();
                if tools.is_empty() {
                    self.entries()
                } else {
                    tools
                }
            } else {
                all
            }
        };

        if candidates.is_empty() {
            return Err(ProxyTargetSelectionError::NoSovereignBackends);
        }

        let slot_snapshot = slot_tracker.snapshot();

        let mut preferred: Vec<(String, ProxyTarget, f64)> = Vec::new();
        let mut fallback: Vec<(String, ProxyTarget, f64)> = Vec::new();

        for (backend_id, target) in candidates {
            let (has_free_slot, utilization) = match slot_snapshot.get(&backend_id) {
                Some(slots) if slots.total > 0 => (slots.busy < slots.total, slots.utilization()),
                Some(_slots) => (true, 1.0),
                None => (true, 0.5),
            };

            if has_free_slot {
                preferred.push((backend_id, target, utilization));
            } else {
                fallback.push((backend_id, target, utilization));
            }
        }

        let selected = if !preferred.is_empty() {
            preferred
        } else {
            fallback
        };

        let mut picked = None;
        for (backend_id, target, utilization) in selected {
            if picked
                .as_ref()
                .is_none_or(|(_, _, current)| utilization < *current)
            {
                picked = Some((backend_id, target, utilization));
            }
        }

        picked
            .map(|(backend_id, target, _)| SelectedProxyTarget { backend_id, target })
            .ok_or(ProxyTargetSelectionError::NoSovereignBackends)
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
        fleet.insert(
            "minicpm-1b".to_string(),
            fleet_entry("http://127.0.0.1:8082/v1", "MiniCPM", true),
        );
        fleet.insert(
            "vibethinker-3b".to_string(),
            fleet_entry("http://127.0.0.1:8087/v1", "VibeThinker", true),
        );
        fleet.insert(
            "qwythos-9b".to_string(),
            fleet_entry("http://127.0.0.1:8080/v1", "Qwythos", true),
        );

        let mut sovereign = HashMap::new();
        sovereign.insert("minicpm-1b".to_string(), true);
        sovereign.insert("vibethinker-3b".to_string(), true);
        sovereign.insert("qwythos-9b".to_string(), true);

        let targets = ProxyTargets::from_fleet_meta(&fleet, &sovereign);

        let fast_target = targets
            .select_by_intent(OperationIntent::FastChat, false)
            .unwrap();
        assert_eq!(fast_target.backend_id, "minicpm-1b");

        let cot_target = targets
            .select_by_intent(OperationIntent::CoTReasoning, false)
            .unwrap();
        assert_eq!(cot_target.backend_id, "vibethinker-3b");

        let audit_target = targets
            .select_by_intent(OperationIntent::CodeAudit, false)
            .unwrap();
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
