//! Runner for the autoresearch surface.
//!
//! Composition root: wires the observation adapter (TraceFsAdapter) + the
//! reasoning adapter (ReflectionAdapter or OfflineReflection) + the pure
//! research loop (run_loop) + judgment projection. Then optionally persists
//! the report.

use std::path::Path;

use cynic_intel::autoresearch::reflection::OfflineReflection;
use cynic_intel::autoresearch::trace_fs::{TraceFsAdapter, TraceSource};
use cynic_protocol::autoresearch::AutoresearchProgram;
use cynic_protocol::autoresearch_port::AutoresearchPort;

use crate::autoresearch_surface::types::{AutoresearchEntry, AutoresearchReport, AutoresearchSummary};

/// Run the full autoresearch loop and project the result into a judgment.
pub fn run_autoresearch(
    corpus: Option<&str>,
    program_path: Option<&str>,
    output: Option<&str>,
    subject: Option<&str>,
) -> AutoresearchReport {
    // 1. Load program (or default).
    let program = match program_path {
        Some(path) => match load_program(path) {
            Ok(p) => p,
            Err(e) => {
                return AutoresearchReport::new(
                    AutoresearchSummary { corpus_sessions: 0, hypotheses_evaluated: 0, kept: 0, discarded: 0, unknown: 0 },
                    vec![],
                    Some(error_judgment(&format!("program_load_failed: {e}"))),
                );
            }
        },
        None => AutoresearchProgram::default(),
    };

    // 2. Observe traces.
    let observation_adapter = TraceFsAdapter::new(TraceSource::from_path(
        corpus.unwrap_or("~/.claude/projects/"),
    ));
    let units = match observation_adapter.observe_traces() {
        Ok(u) => u,
        Err(e) => {
            return AutoresearchReport::new(
                AutoresearchSummary { corpus_sessions: 0, hypotheses_evaluated: 0, kept: 0, discarded: 0, unknown: 0 },
                vec![],
                Some(error_judgment(&format!("observation_failed: {e}"))),
            );
        }
    };

    // 3. Hypothesize (offline for now — no live LLM in CLI surface).
    let reasoning_adapter = OfflineReflection::new();
    let hypotheses = reasoning_adapter.hypothesize(&units);

    // 4. Run the pure loop.
    let report = cynic_protocol::autoresearch_port::run_loop_with_hypotheses(
        &program,
        &units,
        &hypotheses,
    );

    // 5. Build entries.
    let entries: Vec<AutoresearchEntry> = report
        .entries
        .iter()
        .map(|e| AutoresearchEntry {
            skill_id: e.skill_id.clone(),
            domain: e.domain.clone(),
            gain: e.gain,
            status: format!("{:?}", e.status),
        })
        .collect();

    let summary = AutoresearchSummary {
        corpus_sessions: report.corpus_sessions,
        hypotheses_evaluated: report.hypotheses_evaluated,
        kept: report.kept.len(),
        discarded: report.discarded.len(),
        unknown: report.entries.iter().filter(|e| matches!(e.status, cynic_protocol::autoresearch::KeepDiscard::Unknown)).count(),
    };

    // 6. Project into judgment.
    let judgment = project_judgment(&report, subject);

    let autoresearch_report = AutoresearchReport::new(summary, entries, Some(judgment));

    // 7. Optionally persist.
    if let Some(output_dir) = output {
        persist_report(autoresearch_report.clone(), output_dir);
    }

    autoresearch_report
}

/// Project the autoresearch report into a CynicJudgment.
fn project_judgment(
    report: &cynic_protocol::autoresearch::AutoresearchReport,
    subject: Option<&str>,
) -> CynicJudgment {
    use cynic_eval::judgment::{CynefinDomain, DijkwLayer, SagePhase, CynicJudgment};

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let subject_id = subject.unwrap_or("local");

    // The judgment is Knowledge layer (pure decision rule applied to observed data),
    // Evaluate phase (closes the loop by judging whether the research converged).
    let mut j = CynicJudgment::new(
        format!("autoresearch-{subject_id}-{now}"),
        false,
        DijkwLayer::Knowledge,
        SagePhase::Evaluate,
        CynefinDomain::Complicated,
        now,
    );

    // Confidence reflects the ratio of decisive outcomes (keep/discard) vs unknown.
    let total = report.hypotheses_evaluated;
    let decisive = report.kept.len() + report.discarded.len();
    j.confidence = if total == 0 { 0.0 } else { decisive as f64 / total as f64 };

    // Residual unknowns: domains with unknown decision.
    j.residual_unknowns = report
        .entries
        .iter()
        .filter(|e| matches!(e.status, cynic_protocol::autoresearch::KeepDiscard::Unknown))
        .map(|e| format!("unknown_decision:{}", e.domain))
        .collect();

    // If everything is unknown, downgrade to Complex.
    if report.kept.is_empty() && report.discarded.is_empty() && !report.entries.is_empty() {
        j.cynefin_domain = CynefinDomain::Complex;
    }

    // If we kept something with high confidence, upgrade to Clear.
    if !report.kept.is_empty() && j.confidence > 0.7 {
        j.cynefin_domain = CynefinDomain::Clear;
    }

    j
}

fn error_judgment(reason: &str) -> CynicJudgment {
    use cynic_eval::judgment::{CynefinDomain, DijkwLayer, SagePhase, CynicJudgment};

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut j = CynicJudgment::new(
        format!("autoresearch-error-{now}"),
        false,
        DijkwLayer::Data,
        SagePhase::Priming,
        CynefinDomain::Disordered,
        now,
    );
    j.residual_unknowns = vec![reason.to_string()];
    j.confidence = 0.0;
    j
}

fn load_program(path: &str) -> Result<AutoresearchProgram, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse failed: {e}"))
}

fn persist_report(report: AutoresearchReport, output_dir: &str) {
    let _ = std::fs::create_dir_all(output_dir);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = Path::new(output_dir).join(format!("autoresearch-report-{now}.json"));
    let _ = serde_json::to_vec_pretty(&report).map(|bytes| {
        let _ = std::fs::write(&path, bytes);
    });
}
