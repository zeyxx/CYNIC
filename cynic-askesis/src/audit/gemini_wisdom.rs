//! Gemini CLI subprocess adapter with inline cynic-wisdom skill text.

use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;

use crate::audit::AuditEngine;
use crate::log::LogEntry;
use crate::reflection::{Reflection, Verdict};

/// Inline cynic-wisdom skill content. Version pinned at compile time.
///
/// **Load-bearing for FOGC** (spec §5): inverting axioms requires
/// changing this embedded text, which is committed to git under CODEOWNERS.
const CYNIC_WISDOM_SKILL: &str = include_str!("../../../.agents/skills/cynic-wisdom/SKILL.md");

const DEFAULT_MODEL: &str = "gemini-2.5-pro";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct GeminiWisdomAudit {
    model: String,
    timeout: Duration,
}

impl Default for GeminiWisdomAudit {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl GeminiWisdomAudit {
    pub fn new(model: impl Into<String>, timeout: Duration) -> Self {
        Self {
            model: model.into(),
            timeout,
        }
    }

    fn build_prompt(logs: &[LogEntry], questions: &[&str]) -> String {
        let mut p = String::new();
        p.push_str("Use skill cynic-wisdom (inline text below).\n\n");
        p.push_str("=== cynic-wisdom SKILL TEXT ===\n");
        p.push_str(CYNIC_WISDOM_SKILL);
        p.push_str("\n=== END SKILL ===\n\n");
        p.push_str("Audit the following log entries against the questions.\n\n");
        p.push_str("=== LOGS ===\n");
        for e in logs {
            p.push_str(&format!(
                "[{}] domain={}: {}\n",
                e.timestamp.format("%Y-%m-%d %H:%M"),
                e.domain.as_deref().unwrap_or("(none)"),
                e.content
            ));
        }
        p.push_str("\n=== QUESTIONS ===\n");
        for q in questions {
            p.push_str(&format!("- {q}\n"));
        }
        p.push_str("\n=== OUTPUT FORMAT ===\n");
        p.push_str("Respond with exactly this structure:\n\n");
        p.push_str("VERDICT: HOWL|WAG|GROWL|BARK\n");
        p.push_str("CONFIDENCE: <float ≤ 0.618>\n");
        p.push_str("KENOSIS_CANDIDATE: <short sentence or NONE>\n");
        p.push_str("PATTERNS:\n- <pattern1>\n- <pattern2>\n\n");
        p.push_str("PROSE:\n<markdown narrative, honest but not shaming>\n");
        p
    }

    fn parse_output(raw: &str) -> Reflection {
        let mut verdict = Verdict::Degraded;
        let mut confidence: f32 = 0.0;
        let mut kenosis: Option<String> = None;
        let mut patterns: Vec<String> = Vec::new();
        let mut prose = String::new();
        let mut in_patterns = false;
        let mut in_prose = false;

        for line in raw.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("VERDICT:") {
                verdict = match rest.trim().to_uppercase().as_str() {
                    "HOWL" => Verdict::Howl,
                    "WAG" => Verdict::Wag,
                    "GROWL" => Verdict::Growl,
                    "BARK" => Verdict::Bark,
                    _ => Verdict::Degraded,
                };
                in_patterns = false;
                in_prose = false;
            } else if let Some(rest) = trimmed.strip_prefix("CONFIDENCE:") {
                confidence = rest.trim().parse().unwrap_or(0.0_f32).min(0.618);
                in_patterns = false;
                in_prose = false;
            } else if let Some(rest) = trimmed.strip_prefix("KENOSIS_CANDIDATE:") {
                let v = rest.trim();
                kenosis = if v.eq_ignore_ascii_case("NONE") || v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                };
                in_patterns = false;
                in_prose = false;
            } else if trimmed == "PATTERNS:" {
                in_patterns = true;
                in_prose = false;
            } else if trimmed == "PROSE:" {
                in_patterns = false;
                in_prose = true;
            } else if in_patterns {
                if let Some(item) = trimmed.strip_prefix("- ") {
                    patterns.push(item.to_string());
                }
            } else if in_prose {
                prose.push_str(line);
                prose.push('\n');
            }
        }

        if verdict == Verdict::Degraded && prose.is_empty() {
            return Reflection::degraded("gemini output malformed");
        }

        Reflection {
            verdict,
            prose: prose.trim().to_string(),
            patterns_detected: patterns,
            kenosis_candidate: kenosis,
            confidence,
        }
    }
}

#[async_trait]
impl AuditEngine for GeminiWisdomAudit {
    async fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> crate::Result<Reflection> {
        let prompt = Self::build_prompt(logs, questions);

        let fut = Command::new("gemini")
            .args(["-m", &self.model, "-p", &prompt])
            .output();

        let output = match timeout(self.timeout, fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Ok(Reflection::degraded(format!("gemini spawn failed: {e}"))),
            Err(_) => return Ok(Reflection::degraded("gemini timed out")),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(Reflection::degraded(format!("gemini exit != 0: {stderr}")));
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_output(&raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_handles_well_formed_response() {
        let raw = "\
VERDICT: WAG
CONFIDENCE: 0.55
KENOSIS_CANDIDATE: stopped checking phone during dinner
PATTERNS:
- reprise progressive
- authentic language on body

PROSE:
Zey shows honest engagement with body tracking this week.
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert_eq!(r.verdict, Verdict::Wag);
        assert!((r.confidence - 0.55).abs() < 1e-6);
        assert_eq!(r.patterns_detected.len(), 2);
        assert!(r.kenosis_candidate.as_deref() == Some("stopped checking phone during dinner"));
        assert!(r.prose.contains("honest engagement"));
    }

    #[test]
    fn parse_output_clamps_confidence_to_phi_inverse() {
        let raw = "\
VERDICT: HOWL
CONFIDENCE: 0.99
KENOSIS_CANDIDATE: NONE
PATTERNS:
PROSE:
test
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert!(r.confidence <= 0.618);
    }

    #[test]
    fn parse_output_degraded_on_garbage() {
        let r = GeminiWisdomAudit::parse_output("lol what");
        assert_eq!(r.verdict, Verdict::Degraded);
    }

    #[test]
    fn parse_output_handles_kenosis_none() {
        let raw = "\
VERDICT: GROWL
CONFIDENCE: 0.4
KENOSIS_CANDIDATE: NONE
PATTERNS:
- shallow reporting
PROSE:
thin descriptions
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert_eq!(r.verdict, Verdict::Growl);
        assert!(r.kenosis_candidate.is_none());
    }

    #[test]
    fn build_prompt_includes_skill_text_and_questions() {
        let logs = vec![LogEntry::new("test entry").with_domain("body")];
        let questions = vec!["q1", "q2"];
        let prompt = GeminiWisdomAudit::build_prompt(&logs, &questions);
        assert!(prompt.contains("cynic-wisdom"));
        assert!(prompt.contains("test entry"));
        assert!(prompt.contains("q1"));
        assert!(prompt.contains("q2"));
        assert!(prompt.contains("VERDICT:"));
    }
}
