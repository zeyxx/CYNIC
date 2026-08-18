use crate::domain::dog::{AxiomScores, DogError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScorePayload {
    fidelity: f64,
    phi: f64,
    verify: f64,
    culture: f64,
    burn: f64,
    sovereignty: f64,
    #[serde(default)]
    reasoning: crate::domain::dog::AxiomReasoning,
}

pub(super) fn parse(text: &str) -> Result<AxiomScores, DogError> {
    let mut trimmed = text.trim();
    if trimmed.starts_with("```") {
        trimmed = trimmed.trim_start_matches('`');
        if let Some(rest) = trimmed.strip_prefix("json") {
            trimmed = rest;
        }
    }
    if trimmed.ends_with('`') {
        trimmed = trimmed.trim_end_matches('`');
    }
    let trimmed = trimmed.trim();
    let payload: ScorePayload = serde_json::from_str(trimmed)
        .map_err(|error| DogError::ParseError(format!("invalid scoring JSON: {error}")))?;
    Ok(AxiomScores {
        fidelity: payload.fidelity,
        phi: payload.phi,
        verify: payload.verify,
        culture: payload.culture,
        burn: payload.burn,
        sovereignty: payload.sovereignty,
        reasoning: payload.reasoning,
        ..AxiomScores::default()
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_strict_scores_and_fenced_json() {
        let scores = parse(
            "```json\n{\"fidelity\":0.5,\"phi\":0.4,\"verify\":0.3,\"culture\":0.2,\"burn\":0.1,\"sovereignty\":0.6}\n```",
        )
        .expect("valid score payload");
        assert_eq!(scores.fidelity, 0.5);
        assert_eq!(scores.sovereignty, 0.6);
    }

    #[test]
    fn rejects_non_json_output() {
        assert!(parse("I think this is true").is_err());
    }
}
