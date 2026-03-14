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

Evaluate THE SUBJECT MATTER described (not the description). Score each axiom from 0.0 to 1.0 with honest uncertainty.

AXIOMS:
1. FIDELITY — Is this faithful to truth? Does it reflect sound principles in its domain?
2. PHI — Is this structurally harmonious? Well-coordinated? Proportional?
3. VERIFY — Is it sound and testable? Can the idea be verified or refuted?
4. CULTURE — Does this honor existing traditions, conventions, and established patterns?
5. BURN — Is this efficient? Minimal waste? Could excess be destroyed without loss?
6. SOVEREIGNTY — Does this preserve individual agency and freedom of choice?

Respond ONLY with this exact JSON. Keep each reason under 15 words.
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

        // Try strict parse first. Fall back to lenient extraction for small models
        // that produce duplicate keys (e.g. Gemma writes "verify": 0.7 AND "verify": "text").
        let scores = match serde_json::from_str::<AxiomResponse>(json_str) {
            Ok(parsed) => AxiomScores {
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
            },
            Err(_) => extract_scores_lenient(json_str)?,
        };

        Ok(scores)
    }
}

/// Lenient score extraction for small models that produce duplicate JSON keys.
/// Uses serde_json::Value (which keeps last duplicate) then scans raw text for first numeric per key.
fn extract_scores_lenient(json_str: &str) -> Result<AxiomScores, DogError> {
    // serde_json::Value accepts duplicate keys (keeps last value).
    // For scores we want the FIRST numeric value, so we scan the raw text.
    let axiom_names = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"];
    let mut scores = std::collections::HashMap::new();
    let mut reasons = std::collections::HashMap::new();

    for name in &axiom_names {
        // Find first occurrence of "name": <number>
        let score_pattern = format!("\"{}\"", name);
        if let Some(pos) = json_str.find(&score_pattern) {
            let after_key = &json_str[pos + score_pattern.len()..];
            // Skip whitespace and colon
            let after_colon = after_key.trim_start().strip_prefix(':').unwrap_or(after_key).trim_start();
            // Try to parse a number
            let num_end = after_colon.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(after_colon.len());
            if let Ok(v) = after_colon[..num_end].parse::<f64>() {
                scores.insert(*name, v);
            }
        }

        // Find first occurrence of "name_reason": "text"
        let reason_key = format!("\"{}_reason\"", name);
        if let Some(pos) = json_str.find(&reason_key) {
            let after_key = &json_str[pos + reason_key.len()..];
            let after_colon = after_key.trim_start().strip_prefix(':').unwrap_or(after_key).trim_start();
            if let Some(inner) = after_colon.strip_prefix('"') {
                // Find closing quote (handle escaped quotes minimally)
                let mut end = 0;
                let mut escaped = false;
                for (i, c) in inner.char_indices() {
                    if escaped { escaped = false; continue; }
                    if c == '\\' { escaped = true; continue; }
                    if c == '"' { end = i; break; }
                }
                reasons.insert(*name, inner[..end].to_string());
            }
        }
    }

    if scores.is_empty() {
        return Err(DogError::ParseError("No numeric scores found in lenient parse".into()));
    }

    let get = |k: &str| *scores.get(k).unwrap_or(&0.0);
    let get_r = |k: &str| reasons.get(k).cloned().unwrap_or_default();

    Ok(AxiomScores {
        fidelity: get("fidelity"),
        phi: get("phi"),
        verify: get("verify"),
        culture: get("culture"),
        burn: get("burn"),
        sovereignty: get("sovereignty"),
        reasoning: AxiomReasoning {
            fidelity: get_r("fidelity_reason"),
            phi: get_r("phi_reason"),
            verify: get_r("verify_reason"),
            culture: get_r("culture_reason"),
            burn: get_r("burn_reason"),
            sovereignty: get_r("sovereignty_reason"),
        },
    })
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
