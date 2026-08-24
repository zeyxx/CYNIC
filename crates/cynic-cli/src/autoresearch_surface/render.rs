//! Render layer for the autoresearch surface.
//!
//! Text projection of an autoresearch report + judgment. No domain logic here.

use crate::autoresearch_surface::types::AutoresearchReport;

pub(crate) fn render_report(report: &AutoresearchReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Autoresearch Report\n  \
         Schema:    {}\n  \
         Sessions:  {}\n  \
         Evaluated: {}\n  \
         Kept:      {}\n  \
         Discarded: {}\n  \
         Unknown:   {}\n",
        report.schema,
        report.summary.corpus_sessions,
        report.summary.hypotheses_evaluated,
        report.summary.kept,
        report.summary.discarded,
        report.summary.unknown,
    ));

    if let Some(judgment) = &report.judgment {
        out.push_str(&format!(
            "  \
             Judgment:  {}\n  \
             DIKW:      {:?}\n  \
             SAGE:      {:?}\n  \
             Cynefin:   {:?}\n  \
             Actionable: {}\n  \
             Confidence: {:.1}%\n",
            judgment.judgment_id,
            judgment.dijkw_layer,
            judgment.sage_phase,
            judgment.cynefin_domain,
            if judgment.is_actionable() {
                "YES"
            } else {
                "NO"
            },
            judgment.confidence * 100.0,
        ));
    }

    if !report.entries.is_empty() {
        out.push_str("\n  Entries:\n");
        for entry in &report.entries {
            out.push_str(&format!(
                "    - {} (domain: {}, gain: {:.2}, status: {:?})\n",
                entry.skill_id, entry.domain, entry.gain, entry.status,
            ));
        }
    }

    out
}
