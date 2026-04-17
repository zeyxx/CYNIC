//! Reflection type — structured audit output.
//!
//! Verdict values parallel CYNIC's judge pipeline: HOWL (authentic) >
//! WAG (ok) > GROWL (shallow) > BARK (self-deception) > Degraded (audit down).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Howl,
    Wag,
    Growl,
    Bark,
    Degraded,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Howl => "HOWL",
            Verdict::Wag => "WAG",
            Verdict::Growl => "GROWL",
            Verdict::Bark => "BARK",
            Verdict::Degraded => "DEGRADED",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured reflection produced by an `AuditEngine`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reflection {
    pub verdict: Verdict,
    pub prose: String,
    pub patterns_detected: Vec<String>,
    pub kenosis_candidate: Option<String>,
    /// Confidence, φ⁻¹-bounded: MUST be ≤ 0.618.
    pub confidence: f32,
}

impl Reflection {
    /// Constructs a degraded reflection when the audit engine is unavailable.
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Degraded,
            prose: reason.into(),
            patterns_detected: Vec::new(),
            kenosis_candidate: None,
            confidence: 0.0,
        }
    }

    /// Render the reflection as a markdown document (for weekly-reflection.md).
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Weekly Reflection — {}\n\n", self.verdict));
        out.push_str(&format!(
            "**Confidence:** {:.3} (φ⁻¹ bounded)\n\n",
            self.confidence
        ));
        out.push_str("## Prose\n\n");
        out.push_str(&self.prose);
        out.push_str("\n\n");
        if !self.patterns_detected.is_empty() {
            out.push_str("## Patterns detected\n\n");
            for p in &self.patterns_detected {
                out.push_str(&format!("- {p}\n"));
            }
            out.push('\n');
        }
        if let Some(k) = &self.kenosis_candidate {
            out.push_str("## KENOSIS candidate\n\n");
            out.push_str(k);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_display_matches_as_str() {
        assert_eq!(Verdict::Howl.to_string(), "HOWL");
        assert_eq!(Verdict::Bark.to_string(), "BARK");
        assert_eq!(Verdict::Degraded.to_string(), "DEGRADED");
    }

    #[test]
    fn reflection_degraded_has_zero_confidence() {
        let r = Reflection::degraded("gemini timeout");
        assert_eq!(r.verdict, Verdict::Degraded);
        assert_eq!(r.confidence, 0.0);
        assert!(r.prose.contains("gemini timeout"));
    }

    #[test]
    fn reflection_markdown_contains_verdict_and_prose() {
        let r = Reflection {
            verdict: Verdict::Wag,
            prose: "Zey a bougé 3 fois cette semaine.".into(),
            patterns_detected: vec!["reprise progressive".into()],
            kenosis_candidate: None,
            confidence: 0.55,
        };
        let md = r.to_markdown();
        assert!(md.contains("WAG"));
        assert!(md.contains("Zey a bougé"));
        assert!(md.contains("reprise progressive"));
        assert!(md.contains("0.550"));
    }

    #[test]
    fn reflection_serializes_roundtrip() {
        let r = Reflection {
            verdict: Verdict::Howl,
            prose: "excellent".into(),
            patterns_detected: vec![],
            kenosis_candidate: Some("stopped smoking after lunch".into()),
            confidence: 0.6,
        };
        let json = serde_json::to_string(&r).unwrap();
        let parsed: Reflection = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }
}
