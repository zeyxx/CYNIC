//! Query-to-doctype routing for per-data-type fusion (Zero-I/O Domain Module).
//!
//! First-principles result: no single fusion weight wins on every data type
//! (measured: Rust golds want dense-heavy alpha, JSON golds want pure BM25,
//! prose golds sit in between). This module routes a query to its data type
//! from query-text signals only, so per-type alphas apply without an oracle.
//!
//! The routing rule below is the exact rule measured at 0.635 accuracy with
//! 0.620 split-half routed MRR against 0.599 best global fusion
//! (contracts/embedding-selection.json, routing analysis entry). Any rule
//! change must re-measure before replacing these defaults.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Data type of the expected answer, predicted from query text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryRoute {
    /// Code identifiers (Rust symbols, paths, signatures).
    Rs,
    /// Contract vocabulary (policies, claims, governance).
    Json,
    /// Prose questions with neither signal.
    Md,
}

/// Per-doctype MinMax fusion alphas. JSON takes pure BM25 (alpha 0.0) because
/// measured fusion only hurts JSON golds; every value stays recalibratable
/// per corpus via the v2 matrix procedure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutingTable {
    /// MinMax alpha for code-routed queries.
    pub alpha_rs: f64,
    /// MinMax alpha for contract-routed queries (0.0 routes to pure BM25).
    pub alpha_json: f64,
    /// MinMax alpha for prose-routed queries.
    pub alpha_md: f64,
}

impl RoutingTable {
    /// Alphas measured on the 85-query fixture (full-data argmax).
    pub fn measured_defaults() -> Self {
        Self {
            alpha_rs: 0.90,
            alpha_json: 0.00,
            alpha_md: 0.45,
        }
    }

    /// MinMax alpha for a routed query.
    pub fn alpha_for(&self, route: QueryRoute) -> f64 {
        match route {
            QueryRoute::Rs => self.alpha_rs,
            QueryRoute::Json => self.alpha_json,
            QueryRoute::Md => self.alpha_md,
        }
    }
}

fn code_token_hits(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == ':' && i + 1 < bytes.len() && bytes[i + 1] as char == ':' {
            return true;
        }
        if c == '(' && i + 1 < bytes.len() && bytes[i + 1] as char == ')' {
            return true;
        }
        if c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_alphabetic() {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j] as char).is_ascii_alphabetic() {
                j += 1;
            }
            if j - (i + 1) <= 4 {
                return true;
            }
        }
        if c == '_' {
            return true;
        }
        if c.is_ascii_uppercase() {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] as char).is_ascii_alphanumeric() {
                j += 1;
            }
            let word = &text[i..j];
            if word.len() > 3
                && word[1..]
                    .chars()
                    .any(|w| w.is_ascii_uppercase() || w.is_ascii_digit())
            {
                return true;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    text.contains('/') && text.chars().any(|w| w.is_ascii_alphanumeric())
}

fn contract_words() -> &'static HashMap<String, ()> {
    static WORDS: OnceLock<HashMap<String, ()>> = OnceLock::new();
    WORDS.get_or_init(|| {
        [
            "contract",
            "policy",
            "policies",
            "ownership",
            "taxonomy",
            "versioning",
            "placement",
            "publication",
            "staging",
            "governance",
            "arbitration",
            "relay",
            "federation",
            "claim",
            "claims",
            "debt",
            "pruning",
            "observability",
        ]
        .into_iter()
        .map(|w| (w.to_string(), ()))
        .collect()
    })
}

/// Routes query text to its expected answer data type.
///
/// Code tokens (snake_case, CamelCase, `::`, `()`, file extensions, paths)
/// route to [`QueryRoute::Rs`]; contract vocabulary routes to
/// [`QueryRoute::Json`]; everything else falls through to [`QueryRoute::Md`].
pub fn route_query(text: &str) -> QueryRoute {
    if code_token_hits(text) {
        return QueryRoute::Rs;
    }
    let words = contract_words();
    let hit = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 2)
        .any(|w| words.contains_key(&w.to_ascii_lowercase()));
    if hit {
        QueryRoute::Json
    } else {
        QueryRoute::Md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_code_identifiers_to_rs() {
        assert_eq!(
            route_query("GitBranchState GitTopologyObservation"),
            QueryRoute::Rs
        );
        assert_eq!(route_query("wilcoxon_signed_rank paired z"), QueryRoute::Rs);
        assert_eq!(
            route_query("resolve_runner_host ssh_build_runner"),
            QueryRoute::Rs
        );
        assert_eq!(
            route_query("crates/cynic-vcs/src/git_topology.rs"),
            QueryRoute::Rs
        );
        assert_eq!(
            route_query("ReasoningAdapter AutoresearchPort trait"),
            QueryRoute::Rs
        );
    }

    #[test]
    fn routes_contract_vocabulary_to_json() {
        assert_eq!(
            route_query("debt taxonomy roi formula frozen iteration"),
            QueryRoute::Json
        );
        assert_eq!(
            route_query("ownership contract declares boundary governed system"),
            QueryRoute::Json
        );
        assert_eq!(
            route_query("release tagging versioning public projection"),
            QueryRoute::Json
        );
    }

    #[test]
    fn routes_prose_questions_to_md() {
        assert_eq!(
            route_query("how do mesh peers reach consensus"),
            QueryRoute::Md
        );
        assert_eq!(
            route_query("who are the canonical actors of the system"),
            QueryRoute::Md
        );
        // "debt" is contract vocabulary in the measured rule, even though
        // this particular gold lives in code; the rule stays identical to
        // what was measured (0.635 accuracy), quirks included.
        assert_eq!(
            route_query("what blocks the audit when new debt is not registered"),
            QueryRoute::Json
        );
    }

    #[test]
    fn routes_mixed_shape_by_code_signal() {
        // Mixed-shape queries (identifier plus prose) split the arms; the
        // code token dominates routing, matching the measured rule.
        assert_eq!(
            route_query("HostCommandSurface how are host commands exposed to agents"),
            QueryRoute::Rs
        );
        assert_eq!(
            route_query("DnsBundleReport build_dns_bundle how are bundles assembled"),
            QueryRoute::Rs
        );
    }

    #[test]
    fn measured_defaults_match_contract() {
        let table = RoutingTable::measured_defaults();
        assert_eq!(table.alpha_for(QueryRoute::Rs), 0.90);
        assert_eq!(table.alpha_for(QueryRoute::Json), 0.00);
        assert_eq!(table.alpha_for(QueryRoute::Md), 0.45);
    }
}
