//! Operator agent — an LLM agent that operates CYNIC via CLI binaries.
//!
//! This is the first "pilot agent" of the organism. It:
//! 1. Observes organism state (git status, branches, claims)
//! 2. Proposes actions via LLM
//! 3. Executes proposed commands
//! 4. Produces proofs
//!
//! The LLM is a tool, never the authority. The agent decides what to ask,
//! whether to use the response, and when to escalate.

use cynic_protocol::llm_port::*;

/// Operator agent that observes and acts on the organism.
pub struct OperatorAgent {
    /// Path to the cynic CLI binary.
    cynic_bin: String,
}

impl OperatorAgent {
    pub fn new(cynic_bin: String) -> Self {
        Self { cynic_bin }
    }

    pub fn with_default_bin() -> Self {
        Self::new("cynic".to_string())
    }
}

impl OperatorAgentPort for OperatorAgent {
    fn agent_name(&self) -> &'static str {
        "operator"
    }

    fn observe(&self) -> OperatorObservation {
        // TODO: implement via ArgvRunner + cynic CLI
        OperatorObservation {
            schema: "cynic.operator_observation.v1".to_string(),
            current_branch: "unknown".to_string(),
            git_status: String::new(),
            recent_commits: Vec::new(),
            available_commands: vec![
                "cynic dev check".to_string(),
                "cynic dev build".to_string(),
                "cynic dev test".to_string(),
            ],
            active_claims: Vec::new(),
        }
    }

    fn propose_action(
        &self,
        observation: &OperatorObservation,
        llm: &dyn LlmPort,
    ) -> Option<OperatorAction> {
        let request = LlmRequest {
            schema: "cynic.llm_request.v1".to_string(),
            task: LlmTask::ProposeAction {
                current_state: format!("{:?}", observation),
                available_commands: observation.available_commands.clone(),
            },
            context: "You are the CYNIC operator agent. Propose the next action.".to_string(),
            output_format: "json".to_string(),
            max_tokens: 512,
            temperature: 0.3,
        };

        let response = llm.complete(&request)?;

        // Parse JSON response into OperatorAction
        // For now, return a placeholder
        Some(OperatorAction {
            schema: "cynic.operator_action.v1".to_string(),
            command: self.cynic_bin.clone(),
            args: vec!["dev".to_string(), "check".to_string()],
            rationale: response.output,
            confidence: 0.8,
            requires_approval: false,
        })
    }

    fn execute_action(&self, action: &OperatorAction) -> OperatorProof {
        // TODO: execute via ArgvRunner
        OperatorProof {
            schema: "cynic.operator_proof.v1".to_string(),
            action: action.clone(),
            success: false,
            stdout: String::new(),
            stderr: "not implemented".to_string(),
            exit_code: None,
            state_hash: None,
        }
    }

    fn can_handle(&self, _observation: &OperatorObservation) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLlm {
        response: String,
    }

    impl LlmPort for FakeLlm {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn is_available(&self) -> bool {
            true
        }

        fn complete(&self, _request: &LlmRequest) -> Option<LlmResponse> {
            Some(LlmResponse {
                schema: "cynic.llm_response.v1".to_string(),
                output: self.response.clone(),
                truncated: false,
                tokens_used: Some(10),
                model: "fake".to_string(),
            })
        }

        fn estimate_cost(&self, _request: &LlmRequest) -> LlmCostEstimate {
            LlmCostEstimate {
                tokens_estimate: 100,
                cost_usd_estimate: 0.0,
                energy_joules_estimate: 0.0,
            }
        }
    }

    #[test]
    fn operator_agent_name() {
        let agent = OperatorAgent::with_default_bin();
        assert_eq!(agent.agent_name(), "operator");
    }

    #[test]
    fn operator_agent_observe() {
        let agent = OperatorAgent::with_default_bin();
        let obs = agent.observe();
        assert_eq!(obs.schema, "cynic.operator_observation.v1");
        assert!(!obs.available_commands.is_empty());
    }

    #[test]
    fn operator_agent_propose_with_fake_llm() {
        let agent = OperatorAgent::with_default_bin();
        let llm = FakeLlm {
            response: "run cynic dev check".to_string(),
        };
        let obs = agent.observe();
        let action = agent.propose_action(&obs, &llm);
        assert!(action.is_some());
        let action = action.unwrap();
        assert_eq!(action.command, "cynic");
    }
}
