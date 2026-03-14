//! InferenceDog — model-agnostic Dog that uses any ChatPort for axiom evaluation.
//! ONE prompt template, N backends. The Dog never knows which model it's talking to.

use crate::dog::*;
use crate::chat_port::ChatPort;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

pub struct InferenceDog {
    chat: Arc<dyn ChatPort>,
    dog_name: String,
}

impl InferenceDog {
    pub fn new(chat: Arc<dyn ChatPort>, name: String) -> Self {
        Self { chat, dog_name: name }
    }

    fn build_system_prompt() -> &'static str {
        "You are CYNIC, a sovereign epistemic judge. You evaluate THE SUBJECT MATTER described in the stimulus — not the quality of its description. In chess, judge the MOVE or STRATEGY, not the text. In science, judge the CLAIM, not the writing. Your axioms measure the SUBSTANCE, not the FORM.\n\nBe harsh. Be honest. Overconfidence is the enemy. Most things deserve 0.3-0.6, not 0.8-0.9."
    }

    fn build_user_prompt(stimulus: &Stimulus) -> String {
        let context_block = stimulus.context.as_deref().unwrap_or("(no additional context)");
        let domain = stimulus.domain.as_deref().unwrap_or("general");

        format!(r#"DOMAIN: {domain}
STIMULUS: {content}
CONTEXT: {context_block}

Evaluate THE SUBJECT MATTER described (not the description). Think step by step about each axiom, then score from 0.0 to 1.0 with honest uncertainty.

AXIOMS:
1. FIDELITY — Is this faithful to truth? Does it reflect sound principles in its domain?
2. PHI — Is this structurally harmonious? Well-coordinated? Proportional?
3. VERIFY — Is it sound and testable? Can the idea be verified or refuted?
4. CULTURE — Does this honor existing traditions, conventions, and established patterns?
5. BURN — Is this efficient? Minimal waste? Could excess be destroyed without loss?
6. SOVEREIGNTY — Does this preserve individual agency and freedom of choice?

First reason about each axiom, then provide scores.

Respond ONLY with this exact JSON (no markdown, no explanation):
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "culture": 0.XX, "burn": 0.XX, "sovereignty": 0.XX, "fidelity_reason": "...", "phi_reason": "...", "verify_reason": "...", "culture_reason": "...", "burn_reason": "...", "sovereignty_reason": "..."}}"#,
            content = stimulus.content,
        )
    }
}

#[derive(Deserialize)]
struct AxiomResponse {
    fidelity: f64,
    phi: f64,
    verify: f64,
    #[serde(default)]
    culture: f64,
    #[serde(default)]
    burn: f64,
    #[serde(default)]
    sovereignty: f64,
    #[serde(default)]
    fidelity_reason: String,
    #[serde(default)]
    phi_reason: String,
    #[serde(default)]
    verify_reason: String,
    #[serde(default)]
    culture_reason: String,
    #[serde(default)]
    burn_reason: String,
    #[serde(default)]
    sovereignty_reason: String,
}

#[async_trait]
impl Dog for InferenceDog {
    fn id(&self) -> &str {
        &self.dog_name
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let system = Self::build_system_prompt();
        let user = Self::build_user_prompt(stimulus);

        let text = self.chat.chat(system, &user).await
            .map_err(|e| match e {
                crate::chat_port::ChatError::RateLimited(m) => DogError::RateLimited(m),
                crate::chat_port::ChatError::Timeout { .. } => DogError::Timeout,
                other => DogError::ApiError(other.to_string()),
            })?;

        let json_str = extract_json(&text)
            .ok_or_else(|| DogError::ParseError(format!("No JSON found in: {}", text)))?;

        let parsed: AxiomResponse = serde_json::from_str(json_str)
            .map_err(|e| DogError::ParseError(format!("JSON parse failed: {} in: {}", e, json_str)))?;

        Ok(AxiomScores {
            fidelity: parsed.fidelity,
            phi: parsed.phi,
            verify: parsed.verify,
            culture: parsed.culture,
            burn: parsed.burn,
            sovereignty: parsed.sovereignty,
            reasoning: AxiomReasoning {
                fidelity: parsed.fidelity_reason,
                phi: parsed.phi_reason,
                verify: parsed.verify_reason,
                culture: parsed.culture_reason,
                burn: parsed.burn_reason,
                sovereignty: parsed.sovereignty_reason,
            },
        })
    }
}

/// Extract JSON object from text that might contain markdown fences or extra text.
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut end = start;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 && end > start {
        Some(&text[start..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_port::MockChatBackend;

    #[test]
    fn extract_json_from_clean() {
        let input = r#"{"fidelity": 0.5, "phi": 0.4, "verify": 0.3}"#;
        assert_eq!(extract_json(input), Some(input));
    }

    #[test]
    fn extract_json_from_markdown() {
        let input = "```json\n{\"fidelity\": 0.5, \"phi\": 0.4, \"verify\": 0.3}\n```";
        let json = extract_json(input).unwrap();
        assert!(json.starts_with('{'));
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(parsed["fidelity"], 0.5);
    }

    #[test]
    fn prompt_contains_stimulus() {
        let stimulus = Stimulus {
            content: "e4 e5 Nf3".into(),
            context: Some("Chess opening".into()),
            domain: Some("chess".into()),
        };
        let prompt = InferenceDog::build_user_prompt(&stimulus);
        assert!(prompt.contains("e4 e5 Nf3"));
        assert!(prompt.contains("chess"));
        assert!(prompt.contains("FIDELITY"));
    }

    #[tokio::test]
    async fn mock_chat_produces_valid_scores() {
        let mock = Arc::new(MockChatBackend::new(
            "test-mock",
            r#"{"fidelity": 0.6, "phi": 0.5, "verify": 0.4, "culture": 0.45, "burn": 0.5, "sovereignty": 0.55, "fidelity_reason": "good", "phi_reason": "ok", "verify_reason": "decent", "culture_reason": "respects patterns", "burn_reason": "efficient", "sovereignty_reason": "preserves agency"}"#,
        ));

        let dog = InferenceDog::new(mock, "test-dog".into());
        let stimulus = Stimulus {
            content: "The sky is blue.".into(),
            context: None,
            domain: None,
        };

        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!((scores.fidelity - 0.6).abs() < 0.01);
        assert!((scores.phi - 0.5).abs() < 0.01);
        assert!((scores.verify - 0.4).abs() < 0.01);
        assert_eq!(scores.reasoning.fidelity, "good");
    }
}
