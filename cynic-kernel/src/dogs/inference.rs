//! InferenceDog — model-agnostic Dog that uses any ChatPort for axiom evaluation.
//! ONE prompt template, N backends. The Dog never knows which model it's talking to.

use crate::domain::chat::{ChatPort, InferenceProfile};
use crate::domain::dog::*;
use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

pub struct InferenceDog {
    chat: Arc<dyn ChatPort>,
    dog_name: String,
    context_size: u32,
    timeout: u64,
    domain_prompts: Arc<std::collections::HashMap<String, String>>,
}

impl std::fmt::Debug for InferenceDog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceDog").finish_non_exhaustive()
    }
}

impl InferenceDog {
    pub fn new(
        chat: Arc<dyn ChatPort>,
        name: String,
        context_size: u32,
        timeout_secs: u64,
    ) -> Self {
        Self {
            chat,
            dog_name: name,
            context_size,
            timeout: timeout_secs,
            domain_prompts: Arc::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_domain_prompts(
        mut self,
        prompts: Arc<std::collections::HashMap<String, String>>,
    ) -> Self {
        self.domain_prompts = prompts;
        self
    }

    fn build_system_prompt() -> &'static str {
        "You are CYNIC, a sovereign epistemic judge. You evaluate THE SUBJECT MATTER described in the stimulus — not the quality of its description. In chess, judge the MOVE or STRATEGY, not the text. In science, judge the CLAIM, not the writing. Your axioms measure the SUBSTANCE, not the FORM.\n\nVERIFY means 'does this SURVIVE testing?' — not 'can you test it?' A strategy easily refuted by analysis scores LOW on VERIFY. A claim disproven by evidence scores LOW on VERIFY.\n\nBe harsh. Be honest. Overconfidence is the enemy. Most things deserve 0.3-0.6, not 0.8-0.9. The value 0.618 (phi^-1) is your absolute upper limit for confidence in any standard evaluation.\n\nSCORING RANGE: 0.05 to 1.0. Never return exactly 0.0 for any axiom. The minimum possible score is 0.05. A score of 0.0 means you failed to evaluate, not that the content is bad. Terrible content scores 0.05-0.15, not 0.0."
    }

    fn build_user_prompt(
        stimulus: &Stimulus,
        domain_prompts: &std::collections::HashMap<String, String>,
    ) -> String {
        let context_block = stimulus
            .context
            .as_deref()
            .unwrap_or("(no additional context)");
        let domain = stimulus.domain.as_deref().unwrap_or("general");

        // Domain-specific axiom evaluation criteria (from domains/*.md)
        let axioms_section = if let Some(domain_prompt) = domain_prompts.get(domain) {
            format!("DOMAIN-SPECIFIC EVALUATION CRITERIA:\n{domain_prompt}")
        } else {
            "AXIOMS:\n\
             1. FIDELITY — Is the SUBJECT MATTER itself sound? Judge the THING, not the accuracy of its description.\n\
             2. PHI — Is this structurally harmonious? Well-coordinated? Proportional?\n\
             3. VERIFY — Does this SURVIVE scrutiny? When tested against the strongest counterarguments, does it hold?\n\
             4. CULTURE — Does this honor existing traditions, conventions, and established patterns?\n\
             5. BURN — Is this efficient? Minimal waste? Could excess be destroyed without loss?\n\
             6. SOVEREIGNTY — Does this preserve individual agency and freedom of choice?".to_string()
        };

        format!(
            r#"DOMAIN: {domain}
STIMULUS: {content}
CONTEXT: {context_block}

IMPORTANT: Evaluate THE SUBJECT MATTER described — not how well it is described. A well-written description of a bad strategy is still a bad strategy. A poorly written description of a brilliant idea is still brilliant.

{axioms_section}

STEP 1: In 2-3 sentences, analyze the QUALITY of the subject matter itself. What is strong? What is weak? Be specific and harsh.
STEP 2: Based on your analysis, score each axiom 0.05-1.0. Your scores MUST reflect your analysis — if you identified weaknesses, the scores must be LOW.

Output your analysis, then this exact JSON inside <json> tags (keep each reason under 15 words):
<json>
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "culture": 0.XX, "burn": 0.XX, "sovereignty": 0.XX, "fidelity_reason": "...", "phi_reason": "...", "verify_reason": "...", "culture_reason": "...", "burn_reason": "...", "sovereignty_reason": "..."}}
</json>"#,
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

    fn max_context(&self) -> u32 {
        self.context_size
    }

    fn timeout_secs(&self) -> u64 {
        self.timeout
    }

    async fn health(&self) -> BackendStatus {
        BackendPort::health(self.chat.as_ref()).await
    }

    #[tracing::instrument(skip(self), err, fields(dog_id = %self.dog_name))]
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let system = Self::build_system_prompt();
        let user = Self::build_user_prompt(stimulus, &self.domain_prompts);

        let chat_resp = self
            .chat
            .chat(
                system,
                &user,
                InferenceProfile::Scoring,
                stimulus.request_id.as_deref(),
            )
            .await
            .map_err(|e| match e {
                crate::domain::chat::ChatError::RateLimited(m) => DogError::RateLimited(m),
                crate::domain::chat::ChatError::Timeout { .. } => DogError::Timeout,
                other => DogError::ApiError(other.to_string()),
            })?;
        let text = &chat_resp.text;
        let prompt_tokens = chat_resp.prompt_tokens;
        let completion_tokens = chat_resp.completion_tokens;

        let json_str = extract_json(text)
            .ok_or_else(|| DogError::ParseError(format!("No JSON found in: {text}")))?;

        // Try strict parse first. Fall back to lenient extraction for small models
        // that produce duplicate keys (e.g. Gemma writes "verify": 0.7 AND "verify": "text").
        let mut scores = match serde_json::from_str::<AxiomResponse>(json_str) {
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
                prompt_tokens: 0,
                completion_tokens: 0,
                abstentions: vec![],
            },
            Err(_) => extract_scores_lenient(json_str)?,
        };
        scores.prompt_tokens = prompt_tokens;
        scores.completion_tokens = completion_tokens;

