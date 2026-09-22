use std::collections::HashMap;
use std::sync::Arc;

use crate::claim::InMemoryClaimStore;
use crate::ledger::InMemoryLedger;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct TurnResult {
    pub agent: String,
    pub output: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CollaborationRequest {
    pub task_id: String,
    pub initial_prompt: String,
    pub starting_agent: String,
    pub max_turns: u32,
    pub deadline_ms: u64,
    pub stagnation_window: usize,
}

#[derive(Debug, Clone)]
pub enum CollaborationOutcome {
    Completed {
        final_output: String,
        turns: Vec<TurnResult>,
    },
    Blocked {
        reason: BlockReason,
        turns: Vec<TurnResult>,
    },
    Failed {
        error: String,
        failed_agent: String,
        turns: Vec<TurnResult>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    MaxTurns,
    DeadlineExceeded,
    Stagnation,
    BudgetExhausted,
}

pub struct Orchestrator {
    ledger: Arc<InMemoryLedger>,
    claim_store: Arc<InMemoryClaimStore>,
    agents: HashMap<String, String>,
}

impl Orchestrator {
    pub fn new(ledger: Arc<InMemoryLedger>, claim_store: Arc<InMemoryClaimStore>) -> Self {
        Self {
            ledger,
            claim_store,
            agents: HashMap::new(),
        }
    }

    pub fn register_agent(&mut self, name: &str, endpoint: &str) {
        self.agents.insert(name.to_string(), endpoint.to_string());
    }

    fn check_guardrails(
        &self,
        options: &GuardrailOptions,
        state: &GuardrailState,
    ) -> Option<BlockReason> {
        let now = now_unix_ms();
        if now >= options.deadline_at_ms {
            return Some(BlockReason::DeadlineExceeded);
        }
        if Self::detect_stagnation(&state.outputs, options.stagnation_window) {
            return Some(BlockReason::Stagnation);
        }
        if state.turns_completed >= options.max_turns {
            return Some(BlockReason::MaxTurns);
        }
        None
    }

    pub fn detect_stagnation(outputs: &[String], window: usize) -> bool {
        if outputs.len() < window {
            return false;
        }
        let recent = &outputs[outputs.len().saturating_sub(window)..];
        let normalized: Vec<String> = recent.iter().map(|s| normalize_output(s)).collect();
        !normalized.is_empty()
            && normalized
                .iter()
                .all(|s| !s.is_empty() && s == &normalized[0])
    }

    async fn run_turn_stub(&self, agent: &str, prompt: &str) -> Result<TurnResult, String> {
        Ok(TurnResult {
            agent: agent.to_string(),
            output: format!(
                "[{}] {}",
                agent,
                prompt.chars().take(80).collect::<String>()
            ),
            session_id: None,
        })
    }

    pub async fn run_collaboration(&self, request: CollaborationRequest) -> CollaborationOutcome {
        let mut turns = Vec::new();
        let mut outputs = Vec::new();
        let mut current_agent = request.starting_agent.clone();

        let guardrail_options = GuardrailOptions {
            max_turns: request.max_turns,
            deadline_at_ms: now_unix_ms() + request.deadline_ms,
            stagnation_window: request.stagnation_window,
        };

        for turn_idx in 0..request.max_turns {
            let state = GuardrailState {
                turns_completed: turn_idx,
                outputs: outputs.clone(),
            };

            if let Some(reason) = self.check_guardrails(&guardrail_options, &state) {
                return CollaborationOutcome::Blocked { reason, turns };
            }

            let prompt = if turn_idx == 0 {
                request.initial_prompt.clone()
            } else {
                format!(
                    "Turn {} for task {}. Previous output:\n{}",
                    turn_idx,
                    request.task_id,
                    outputs.last().unwrap_or(&"".to_string())
                )
            };

            match self.run_turn_stub(&current_agent, &prompt).await {
                Ok(result) => {
                    outputs.push(result.output.clone());
                    turns.push(result);
                }
                Err(e) => {
                    return CollaborationOutcome::Failed {
                        error: e,
                        failed_agent: current_agent,
                        turns,
                    };
                }
            }

            current_agent = if current_agent == request.starting_agent {
                "reviewer".to_string()
            } else {
                request.starting_agent.clone()
            };
        }

        CollaborationOutcome::Completed {
            final_output: outputs.last().cloned().unwrap_or_default(),
            turns,
        }
    }
}

struct GuardrailOptions {
    max_turns: u32,
    deadline_at_ms: u64,
    stagnation_window: usize,
}

struct GuardrailState {
    turns_completed: u32,
    outputs: Vec<String>,
}

fn normalize_output(s: &str) -> String {
    s.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_stagnation_repeated_output() {
        let outputs = vec![
            "response A".to_string(),
            "response B".to_string(),
            "response A".to_string(),
            "response A".to_string(),
            "response A".to_string(),
        ];
        assert!(Orchestrator::detect_stagnation(&outputs, 3));
    }

    #[test]
    fn detect_stagnation_no_stagnation() {
        let outputs = vec![
            "response A".to_string(),
            "response B".to_string(),
            "response C".to_string(),
        ];
        assert!(!Orchestrator::detect_stagnation(&outputs, 3));
    }

    #[test]
    fn normalize_output_collapses_whitespace() {
        assert_eq!(normalize_output("  Hello   World  "), "hello world");
    }
}
