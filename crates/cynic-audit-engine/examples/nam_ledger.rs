//! Generate `infra/retired-metaphor-ledger.json` from a live observation.
//!
//! The baseline is written from `retired_metaphor_rule::observe`, the SAME
//! function the rule compares against. A baseline recorded from a different
//! observation would fail for reasons unrelated to the vocabulary.
//!
//! Run it ONLY as a deliberate act, when the observed state is the state
//! worth recording. A routine run would ratify whatever the workspace
//! happens to contain, which is exactly what a shrink-only baseline is
//! supposed to prevent.
//!
//! Usage: cargo run -p cynic-audit-engine --example nam_ledger -- <root>

use std::collections::BTreeMap;
use std::path::PathBuf;

use cynic_audit_engine::retired_metaphor_rule::observe;
use cynic_protocol::vocabulary::{
    INFRASTRUCTURE_COMPOSED_IDENTIFIERS, RETIRED_METAPHORS,
};
use cynic_audit_engine::scanner::scan_workspace;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let index = scan_workspace(&root);
    let observed = observe(&index);

    let mut entries: Vec<(String, usize, String, String)> = Vec::new();
    let mut by_term: BTreeMap<String, usize> = BTreeMap::new();

    for (path, (count, evidence, terms)) in &observed {
        let term_list = terms.join(", ");
        for term in terms {
            *by_term.entry(term.to_ascii_lowercase()).or_default() += count;
        }
        entries.push((path.clone(), *count, evidence.clone(), term_list));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let total: usize = entries.iter().map(|e| e.1).sum();

    let mut document = String::new();
    document.push_str("{\n");
    document.push_str("  \"schema\": \"cynic.retired_metaphor_ledger.v1\",\n");
    document.push_str("  \"status\": \"normative\",\n");
    document.push_str(
        "  \"purpose\": \"Declared, counted, shrink-only baseline of the biological \
         metaphors still present in the domain vocabulary. Read by \
         audit_engine::retired_metaphor_rule (NAM-001). A file with no entry here \
         fails the gate; an entry with no observed occurrence also fails, because paid \
         debt left in the list hides that the list is stale.\",\n",
    );
    document.push_str("  \"rule\": {\n");
    document.push_str(
        "    \"growth\": \"blocked -- a new file, or a count above the recorded one, \
         fails\",\n",
    );
    document.push_str(
        "    \"shrink\": \"required -- an entry with no observed occurrence must be \
         deleted\",\n",
    );
    document.push_str(
        "    \"why_not_a_threshold\": \"A threshold is a policy that erodes: it tolerates \
         the next occurrence by construction. An exact per-file count does not, because \
         any increase is a distinct, named failure.\"\n",
    );
    document.push_str("  },\n");
    document.push_str("  \"vocabulary\": {\n");
    // The vocabulary block is DERIVED from the canonical mapping, never
    // typed here: a second copy of the list is a second place to be wrong,
    // and a baseline that names a term the rule does not know is a
    // baseline that describes a rule that does not exist.
    let vocabulary_lines: Vec<String> = RETIRED_METAPHORS
        .iter()
        .map(|mapping| format!("    \"{}\": \"{}\"", mapping.retired, mapping.canonical))
        .collect();
    document.push_str(&vocabulary_lines.join(",\n"));
    document.push_str("\n  },\n");
    document.push_str("  \"exempt_infrastructure_identifiers\": {\n");
    // The service plane was exempt here while it was still the live plane
    // name. The plane is canonically `service` now, so the exemption would
    // describe a name that no longer exists and would hide a retired term
    // from the baseline. `cortex` remains: it is the operator workstation
    // hostname.
    document.push_str(
        "    \"cortex\": \"the operator workstation hostname (cynic-host-cortex); \
         renaming it would break host addressing\"\n",
    );
    document.push_str("  },\n");
    document.push_str("  \"exempt_composed_identifiers\": {\n");
    // Also derived: a name that merely CONTAINS a retired term is a
    // machine or a service domain, not a metaphor.
    let composed_lines: Vec<String> = INFRASTRUCTURE_COMPOSED_IDENTIFIERS
        .iter()
        .map(|identifier| {
            format!("    \"{identifier}\": \"a live name, exempted by whole identifier\"")
        })
        .collect();
    document.push_str(&composed_lines.join(",\n"));
    document.push_str("\n  },\n");
    document.push_str("  \"measured\": {\n");
    document.push_str(
        "    \"note\": \"Observation at ratification. Regenerated only by deliberate act.\",\n",
    );
    document.push_str(&format!("    \"files\": {},\n", entries.len()));
    document.push_str(&format!("    \"occurrences\": {},\n", total));
    document.push_str("    \"by_term\": {\n");
    let term_lines: Vec<String> = by_term
        .iter()
        .map(|(term, count)| format!("      \"{term}\": {count}"))
        .collect();
    document.push_str(&term_lines.join(",\n"));
    document.push_str("\n    }\n");
    document.push_str("  },\n");
    document.push_str("  \"entries\": [\n");
    let entry_lines: Vec<String> = entries
        .iter()
        .map(|(path, count, evidence, terms)| {
            format!(
                "    {{\"path\": \"{path}\", \"count\": {count}, \"evidence\": \"{evidence}\", \
                 \"terms\": \"{terms}\"}}"
            )
        })
        .collect();
    document.push_str(&entry_lines.join(",\n"));
    document.push_str("\n  ]\n}\n");

    let out = root.join("infra/retired-metaphor-ledger.json");
    std::fs::write(&out, document).expect("write the ledger");
    println!(
        "wrote {} with {} files and {} occurrences",
        out.display(),
        entries.len(),
        total
    );
}
