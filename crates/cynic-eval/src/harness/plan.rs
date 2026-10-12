use super::types::{ScenarioSpec, TaskSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioPlan {
    pub scenario: ScenarioSpec,
    pub task_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkPlan {
    pub schema: String,
    pub plan_id: String,
    pub scenarios: Vec<ScenarioPlan>,
}

impl BenchmarkPlan {
    pub fn new(plan_id: impl Into<String>, scenarios: Vec<ScenarioSpec>) -> Self {
        let scenarios = scenarios
            .into_iter()
            .map(|scenario| {
                let task_order = scenario
                    .tasks
                    .iter()
                    .map(|task: &TaskSpec| task.task_id.clone())
                    .collect();
                ScenarioPlan {
                    scenario,
                    task_order,
                }
            })
            .collect();
        Self {
            schema: "cynic.agent_harness.plan.v1".into(),
            plan_id: plan_id.into(),
            scenarios,
        }
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != "cynic.agent_harness.plan.v1" {
            errors.push(format!("schema_mismatch:{}", self.schema));
        }
        if self.plan_id.trim().is_empty() {
            errors.push("empty_plan_id".into());
        }
        if self.scenarios.is_empty() {
            errors.push("empty_scenarios".into());
        }
        let mut scenario_ids = std::collections::BTreeSet::new();
        for planned in &self.scenarios {
            let scenario = &planned.scenario;
            if scenario.scenario_id.trim().is_empty() {
                errors.push("empty_scenario_id".into());
            }
            if scenario
                .runtime_id
                .as_deref()
                .is_some_and(|runtime_id| runtime_id.trim().is_empty())
            {
                errors.push(format!("empty_runtime_id:{}", scenario.scenario_id));
            }
            if scenario.model.role.trim().is_empty() {
                errors.push(format!("empty_model_role:{}", scenario.scenario_id));
            }
            let parameters = &scenario.model.parameters;
            if parameters.model_id.trim().is_empty() {
                errors.push(format!("empty_model_id:{}", scenario.scenario_id));
            }
            if parameters.model_path.trim().is_empty() {
                errors.push(format!("empty_model_path:{}", scenario.scenario_id));
            }
            if parameters.context_requested == 0 {
                errors.push(format!("zero_context_requested:{}", scenario.scenario_id));
            }
            if parameters.parallel_slots == 0 {
                errors.push(format!("zero_parallel_slots:{}", scenario.scenario_id));
            }
            if parameters.context_per_slot == Some(0) {
                errors.push(format!("zero_context_per_slot:{}", scenario.scenario_id));
            }
            if scenario.tasks.is_empty() {
                errors.push(format!("empty_tasks:{}", scenario.scenario_id));
            }
            if !scenario_ids.insert(scenario.scenario_id.clone()) {
                errors.push(format!("duplicate_scenario:{}", scenario.scenario_id));
            }
            let expected_order: Vec<&str> = scenario
                .tasks
                .iter()
                .map(|task| task.task_id.as_str())
                .collect();
            let actual_order: Vec<&str> = planned.task_order.iter().map(String::as_str).collect();
            if expected_order != actual_order {
                errors.push(format!("task_order_mismatch:{}", scenario.scenario_id));
            }
            let mut task_ids = std::collections::BTreeSet::new();
            for task in &scenario.tasks {
                if task.task_id.trim().is_empty() {
                    errors.push(format!("empty_task:{}", scenario.scenario_id));
                }
                if task.prompt.trim().is_empty() {
                    errors.push(format!(
                        "empty_task_prompt:{}:{}",
                        scenario.scenario_id, task.task_id
                    ));
                }
                if task.max_output_tokens == 0 {
                    errors.push(format!(
                        "zero_max_output_tokens:{}:{}",
                        scenario.scenario_id, task.task_id
                    ));
                }
                if task.expected_proof.trim().is_empty() {
                    errors.push(format!(
                        "empty_expected_proof:{}:{}",
                        scenario.scenario_id, task.task_id
                    ));
                }
                if !task_ids.insert(task.task_id.clone()) {
                    errors.push(format!(
                        "duplicate_task:{}:{}",
                        scenario.scenario_id, task.task_id
                    ));
                }
            }
            if let Some(context_per_slot) = scenario.model.parameters.context_per_slot {
                let aggregate = u64::from(context_per_slot)
                    .saturating_mul(u64::from(scenario.model.parameters.parallel_slots));
                if aggregate > u64::from(scenario.model.parameters.context_requested) {
                    errors.push(format!("context_budget_exceeded:{}", scenario.scenario_id));
                }
            }
            if scenario
                .thresholds
                .min_available_memory_percent
                .is_some_and(|value| !(0.0..=100.0).contains(&value))
            {
                errors.push(format!(
                    "invalid_min_memory_percent:{}",
                    scenario.scenario_id
                ));
            }
            if scenario
                .thresholds
                .max_rss_percent
                .is_some_and(|value| !(0.0..=100.0).contains(&value))
            {
                errors.push(format!("invalid_max_rss_percent:{}", scenario.scenario_id));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_context_budget_overflow() {
        let scenario: ScenarioSpec = serde_json::from_value(serde_json::json!({
            "scenario_id":"s",
            "model":{"role":"worker","parameters":{"model_id":"m","model_path":"/m","context_requested":10,"parallel_slots":2,"context_per_slot":8}},
            "tasks":[],
            "thresholds":{}
        })).unwrap();
        let mut plan = BenchmarkPlan::new("p", vec![scenario]);
        plan.scenarios[0].task_order.clear();
        assert!(
            plan.validate()
                .unwrap_err()
                .iter()
                .any(|error| error == "context_budget_exceeded:s")
        );
    }

    #[test]
    fn rejects_empty_matrix_identity_and_workload() {
        let plan = BenchmarkPlan::new("", vec![]);
        let errors = plan.validate().unwrap_err();
        assert!(errors.iter().any(|error| error == "empty_plan_id"));
        assert!(errors.iter().any(|error| error == "empty_scenarios"));
    }

    #[test]
    fn rejects_unexecutable_task_and_model_declarations() {
        let scenario: ScenarioSpec = serde_json::from_value(serde_json::json!({
            "scenario_id":"invalid",
            "runtime_id":"   ",
            "model":{"role":" ","parameters":{"model_id":" ","model_path":" ","context_requested":0,"parallel_slots":0,"context_per_slot":0}},
            "tasks":[{"task_id":"task","kind":"audit","prompt":" ","max_output_tokens":0,"expected_proof":" "}],
            "thresholds":{}
        }))
        .unwrap();
        let errors = BenchmarkPlan::new("p", vec![scenario])
            .validate()
            .unwrap_err();
        for expected in [
            "empty_runtime_id:invalid",
            "empty_model_role:invalid",
            "empty_model_id:invalid",
            "empty_model_path:invalid",
            "zero_context_requested:invalid",
            "zero_parallel_slots:invalid",
            "zero_context_per_slot:invalid",
            "empty_task_prompt:invalid:task",
            "zero_max_output_tokens:invalid:task",
            "empty_expected_proof:invalid:task",
        ] {
            assert!(errors.iter().any(|error| error == expected), "{expected}");
        }
    }
}