        // Validate BEFORE phi_bound — catch pathological outputs that phi_bound would mask
        crate::domain::dog::validate_scores(&scores)?;

        Ok(scores)
    }
}

/// Lenient score extraction for small models that produce duplicate JSON keys.
/// Uses serde_json::Value (which keeps last duplicate) then scans raw text for first numeric per key.
fn extract_scores_lenient(json_str: &str) -> Result<AxiomScores, DogError> {
    // serde_json::Value accepts duplicate keys (keeps last value).
    // For scores we want the FIRST numeric value, so we scan the raw text.
    let axiom_names = [
        "fidelity",
        "phi",
        "verify",
        "culture",
        "burn",
        "sovereignty",
    ];
    let mut scores = std::collections::HashMap::new();
    let mut reasons = std::collections::HashMap::new();

    for name in &axiom_names {
        // Find first occurrence of "name": <number>
        let score_pattern = format!("\"{name}\"");
        if let Some(pos) = json_str.find(&score_pattern) {
            let after_key = &json_str[pos + score_pattern.len()..];
            // Skip whitespace and colon
            let after_colon = after_key
                .trim_start()
                .strip_prefix(':')
                .unwrap_or(after_key)
                .trim_start();
            // Try to parse a number
            let num_end = after_colon
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(after_colon.len());
            if let Ok(v) = after_colon[..num_end].parse::<f64>() {
                scores.insert(*name, v);
            }
        }

        // Find first occurrence of "name_reason": "text"
        let reason_key = format!("\"{name}_reason\"");
        if let Some(pos) = json_str.find(&reason_key) {
            let after_key = &json_str[pos + reason_key.len()..];
            let after_colon = after_key
                .trim_start()
                .strip_prefix(':')
                .unwrap_or(after_key)
                .trim_start();
            if let Some(inner) = after_colon.strip_prefix('"') {
                // Find closing quote (handle escaped quotes minimally)
                let mut end = 0;
                let mut escaped = false;
                for (i, c) in inner.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                        continue;
                    }
                    if c == '"' {
                        end = i;
                        break;
                    }
                }
                reasons.insert(*name, inner[..end].to_string());
            }
        }
    }

    if scores.is_empty() {
        return Err(DogError::ParseError(
            "No numeric scores found in lenient parse".into(),
        ));
    }

    // Warn on missing axioms — silent 0.0 defaults can sneak through validate_scores()
    // if ≤3 axioms are missing (under the MAX_ZERO_SCORES threshold).
    let all_axioms = [
        "fidelity",
        "phi",
        "verify",
        "culture",
        "burn",
        "sovereignty",
    ];
    let missing: Vec<&str> = all_axioms
        .iter()
        .filter(|k| !scores.contains_key(**k))
        .copied()
        .collect();
    if !missing.is_empty() {
        tracing::warn!(
            missing_axioms = ?missing,
            found_count = scores.len(),
            "lenient parse: missing axioms defaulted to 0.0"
        );
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
            fidelity: get_r("fidelity"),
            phi: get_r("phi"),
            verify: get_r("verify"),
            culture: get_r("culture"),
            burn: get_r("burn"),
            sovereignty: get_r("sovereignty"),
        },
        prompt_tokens: 0,
        completion_tokens: 0,
        abstentions: vec![],
    })
}

