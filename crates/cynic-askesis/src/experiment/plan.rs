use crate::error::AskesisError;
use crate::experiment::types::{DimensionSpec, ExperimentSpec, TrialPlan};

pub fn plan_trials(spec: &ExperimentSpec) -> crate::Result<Vec<TrialPlan>> {
    validate_spec(spec)?;
    let coordinates = cartesian_coordinates(&spec.dimensions);
    let mut plans = Vec::new();
    for domain in &spec.domains {
        for coordinate in &coordinates {
            for scale in &spec.scales {
                for repetition in 0..spec.repetitions {
                    let coordinate_id = coordinate
                        .iter()
                        .map(|(dimension, value)| format!("{dimension}={value}"))
                        .collect::<Vec<_>>()
                        .join(",");
                    plans.push(TrialPlan {
                        schema: "cynic.askesis.experiment.trial_plan.v1",
                        experiment_id: spec.id.clone(),
                        trial_id: format!(
                            "{}:{}:{}:{}:{}",
                            spec.id, domain.id, coordinate_id, scale.id, repetition
                        ),
                        domain: domain.clone(),
                        coordinates: coordinate.clone(),
                        scale: scale.clone(),
                        repetition,
                        mutates: false,
                        status: "planned",
                    });
                }
            }
        }
    }
    Ok(plans)
}

fn validate_spec(spec: &ExperimentSpec) -> crate::Result<()> {
    if spec.schema != "cynic.askesis.experiment.spec.v1" {
        return Err(AskesisError::EvalPolicy(
            "unsupported experiment schema".into(),
        ));
    }
    if spec.id.trim().is_empty()
        || spec.domains.is_empty()
        || spec.scales.is_empty()
        || spec.repetitions == 0
    {
        return Err(AskesisError::EvalPolicy(
            "experiment id, domains, scales, and repetitions must be non-empty".into(),
        ));
    }
    if spec.dimensions.iter().any(|axis| axis.values.is_empty())
        || spec.scales.iter().any(|scale| scale.sample_count == 0)
    {
        return Err(AskesisError::EvalPolicy(
            "dimension values and scale sample counts must be non-empty".into(),
        ));
    }
    Ok(())
}

fn cartesian_coordinates(dimensions: &[DimensionSpec]) -> Vec<Vec<(String, String)>> {
    dimensions
        .iter()
        .fold(vec![Vec::new()], |coordinates, dimension| {
            coordinates
                .into_iter()
                .flat_map(|coordinate| {
                    dimension.values.iter().map(move |value| {
                        let mut next = coordinate.clone();
                        next.push((dimension.id.clone(), value.clone()));
                        next
                    })
                })
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::types::{DomainSpec, ScaleSpec};

    #[test]
    fn plans_the_domain_dimension_scale_product() {
        let spec = ExperimentSpec {
            schema: "cynic.askesis.experiment.spec.v1".into(),
            id: "small-model-capabilities".into(),
            domains: vec![DomainSpec {
                id: "wordle".into(),
                task_kind: "interactive_search".into(),
            }],
            dimensions: vec![DimensionSpec {
                id: "prompt".into(),
                values: vec!["direct".into(), "reasoning".into()],
            }],
            scales: vec![ScaleSpec {
                id: "smoke".into(),
                sample_count: 8,
            }],
            repetitions: 3,
        };
        let plans = plan_trials(&spec).expect("valid plan");
        assert_eq!(plans.len(), 6);
        assert!(plans.iter().all(|plan| !plan.mutates));
    }
}
