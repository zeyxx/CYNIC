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

use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_protocol::llm_port::*;

/// Operator agent that observes and acts on the organism.
pub struct OperatorAgent<R: ArgvRunner> {
    /// Path to the cynic CLI binary.
    cynic_bin: String,
    /// Argv runner for executing commands.
    runner: R,
}

impl OperatorAgent<LocalArgvRunner> {
    pub fn with_default_bin() -> Self {
        Self {
            cynic_bin: "cynic".to_string(),
            runner: LocalArgvRunner,
        }
    }
}

impl<R: ArgvRunner> OperatorAgent<R> {
    pub fn new(cynic_bin: String, runner: R) -> Self {
        Self { cynic_bin, runner }
    }

    /// Observe organism state via CLI.
    fn observe_state(&self) -> OperatorObservation {
        let branch = self.runner.run(&["git", "branch", "--show-current"]);
        let status = self.runner.run(&["git", "status", "--short"]);
        let log = self.runner.run(&["git", "log", "--oneline", "-5"]);

        OperatorObservation {
            schema: "cynic.operator_observation.v1".to_string(),
            current_branch: branch.stdout.trim().to_string(),
            git_status: status.stdout,
            recent_commits: log.stdout.lines().map(|l| l.to_string()).collect(),
            available_commands: vec![
                "cynic dev check".to_string(),
                "cynic dev build".to_string(),
                "cynic dev test".to_string(),
            ],
            active_claims: Vec::new(),
        }
    }
}

impl<R: ArgvRunner> OperatorAgentPort for OperatorAgent<R> {
    fn agent_name(&self) -> &'static str {
        "operator"
    }

    fn observe(&self) -> OperatorObservation {
        self.observe_state()
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
        let mut args: Vec<&str> = vec![];
        args.push(action.command.as_str());
        for arg in &action.args {
            args.push(arg.as_str());
        }

        let output = self.runner.run(&args);

        OperatorProof {
            schema: "cynic.operator_proof.v1".to_string(),
            action: action.clone(),
            success: output.exit_code == Some(0) && output.spawn_error.is_none(),
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
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
    use crate::update_lifecycle::MapArgvRunner;

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

    fn test_runner() -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![
                (
                    vec!["git".to_string(), "branch".to_string(), "--show-current".to_string()],
                    crate::update_lifecycle::ArgvOutput {
                        exit_code: Some(0),
                        stdout: "main".to_string(),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
                (
                    vec!["git".to_string(), "status".to_string(), "--short".to_string()],
                    crate::update_lifecycle::ArgvOutput {
                        exit_code: Some(0),
                        stdout: " M file.rs".to_string(),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
                (
                    vec!["git".to_string(), "log".to_string(), "--oneline".to_string(), "-5".to_string()],
                    crate::update_lifecycle::ArgvOutput {
                        exit_code: Some(0),
                        stdout: "abc123 commit 1\ndef456 commit 2".to_string(),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
            ],
        }
    }

    #[test]
    fn operator_agent_name() {
        let agent = OperatorAgent::with_default_bin();
        assert_eq!(agent.agent_name(), "operator");
    }

    #[test]
    fn operator_agent_observe() {
        let agent = OperatorAgent::new("cynic".to_string(), test_runner());
        let obs = agent.observe();
        assert_eq!(obs.schema, "cynic.operator_observation.v1");
        assert_eq!(obs.current_branch, "main");
        assert!(!obs.available_commands.is_empty());
    }

    #[test]
    fn operator_agent_propose_with_fake_llm() {
        let agent = OperatorAgent::new("cynic".to_string(), test_runner());
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
