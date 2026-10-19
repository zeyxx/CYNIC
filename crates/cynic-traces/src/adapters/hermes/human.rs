//! Human intervention detection — shared, harness-agnostic, conservative.
//!
//! Strict port of `.../normalizers/_human.py`. A human turn is only tagged
//! when its text carries an explicit marker; everything else stays UNKNOWN.

use regex::Regex;
use std::sync::OnceLock;

use crate::domain::CorrectionKind;

static REJECTION: &[&str] = &[
    r"\bnon\b\s*[,.]",
    r"\bno\b\s*[,.]",
    r"\bpas\b.{0,20}\bmais\b",
    r"that'?s wrong",
    r"c'?est faux",
    r"incorrect",
    r"stop\b",
    r"arrête",
    r"revert",
    r"undo",
    r"annule",
];

static REDIRECT: &[&str] = &[
    r"\binstead\b",
    r"\bplutôt\b",
    r"\bplutot\b",
    r"au lieu de",
    r"\btry again\b",
    r"\brefais\b",
    r"\brecommence\b",
    r"\bchange\b",
    r"\buse\b.{0,20}\bnot\b",
    r"n'?utilise pas",
];

static CONSTRAINT: &[&str] = &[
    r"\bdon'?t\b",
    r"\bnever\b",
    r"\bjamais\b",
    r"\btoujours\b",
    r"\balways\b",
    r"\bmust\b",
    r"\bdoit\b",
    r"\bonly\b",
    r"\bseulement\b",
    r"interdit",
];

static APPROVAL: &[&str] = &[
    r"^ok\b",
    r"^oui\b",
    r"^yes\b",
    r"^go\b",
    r"^parfait\b",
    r"^nickel\b",
    r"^bien\b",
    r"^correct\b",
    r"^top\b",
];

fn compiled(patterns: &[&str]) -> Vec<(String, Regex)> {
    patterns
        .iter()
        .map(|p| {
            (
                (*p).to_owned(),
                Regex::new(&format!("(?i){p}")).expect("static human marker regex"),
            )
        })
        .collect()
}

fn matchers() -> &'static [(CorrectionKind, Vec<(String, Regex)>)] {
    static CELL: OnceLock<Vec<(CorrectionKind, Vec<(String, Regex)>)>> = OnceLock::new();
    CELL.get_or_init(|| {
        vec![
            (CorrectionKind::Rejection, compiled(REJECTION)),
            (CorrectionKind::Redirect, compiled(REDIRECT)),
            (CorrectionKind::Constraint, compiled(CONSTRAINT)),
            (CorrectionKind::Approval, compiled(APPROVAL)),
        ]
    })
}

/// Return `(kind, markers)`. Priority: rejection > redirect > constraint > approval.
pub fn classify_human_turn(text: &str) -> (CorrectionKind, Vec<String>) {
    let low = text.trim().to_lowercase();
    if low.is_empty() {
        return (CorrectionKind::Unknown, vec![]);
    }
    for (kind, patterns) in matchers() {
        let mut markers = Vec::new();
        for (source, re) in patterns {
            if re.is_match(&low) {
                markers.push(source.clone());
            }
        }
        if !markers.is_empty() {
            return (*kind, markers);
        }
    }
    (CorrectionKind::Unknown, vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_and_defaults() {
        assert_eq!(classify_human_turn("").0, CorrectionKind::Unknown);
        assert_eq!(classify_human_turn("   ").0, CorrectionKind::Unknown);
        assert_eq!(classify_human_turn("c'est faux, refais").0, CorrectionKind::Rejection);
        assert_eq!(classify_human_turn("plutôt comme ça").0, CorrectionKind::Redirect);
        assert_eq!(classify_human_turn("ne jamais faire ça").0, CorrectionKind::Constraint);
        assert_eq!(classify_human_turn("ok merci").0, CorrectionKind::Approval);
        assert_eq!(classify_human_turn("continue l'analyse").0, CorrectionKind::Unknown);
    }
}
