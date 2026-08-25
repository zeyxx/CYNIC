//! Schema registry audit: inventories every versioned `cynic.*.vN` schema
//! literal in workspace Rust sources and derives fragmentation facts from that
//! scan — never from an editorial list.
//!
//! This is the observe step for canonical schema-registry work: before a
//! registry can own schema identity, the audit must prove how many identities
//! exist, where they are scattered, and which version families drift.
//!
//! Schema: `cynic.cli.schema_registry_audit.v1`

use serde::Serialize;

use std::collections::{BTreeMap, BTreeSet};

use crate::rust_source::{relative_path, rust_files_under, workspace_root};

pub(crate) const SCHEMA_REGISTRY_AUDIT_SCHEMA: &str = "cynic.cli.schema_registry_audit.v1";

/// Crates scanned, in canonical order. `cynic-protocol` first: it is the
/// intended canonical home for wire contracts (protocol_is_pure law).
const SCHEMA_SCAN_CRATES: &[&str] = &[
    "cynic-protocol",
    "cynic-host",
    "cynic-node",
    "cynic-kernel",
    "cynic-inference",
    "cynic-cli",
    "cynic-askesis",
    "cynic-observer",
    "cynic-intel",
    "cynic-zeroclaw-solana",
];

#[derive(Serialize)]
pub(crate) struct SchemaRegistryAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) summary: SchemaRegistrySummary,
    /// Version families carrying more than one live version at once — each is
    /// an unresolved migration or a silent fork, and both need a decision.
    pub(crate) version_drift_families: Vec<SchemaVersionDriftFamily>,
}

#[derive(Serialize)]
pub(crate) struct SchemaRegistrySummary {
    pub(crate) unique_schema_count: usize,
    pub(crate) literal_occurrence_count: usize,
    pub(crate) files_with_literals_count: usize,
    pub(crate) scattered_definition_count: usize,
    pub(crate) version_drift_family_count: usize,
    pub(crate) protocol_canonical_count: usize,
    pub(crate) scan_unobserved: bool,
}

#[derive(Serialize)]
pub(crate) struct SchemaVersionDriftFamily {
    pub(crate) base_name: String,
    pub(crate) versions: Vec<String>,
    pub(crate) evidence_paths: Vec<String>,
}

#[derive(Default)]
struct SchemaScan {
    /// schema id -> ordered set of files carrying its literal
    by_schema: BTreeMap<String, Vec<String>>,
    literal_occurrences: usize,
    files_with_literals: BTreeSet<String>,
    unobserved_crates: Vec<String>,
}

impl SchemaScan {
    fn scan_workspace() -> Self {
        let mut scan = Self::default();
        let crates_root = workspace_root().join("crates");
        let root_observable = crates_root.exists();

        for crate_name in SCHEMA_SCAN_CRATES {
            let crate_src = crates_root.join(crate_name).join("src");
            if !crate_src.exists() {
                // Unknown is data: an absent crate source must not silently
                // shrink the inventory into false confidence.
                if root_observable {
                    scan.unobserved_crates.push(crate_name.to_string());
                }
                continue;
            }
            for file in rust_files_under(&crate_src) {
                if file
                    .file_name()
                    .is_some_and(|name| name == "schema_registry_audit.rs")
                {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&file) else {
                    continue;
                };
                for schema_id in extract_schema_ids(&content) {
                    let rel = relative_path(&crate_src, &file);
                    scan.literal_occurrences += 1;
                    scan.files_with_literals
                        .insert(format!("{crate_name}/{rel}"));
                    let entry = scan.by_schema.entry(schema_id).or_default();
                    if !entry.contains(&format!("{crate_name}/{rel}")) {
                        entry.push(format!("{crate_name}/{rel}"));
                    }
                }
            }
        }

        if !root_observable {
            scan.unobserved_crates.push("workspace_root".to_string());
        }
        scan
    }

    fn scattered(&self) -> Vec<(String, Vec<String>)> {
        self.by_schema
            .iter()
            .filter(|(_, files)| files.len() > 2)
            .map(|(schema_id, files)| (schema_id.clone(), files.clone()))
            .collect()
    }

