//! Phase 3 — reflection adapter: turn observed trace evidence into skill
//! hypotheses through sovereign local inference.
//!
//! Faithful to autoresearch: the `reasoning` step is the LLM that RELIES on the
//! observed units to form hypotheses (the "why", not just "what"). The loop
//! (`run_loop`) then scores them against the metric (`skill_reuse_gain`) and
//! keeps/discards. Inference is local-first behind `cynic_inference::ChatPort`:
//! never a SaaS, never a hardcoded client. A fake offline adapter (`OfflineReflection`)
//! makes the loop testable without a model.

use cynic_protocol::autoresearch::{SkillHypothesis, AUTORESEARCH_HYPOTHESIS_SCHEMA};
use cynic_protocol::autoresearch_port::{ReasoningAdapter, ReasoningError};

/// Parse a structured hypothesis block emitted by the reasoning model.
///
/// Format (one block per hypothesis):
/// ```text
/// ### <domain>
/// why: <one line: root cause / repeated pattern>
/// when: <one line: trigger condition>
/// reuse: <one line: how a CYNIC skill would prevent/shorten this next time>
/// actionable: [yes|no]
/// ```
/// Lines that do not parse cleanly are skipped; an empty/garbled response yields
/// no hypotheses (honest Unknown, never a fabricated skill).
pub fn parse_hypotheses(text: &str) -> Vec<SkillHypothesis> {
    let mut out = Vec::new();
    let mut cur_domain: Option<String> = None;
    let mut why = String::new();
    let mut when = String::new();
    let mut reuse = String::new();
    let mut actionable_val = false;

    let mut flush = |domain: &Option<String>, why: &str, when: &str, reuse: &str, actionable: bool| {
        if let Some(d) = domain {
            if !d.is_empty() && (!why.is_empty() || !reuse.is_empty()) {
                out.push(SkillHypothesis {
                    schema: AUTORESEARCH_HYPOTHESIS_SCHEMA.to_string(),
                    skill_id: format!("skill-{}", d),
                    domain: d.clone(),
                    description: format!("why: {}\\nwhen: {}\\nreuse: {}", why, when, reuse),
                    actionability: actionable_val,
                    claimed_coverage: 0.0,
                });
            }
        }
    };

    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("### ") {
            flush(&cur_domain, &why, &when, &reuse, actionable);
            cur_domain = Some(rest.trim().to_string());
            why.clear();
            when.clear();
            reuse.clear();
            actionable_val = false;
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = line.strip_prefix("why:") {
            why = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("when:") {
            when = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("reuse:") {
            reuse = v.trim().to_string();
        } else if let Some(v) = lower.strip_prefix("actionable:") {
            actionable_val = v.trim() == "yes";
        }
    }
    flush(&cur_domain, &why, &when, &reuse, actionable_val);
    out
}

/// Sovereign reflection: asks a local-first chat backend to interpret the
/// observed units and propose skill hypotheses.
pub struct ReflectionAdapter< C: cynic_inference::ports::ChatPort> {
    chat: C,
    system_prompt: String,
    profile: cynic_inference::domain::chat::InferenceProfile,
}

impl<C: cynic_inference::ports::ChatPort> ReflectionAdapter<C> {
    pub fn new(chat: C) -> Self {
        Self {
            chat,
            system_prompt: default_reflection_system_prompt(),
            profile: cynic_inference::domain::chat::InferenceProfile::SCORING,
        }
    }

    /// Build the user prompt from observed evidence (the "why" depends on this).
    fn build_user_prompt(&self, evidence: &str) -> String {
        format!(
            "Observed agent session evidence (failure signatures + frequencies):\n\n{evidence}\n\n\
Propose CYNIC skills that would prevent or shorten these failures next time. \
One hypothesis per block, format:\n\
### <domain>\nwhy: <root cause / repeated pattern>\nwhen: <trigger>\nreuse: <how a CYNIC skill helps>\nactionable: [yes|no]"
        )
    }
}

#[async_trait::async_trait]
impl<C: cynic_inference::ports::ChatPort + Send + Sync> ReasoningAdapter for ReflectionAdapter<C> {
    async fn hypothesize(&self, evidence: &str) -> Result<Vec<SkillHypothesis>, ReasoningError> {
        let resp = self
            .chat
            .chat(&self.system_prompt, &self.build_user_prompt(evidence), self.profile, None)
            .await
            .map_err(|e| ReasoningError::new("reflection_chat_failed", &format!("{e}")))?;
        Ok(parse_hypotheses(&resp.text))
    }
}

/// Offline fake: deterministic structured hypotheses, no model. Used in tests
/// and as a lab fallback when no local inference backend is enrolled.
pub struct OfflineReflection {
    canned: String,
}

impl OfflineReflection {
    pub fn new() -> Self {
        Self {
            canned: "\
### toolchain_ice
why: rustc 1.95/1.96 SIGSEGV on proc-macro2/syn across cynic-kernel/host builds
when: any `cargo build`/`cargo test` of cynic-kernel or cynic-host after a toolchain bump
reuse: a CYNIC skill pins 1.95 for kernel, 1.96 for host, escalates RUST_MIN_STACK, CARGO_BUILD_JOBS=1
actionable: yes

### network
why: egress gate blocks curl to GitHub/raw at session start until explicit consent
when: any out-of-band network fetch (README, crate, API) before operator consent
reuse: a CYNIC skill records the egress gate and requires one-shot consent before fetch
actionable: yes"
                .to_string(),
        }
    }
}

#[async_trait::async_trait]
impl ReasoningAdapter for OfflineReflection {
    async fn hypothesize(&self, _evidence: &str) -> Result<Vec<SkillHypothesis>, ReasoningError> {
        Ok(parse_hypotheses(&self.canned))
    }
}

fn default_reflection_system_prompt() -> String {
    "You are the CYNIC Autoresearcher reflection step. Given observed agent session \
failure evidence, propose reusable CYNIC skills (machine-actionable operating procedures) \
that would prevent or shorten those failures next time. Prefer merging into existing \
CYNIC skills over creating duplicates. Output only the structured hypothesis blocks."
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_extracts_multiple_hypotheses() {
        let text = "\
### toolchain_ice
why: SIGSEGV on proc-macro2
when: cargo build kernel
reuse: pin toolchain
actionable: yes

### network
why: egress gate
when: curl outside
reuse: require consent
actionable: no";
        let hs = parse_hypotheses(text);
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].domain, "toolchain_ice");
        assert!(hs[0].actionable);
        assert!(!hs[1].actionable);
        assert_eq!(hs[1].domain, "network");
    }

    #[test]
    fn parser_returns_empty_on_garbage() {
        // No `### domain` header -> nothing parseable, honest Unknown.
        assert!(parse_hypotheses("just some free text with no structure").is_empty());
    }

    #[test]
    fn offline_reflection_yields_two_actionable_hypotheses() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let hs = rt.block_on(OfflineReflection::new().hypothesize("evidence")).unwrap();
        assert_eq!(hs.len(), 2);
        assert!(hs.iter().all(|h| h.actionable));
    }
}
