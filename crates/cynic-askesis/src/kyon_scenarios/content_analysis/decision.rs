use super::{
    AnalysisDisposition, AnalysisStage, ContentAnalysisPolicy, ContentAnalysisResult,
    ContentCascadeDecision,
};

pub fn decide_content_analysis(
    policy: &ContentAnalysisPolicy,
    results: &[ContentAnalysisResult],
) -> ContentCascadeDecision {
    let mut blockers = validate(policy, results);
    if !blockers.is_empty() {
        return ContentCascadeDecision {
            status: "blocked".to_string(),
            accepted_stage: None,
            next_stage: None,
            disposition: AnalysisDisposition::Unknown,
            blockers,
            residual_unknowns: collect_unknowns(results),
        };
    }

    let latest = results.last();
    let acceptable = latest.is_some_and(|result| {
        result.disposition != AnalysisDisposition::Unknown
            && result.confidence_bps >= policy.target_confidence_bps
            && (!policy.require_calibrated_result || result.calibrated)
            && (!policy.stop_on_material_disagreement || !result.material_disagreement)
            && !result.evidence_refs.is_empty()
    });
    if acceptable {
        let result = latest.expect("latest result exists when acceptable");
        return ContentCascadeDecision {
            status: "sufficient".to_string(),
            accepted_stage: Some(result.stage),
            next_stage: None,
            disposition: result.disposition,
            blockers,
            residual_unknowns: collect_unknowns(results),
        };
    }

    let next_stage = next_stage(results.len());
    if next_stage.is_some_and(|stage| stage <= policy.maximum_stage) {
        ContentCascadeDecision {
            status: "escalate".to_string(),
            accepted_stage: None,
            next_stage,
            disposition: AnalysisDisposition::Unknown,
            blockers,
            residual_unknowns: collect_unknowns(results),
        }
    } else {
        blockers.push("confidence_need_not_met_within_user_boundary".to_string());
        ContentCascadeDecision {
            status: "inconclusive".to_string(),
            accepted_stage: None,
            next_stage: None,
            disposition: AnalysisDisposition::Unknown,
            blockers,
            residual_unknowns: collect_unknowns(results),
        }
    }
}

fn validate(policy: &ContentAnalysisPolicy, results: &[ContentAnalysisResult]) -> Vec<String> {
    let mut blockers = Vec::new();
    if policy.target_confidence_bps > 10_000 {
        blockers.push("target_confidence_out_of_range".to_string());
    }
    for (index, result) in results.iter().enumerate() {
        if AnalysisStage::ORDERED.get(index).copied() != Some(result.stage) {
            blockers.push(format!("analysis_stage_skipped_or_reordered:{index}"));
        }
        if result.stage > policy.maximum_stage {
            blockers.push(format!(
                "analysis_exceeded_user_boundary:{:?}",
                result.stage
            ));
        }
        if result.confidence_bps > 10_000 {
            blockers.push(format!("confidence_out_of_range:{:?}", result.stage));
        }
    }
    blockers
}

fn next_stage(completed: usize) -> Option<AnalysisStage> {
    AnalysisStage::ORDERED.get(completed).copied()
}

fn collect_unknowns(results: &[ContentAnalysisResult]) -> Vec<String> {
    let mut unknowns = results
        .iter()
        .flat_map(|result| result.residual_unknowns.iter().cloned())
        .collect::<Vec<_>>();
    unknowns.sort();
    unknowns.dedup();
    unknowns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(maximum_stage: AnalysisStage) -> ContentAnalysisPolicy {
        ContentAnalysisPolicy {
            target_confidence_bps: 9_000,
            maximum_stage,
            require_calibrated_result: true,
            stop_on_material_disagreement: true,
        }
    }

    fn result(stage: AnalysisStage, confidence_bps: u16) -> ContentAnalysisResult {
        ContentAnalysisResult {
            stage,
            disposition: AnalysisDisposition::LikelyOutsidePolicy,
            confidence_bps,
            calibrated: true,
            material_disagreement: false,
            labels: vec!["sexual_content".to_string()],
            evidence_refs: vec![format!("local:{stage:?}")],
            residual_unknowns: vec!["sender_intent".to_string()],
        }
    }

    #[test]
    fn starts_with_heuristics() {
        let decision = decide_content_analysis(&policy(AnalysisStage::LocalMultimodal), &[]);

        assert_eq!(decision.status, "escalate");
        assert_eq!(decision.next_stage, Some(AnalysisStage::Heuristic));
    }

    #[test]
    fn sufficient_heuristic_stops_the_cascade() {
        let decision = decide_content_analysis(
            &policy(AnalysisStage::LocalMultimodal),
            &[result(AnalysisStage::Heuristic, 9_400)],
        );

        assert_eq!(decision.status, "sufficient");
        assert_eq!(decision.accepted_stage, Some(AnalysisStage::Heuristic));
        assert_eq!(decision.next_stage, None);
    }

    #[test]
    fn insufficient_results_escalate_one_stage_at_a_time() {
        let decision = decide_content_analysis(
            &policy(AnalysisStage::LocalMultimodal),
            &[
                result(AnalysisStage::Heuristic, 4_000),
                result(AnalysisStage::ClassicalMl, 7_500),
            ],
        );

        assert_eq!(decision.status, "escalate");
        assert_eq!(decision.next_stage, Some(AnalysisStage::LocalModel));
    }

    #[test]
    fn user_maximum_is_an_inviolable_boundary() {
        let decision = decide_content_analysis(
            &policy(AnalysisStage::ClassicalMl),
            &[
                result(AnalysisStage::Heuristic, 4_000),
                result(AnalysisStage::ClassicalMl, 7_500),
            ],
        );

        assert_eq!(decision.status, "inconclusive");
        assert_eq!(decision.next_stage, None);
        assert_eq!(decision.disposition, AnalysisDisposition::Unknown);
    }

    #[test]
    fn stage_skipping_is_blocked() {
        let decision = decide_content_analysis(
            &policy(AnalysisStage::LocalMultimodal),
            &[result(AnalysisStage::LocalModel, 9_900)],
        );

        assert_eq!(decision.status, "blocked");
        assert_eq!(decision.accepted_stage, None);
    }

    #[test]
    fn high_confidence_disagreement_is_not_silently_accepted() {
        let mut heuristic = result(AnalysisStage::Heuristic, 9_900);
        heuristic.material_disagreement = true;

        let decision = decide_content_analysis(&policy(AnalysisStage::Heuristic), &[heuristic]);

        assert_eq!(decision.status, "inconclusive");
        assert_eq!(decision.disposition, AnalysisDisposition::Unknown);
    }
}
