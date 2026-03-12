//! GeminiDog — calls Google AI Studio (Gemini 2.5 Flash) for axiom evaluation.
//! Free tier: 10 RPM, 250 req/day, zero cost.
//! The Dog returns RAW scores. Phi-bounding happens in the kernel.

use crate::dog::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GeminiDog {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiDog {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "gemini-2.5-flash".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    fn build_prompt(stimulus: &Stimulus) -> String {
        let context_block = stimulus.context.as_deref().unwrap_or("(no additional context)");
        let domain = stimulus.domain.as_deref().unwrap_or("general");

        format!(r#"You are CYNIC, a sovereign judgment engine. Evaluate this stimulus through 3 axioms.

DOMAIN: {domain}
STIMULUS: {content}
CONTEXT: {context_block}

Score each axiom from 0.0 to 1.0 with honest uncertainty. DO NOT inflate scores.
If you're unsure, score lower. Overconfidence is the enemy.

AXIOMS:
1. FIDELITY — Is this faithful to truth/reality? Does it reflect what IS, not what we wish?
2. PHI — Is this structurally harmonious? Well-proportioned? Elegant or clumsy?
3. VERIFY — Is this verifiable or falsifiable? Can we test it? What evidence supports/refutes it?

Respond ONLY with this exact JSON (no markdown, no explanation):
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "fidelity_reason": "...", "phi_reason": "...", "verify_reason": "..."}}"#,
            content = stimulus.content,
        )
    }
}

#[derive(Deserialize)]
struct GeminiAxiomResponse {
    fidelity: f64,
    phi: f64,
    verify: f64,
    #[serde(default)]
    fidelity_reason: String,
    #[serde(default)]
    phi_reason: String,
    #[serde(default)]
    verify_reason: String,
}

// Google AI Studio generateContent request/response structures
#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

#[async_trait]
impl Dog for GeminiDog {
    fn id(&self) -> &str {
        "gemini-dog"
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let prompt = Self::build_prompt(stimulus);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
        };

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DogError::ApiError(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DogError::RateLimited("Gemini free tier rate limit hit".into()));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DogError::ApiError(format!("{}: {}", status, text)));
        }

        let gen_resp: GenerateContentResponse = resp.json().await
            .map_err(|e| DogError::ParseError(format!("Failed to parse Gemini response: {}", e)))?;

        let text = gen_resp.candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .ok_or_else(|| DogError::ParseError("Empty Gemini response".into()))?;

        // Extract JSON from response (Gemini sometimes wraps in markdown)
        let json_str = extract_json(&text)
            .ok_or_else(|| DogError::ParseError(format!("No JSON found in: {}", text)))?;

        let parsed: GeminiAxiomResponse = serde_json::from_str(json_str)
            .map_err(|e| DogError::ParseError(format!("JSON parse failed: {} in: {}", e, json_str)))?;

        Ok(AxiomScores {
            fidelity: parsed.fidelity,
            phi: parsed.phi,
            verify: parsed.verify,
            reasoning: AxiomReasoning {
                fidelity: parsed.fidelity_reason,
                phi: parsed.phi_reason,
                verify: parsed.verify_reason,
            },
        })
    }
}

/// Extract JSON object from text that might contain markdown fences or extra text
fn extract_json(text: &str) -> Option<&str> {
    // Try to find { ... } boundaries
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
    fn extract_json_from_nested() {
        let input = r#"Here is the result: {"fidelity": 0.5, "phi": 0.4, "verify": 0.3, "nested": {"a": 1}} done"#;
        let json = extract_json(input).unwrap();
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
        let prompt = GeminiDog::build_prompt(&stimulus);
        assert!(prompt.contains("e4 e5 Nf3"));
        assert!(prompt.contains("chess"));
        assert!(prompt.contains("FIDELITY"));
    }
}