/// Extract JSON object from text that might contain markdown fences or extra text.
fn extract_json(text: &str) -> Option<&str> {
    // 1. Try explicit tags first (new robust path for small models)
    if let Some(start) = text.find("<json>") {
        if let Some(end) = text.find("</json>") {
            return Some(text[start + 6..end].trim());
        }
        // Handle case where closing tag is missing but opening is present
        return extract_json(&text[start + 6..]);
    }

    // 2. Existing brace-counting logic as fallback
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
    use crate::domain::chat::MockChatBackend;

    #[test]
    fn extract_json_from_tags() {
        let input = "Here is my analysis... <json>{\"fidelity\": 0.5}</json> I hope this helps.";
        let json = extract_json(input).unwrap();
        assert_eq!(json, "{\"fidelity\": 0.5}");
    }

    #[test]
    fn extract_json_from_incomplete_tags() {
        let input = "Conversational filler... <json>{\"fidelity\": 0.6}";
        let json = extract_json(input).unwrap();
        assert_eq!(json, "{\"fidelity\": 0.6}");
    }

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
    fn prompt_contains_stimulus_with_generic_fallback() {
        let stimulus = Stimulus {
            content: "e4 e5 Nf3".into(),
            context: Some("Chess opening".into()),
            domain: Some("chess".into()),
            request_id: None,
        };
        let empty = std::collections::HashMap::new();
        let prompt = InferenceDog::build_user_prompt(&stimulus, &empty);
        assert!(prompt.contains("e4 e5 Nf3"));
        assert!(prompt.contains("chess"));
        assert!(prompt.contains("FIDELITY"));
        // Generic fallback — no domain-specific criteria
        assert!(
            prompt.contains("AXIOMS:"),
            "should use generic axioms when no domain prompt"
        );
    }

    #[test]
    fn prompt_uses_domain_specific_criteria() {
        let stimulus = Stimulus {
            content: "e4 c5 Sicilian".into(),
            context: None,
            domain: Some("chess".into()),
            request_id: None,
        };
        let mut prompts = std::collections::HashMap::new();
        prompts.insert(
            "chess".to_string(),
            "## FIDELITY\nIs this faithful to sound chess principles?".to_string(),
        );
        let prompt = InferenceDog::build_user_prompt(&stimulus, &prompts);
        assert!(
            prompt.contains("DOMAIN-SPECIFIC EVALUATION CRITERIA:"),
            "should use domain prompt"
        );
        assert!(prompt.contains("faithful to sound chess principles"));
        assert!(
            !prompt.contains("AXIOMS:"),
            "should NOT have generic axioms when domain prompt exists"
        );
    }

    #[tokio::test]
    async fn mock_chat_produces_valid_scores() {
        let mock = Arc::new(MockChatBackend::new(
            "test-mock",
            r#"{"fidelity": 0.6, "phi": 0.5, "verify": 0.4, "culture": 0.45, "burn": 0.5, "sovereignty": 0.55, "fidelity_reason": "good", "phi_reason": "ok", "verify_reason": "decent", "culture_reason": "respects patterns", "burn_reason": "efficient", "sovereignty_reason": "preserves agency"}"#,
        ));

        let dog = InferenceDog::new(mock, "test-dog".into(), 4096, 30);
        let stimulus = Stimulus {
            content: "The sky is blue.".into(),
            context: None,
            domain: None,
            request_id: None,
        };

        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!((scores.fidelity - 0.6).abs() < 0.01);
        assert!((scores.phi - 0.5).abs() < 0.01);
        assert!((scores.verify - 0.4).abs() < 0.01);
        assert_eq!(scores.reasoning.fidelity, "good");
    }

    // ── extract_scores_lenient (P1 VERIFIABILITY) ───────────

    #[test]
    fn lenient_parse_valid_json() {
        let input = r#"{"fidelity": 0.7, "phi": 0.6, "verify": 0.5, "culture": 0.4, "burn": 0.8, "sovereignty": 0.3}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert!((scores.fidelity - 0.7).abs() < 0.01);
        assert!((scores.phi - 0.6).abs() < 0.01);
        assert!((scores.sovereignty - 0.3).abs() < 0.01);
    }

    #[test]
    fn lenient_parse_duplicate_keys_takes_first() {
        // Small models sometimes emit duplicate keys — lenient parser takes FIRST occurrence
        let input = r#"{"fidelity": 0.8, "phi": 0.5, "verify": 0.4, "culture": 0.3, "burn": 0.6, "sovereignty": 0.7, "fidelity": 0.1}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert!(
            (scores.fidelity - 0.8).abs() < 0.01,
            "should take first fidelity (0.8), got {}",
            scores.fidelity
        );
    }

    #[test]
    fn lenient_parse_with_reasons() {
        let input = r#"{"fidelity": 0.6, "fidelity_reason": "accurate claim", "phi": 0.5, "verify": 0.4, "culture": 0.3, "burn": 0.7, "sovereignty": 0.5}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert_eq!(scores.reasoning.fidelity, "accurate claim");
    }

    #[test]
    fn lenient_parse_no_scores_returns_error() {
        let input = r#"{"message": "I cannot evaluate this"}"#;
        let result = extract_scores_lenient(input);
        assert!(result.is_err(), "should fail when no axiom scores found");
    }

    #[test]
    fn lenient_parse_partial_scores_defaults_missing() {
        // Only some axioms present — missing ones default to 0.0
        let input = r#"{"fidelity": 0.7, "burn": 0.5}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert!((scores.fidelity - 0.7).abs() < 0.01);
        assert!((scores.burn - 0.5).abs() < 0.01);
        assert!((scores.phi - 0.0).abs() < 0.01, "missing phi should be 0.0");
        assert!(
            (scores.verify - 0.0).abs() < 0.01,
            "missing verify should be 0.0"
        );
    }

    #[test]
    fn lenient_parse_non_numeric_value_skipped() {
        // Model returns string instead of number for a score
        let input = r#"{"fidelity": "high", "phi": 0.5, "verify": 0.4, "culture": 0.3, "burn": 0.6, "sovereignty": 0.7}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert!(
            (scores.fidelity - 0.0).abs() < 0.01,
            "non-numeric fidelity should default to 0.0"
        );
        assert!((scores.phi - 0.5).abs() < 0.01, "numeric phi should parse");
    }

    #[test]
    fn lenient_parse_reason_with_escaped_quotes() {
        let input = r#"{"fidelity": 0.5, "fidelity_reason": "it's a \"strong\" claim", "phi": 0.4, "verify": 0.3, "culture": 0.2, "burn": 0.6, "sovereignty": 0.5}"#;
        let scores = extract_scores_lenient(input).unwrap();
        assert_eq!(scores.reasoning.fidelity, r#"it's a \"strong\" claim"#);
    }
}