    fn version_drift_families(&self) -> Vec<SchemaVersionDriftFamily> {
        let mut families: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for (schema_id, files) in &self.by_schema {
            let Some((base_name, version)) = schema_id.rsplit_once(".v") else {
                continue;
            };
            if !version.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            families
                .entry(base_name.to_string())
                .or_default()
                .entry(version.to_string())
                .or_default()
                .extend(files.iter().cloned());
        }

        families
            .into_iter()
            .filter_map(|(base_name, versions)| {
                if versions.len() < 2 {
                    return None;
                }
                Some(SchemaVersionDriftFamily {
                    base_name,
                    versions: versions.keys().cloned().collect(),
                    evidence_paths: versions
                        .into_values()
                        .flatten()
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                })
            })
            .collect()
    }
}

/// A schema id is `cynic.<segments...>.v<N>`: lowercase/digit/underscore
/// segments, ending in `.v` + digits only.
fn is_versioned_schema_id(candidate: &str) -> bool {
    let Some((_, version)) = candidate.rsplit_once(".v") else {
        return false;
    };
    !version.is_empty()
        && version.chars().all(|c| c.is_ascii_digit())
        && candidate
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
}

/// Extracts distinct `cynic.<segment>.vN` literals from one source file.
fn extract_schema_ids(content: &str) -> Vec<String> {
    const LEAD: &str = "\"cynic.";
    let mut found = BTreeSet::new();
    let mut cursor = 0usize;

    while let Some(rel) = content[cursor..].find(LEAD) {
        let start = cursor + rel + 1; // first char after the opening quote
        let Some(end) = content[start..].find('"') else {
            break;
        };
        let candidate = &content[start..start + end];
        if is_versioned_schema_id(candidate) {
            found.insert(candidate.to_string());
        }
        cursor = start + end + 1;
    }

    found.into_iter().collect()
}

pub(crate) fn audit_schema_registry() -> SchemaRegistryAuditReport {
    let scan = SchemaScan::scan_workspace();
    let scan_unobserved = !scan.unobserved_crates.is_empty();

    let scattered = scan.scattered();
    let drift = scan.version_drift_families();
    let unique_schema_count = scan.by_schema.len();
    let protocol_canonical_count = scan
        .by_schema
        .values()
        .filter(|files| files.iter().any(|path| path.starts_with("cynic-protocol/")))
        .count();

    // Status law: unobservable scan -> blocked_unobserved. Any active version
    // drift -> fragmented (two live versions of one identity is a silent fork
    // until someone decides). Otherwise ok.
    let status = if scan_unobserved {
        "blocked_unobserved"
    } else if !drift.is_empty() {
        "fragmented"
    } else {
        "ok"
    };

    SchemaRegistryAuditReport {
        schema: SCHEMA_REGISTRY_AUDIT_SCHEMA,
        status,
        summary: SchemaRegistrySummary {
            unique_schema_count,
            literal_occurrence_count: scan.literal_occurrences,
            files_with_literals_count: scan.files_with_literals.len(),
            scattered_definition_count: scattered.len(),
            version_drift_family_count: drift.len(),
            protocol_canonical_count,
            scan_unobserved,
        },
        // Scattered schemas are reported in full only when few enough to stay
        // readable; beyond that the count carries the signal and the JSON keeps
        // the report bounded.
        version_drift_families: drift,
    }
}

#[cfg(test)]
mod tests {
    use super::audit_schema_registry;

    #[test]
    fn reports_expected_schema_and_counts_summarize_the_scan() {
        let report = audit_schema_registry();
        assert_eq!(report.schema, "cynic.cli.schema_registry_audit.v1");
        assert!(report.summary.unique_schema_count > 100);
        assert!(report.summary.files_with_literals_count > 50);
    }

    #[test]
    fn status_law_is_derived_not_asserted() {
        let report = audit_schema_registry();
        if report.summary.scan_unobserved {
            assert_eq!(report.status, "blocked_unobserved");
        } else if report.summary.version_drift_family_count > 0 {
            assert_eq!(report.status, "fragmented");
        } else {
            assert_eq!(report.status, "ok");
        }
    }

    #[test]
    fn known_live_version_drift_stays_visible_until_resolved() {
        let report = audit_schema_registry();
        let journal = report
            .version_drift_families
            .iter()
            .find(|family| family.base_name == "cynic.evidence.journal_entry")
            .expect("journal_entry drift family must be visible until resolved");
        assert!(
            journal.versions.contains(&"1".to_string())
                && journal.versions.contains(&"9".to_string()),
            "v1 and v9 are observed live; resolution must update this expectation"
        );
    }
}
