//! Render layer for the judgment surface.
//!
//! Text projection of a CynicJudgment. No domain logic here — only formatting.

use cynic_eval::judgment::{CynicJudgment, CynefinDomain, DijkwLayer, SagePhase};

/// Render a human-readable summary of the judgment.
pub(super) fn render_judgment(j: &CynicJudgment) -> String {
    let actionable = if j.is_actionable() { "YES" } else { "NO" };

    let unknowns = if j.residual_unknowns.is_empty() {
        "(none)".to_string()
    } else {
        j.residual_unknowns.join(", ")
    };

    format!(
        "CynicJudgment {}\n  \
         DIKW:      {:?}\n  \
         SAGE:      {:?}\n  \
         Cynefin:   {:?}\n  \
         Actionable: {}\n  \
         Confidence: {:.1}%\n  \
         Unknowns:  {}",
        j.judgment_id,
        layer_label(&j.dijkw_layer),
        phase_label(&j.sage_phase),
        domain_label(&j.cynefin_domain),
        actionable,
        j.confidence * 100.0,
        unknowns,
    )
}

fn layer_label(layer: &DijkwLayer) -> &str {
    match layer {
        DijkwLayer::Data => "Data (raw, unstructured)",
        DijkwLayer::Information => "Information (schema-validated)",
        DijkwLayer::Knowledge => "Knowledge (pure decision rule)",
        DijkwLayer::Wisdom => "Wisdom (metacognitive, <100%)",
        DijkwLayer::Unknown => "Unknown (forward-compat sentinel)",
    }
}

fn phase_label(phase: &SagePhase) -> &str {
    match phase {
        SagePhase::Priming => "Priming (recall past experience)",
        SagePhase::Sense => "Sense (assemble observations)",
        SagePhase::Simulate => "Simulate (play out before commit)",
        SagePhase::Act => "Act (execute with bounded authority)",
        SagePhase::Evaluate => "Evaluate (close loop via read-back)",
        SagePhase::Unknown => "Unknown (forward-compat sentinel)",
    }
}

fn domain_label(domain: &CynefinDomain) -> &str {
    match domain {
        CynefinDomain::Clear => "Clear (known knowns)",
        CynefinDomain::Complicated => "Complicated (known unknowns)",
        CynefinDomain::Complex => "Complex (unknown unknowns)",
        CynefinDomain::Chaotic => "Chaotic (act first)",
        CynefinDomain::Disordered => "Disordered (not yet classified)",
        CynefinDomain::Unknown => "Unknown (forward-compat sentinel)",
    }
}
