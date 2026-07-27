use super::{AnalysisCapability, AnalysisLocality, AnalysisModality, AnalysisStage, InputExposure};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzerSelectionPolicy {
    pub modality: AnalysisModality,
    pub required_stage: AnalysisStage,
    pub maximum_input_exposure: InputExposure,
    pub maximum_locality: AnalysisLocality,
    pub maximum_cost_units: u32,
    pub maximum_latency_ms: u32,
    pub require_calibration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzerSelection {
    pub status: &'static str,
    pub selected_capability_id: Option<String>,
    pub blockers: Vec<String>,
    pub rejected_capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

pub fn select_analyzer(
    policy: &AnalyzerSelectionPolicy,
    capabilities: &[AnalysisCapability],
) -> AnalyzerSelection {
    let mut rejected = Vec::new();
    let mut candidates = capabilities
        .iter()
        .filter(|capability| {
            let reasons = rejection_reasons(policy, capability);
            if reasons.is_empty() {
                true
            } else {
                rejected.push(format!("{}:{}", capability.id, reasons.join(",")));
                false
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|capability| {
        (
            capability.locality,
            capability.input_exposure,
            capability.estimated_cost_units,
            capability.estimated_latency_ms,
            capability.id.as_str(),
        )
    });
    let selected = candidates.first().copied();

    AnalyzerSelection {
        status: if selected.is_some() {
            "selected"
        } else {
            "blocked"
        },
        selected_capability_id: selected.map(|capability| capability.id.clone()),
        blockers: if selected.is_some() {
            Vec::new()
        } else {
            vec!["no_compatible_analysis_capability".to_string()]
        },
        rejected_capabilities: rejected,
        evidence_refs: selected
            .map(|capability| capability.availability_evidence_refs.clone())
            .unwrap_or_default(),
        residual_unknowns: selected
            .map(|capability| capability.residual_unknowns.clone())
            .unwrap_or_else(|| vec!["analysis_capability_coverage".to_string()]),
    }
}

fn rejection_reasons(
    policy: &AnalyzerSelectionPolicy,
    capability: &AnalysisCapability,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if capability.stage != policy.required_stage {
        reasons.push("stage_mismatch");
    }
    if !capability.modalities.contains(&policy.modality) {
        reasons.push("modality_unsupported");
    }
    if capability.input_exposure > policy.maximum_input_exposure {
        reasons.push("input_exposure_exceeds_boundary");
    }
    if capability.locality > policy.maximum_locality {
        reasons.push("locality_exceeds_boundary");
    }
    if capability.estimated_cost_units > policy.maximum_cost_units {
        reasons.push("cost_exceeds_boundary");
    }
    if capability.estimated_latency_ms > policy.maximum_latency_ms {
        reasons.push("latency_exceeds_boundary");
    }
    if policy.require_calibration
        && capability
            .calibration_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        reasons.push("calibration_missing");
    }
    if !capability.available || capability.availability_evidence_refs.is_empty() {
        reasons.push("availability_not_proved");
    }
    if capability.evidence_kinds.is_empty() {
        reasons.push("evidence_contract_missing");
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> AnalyzerSelectionPolicy {
        AnalyzerSelectionPolicy {
            modality: AnalysisModality::Image,
            required_stage: AnalysisStage::Heuristic,
            maximum_input_exposure: InputExposure::LocalRaw,
            maximum_locality: AnalysisLocality::OnDevice,
            maximum_cost_units: 20,
            maximum_latency_ms: 50,
            require_calibration: true,
        }
    }

    fn capability(
        id: &str,
        exposure: InputExposure,
        cost: u32,
        available: bool,
    ) -> AnalysisCapability {
        AnalysisCapability {
            id: id.to_string(),
            stage: AnalysisStage::Heuristic,
            modalities: vec![AnalysisModality::Image],
            input_exposure: exposure,
            locality: AnalysisLocality::OnDevice,
            estimated_cost_units: cost,
            estimated_latency_ms: 10,
            calibration_ref: Some("calibration:v1".to_string()),
            evidence_kinds: vec!["typed_labels".to_string()],
            available,
            availability_evidence_refs: available
                .then(|| vec![format!("local:{id}:available")])
                .unwrap_or_default(),
            residual_unknowns: vec!["future_distribution_shift".to_string()],
        }
    }

    #[test]
    fn chooses_least_exposing_candidate_before_cost() {
        let candidates = vec![
            capability("raw-cheap", InputExposure::LocalRaw, 1, true),
            capability("derived", InputExposure::DerivedFeatures, 10, true),
        ];

        let selection = select_analyzer(&policy(), &candidates);

        assert_eq!(selection.status, "selected");
        assert_eq!(selection.selected_capability_id.as_deref(), Some("derived"));
    }

    #[test]
    fn unavailable_candidate_is_not_assumed_usable() {
        let candidates = vec![capability(
            "not-observed",
            InputExposure::EnvelopeOnly,
            1,
            false,
        )];

        let selection = select_analyzer(&policy(), &candidates);

        assert_eq!(selection.status, "blocked");
        assert!(selection.rejected_capabilities[0].contains("availability_not_proved"));
    }

    #[test]
    fn external_analyzer_cannot_cross_an_on_device_boundary() {
        let mut external = capability("external", InputExposure::DerivedFeatures, 1, true);
        external.locality = AnalysisLocality::External;

        let selection = select_analyzer(&policy(), &[external]);

        assert_eq!(selection.status, "blocked");
        assert!(selection.rejected_capabilities[0].contains("locality_exceeds_boundary"));
    }
}
