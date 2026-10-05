//! `PROSE-001` — code must not cite markdown as authority.
//!
//! Constitutional boundary `code_cites_prose` (foundation.json): a normative rule
//! that code depends on must live in a contract, a Rust type, or a test — never
//! in markdown. A Rust source may not cite a `.md` path as `Authority:` or
//! `Specifications:`.
//!
//! Markdown is for plans, designs, and audits. It is normative for humans and
//! carries zero execution authority. A prose citation has no existence check, so
//! a phantom section goes undetected — the five files citing an `AGENTS.md secret
//! classes table` that does not exist prove this.
//!
//! This rule scans Rust source for doc comments and string literals that declare
//! `Authority:` or `Specifications:` followed by a `.md` path. It reports each
//! violation with the file, line, and cited path.

use std::collections::BTreeSet;

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::{SourceFile, WorkspaceSourceIndex};

/// `PROSE-001` — a Rust source may not cite a `.md` path as authority.
///
/// The rule is deliberately narrow: it fires only when a `.md` path appears in
/// the same doc comment or string literal as `Authority:` or `Specifications:`.
/// A markdown path cited without an authority declaration is not a violation —
/// it may be a reference, a link, or a note.
pub struct CodeCitesProseRule;

impl AuditRule for CodeCitesProseRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "PROSE-001".to_string(),
            name: "code_cites_prose".to_string(),
            description:
                "A Rust source may not cite a `.md` path as `Authority:` or `Specifications:`. \
                 Code cites machine-readable authority only; markdown is for plans, designs, \
                 and audits, never execution authority."
                    .to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();
        for file in &index.rust_files {
            // Test modules and integration tests are excluded: they legitimately
            // name paths that do not exist, and a regression fixture for this
            // very rule is one. Same convention as `quoted_paths`.
            if file.relative_path.starts_with("tests/") || file.relative_path.contains("/tests/") {
                continue;
            }
            // This file documents the rule with deliberately absent example paths.
            if file.relative_path.ends_with("audit_engine/prose_citation_rule.rs") {
                continue;
            }
            for (line_idx, line) in file.content.lines().enumerate() {
                let trimmed = line.trim();
                // Only doc comments and string literals can declare authority.
                // A bare `Authority:` in code is not a citation.
                if !(trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("//")) {
                    continue;
                }
                let lower = trimmed.to_lowercase();
                if !(lower.contains("authority:") || lower.contains("specifications:")) {
                    continue;
                }
                // Extract `.md` paths from the line.
                for path in extract_markdown_paths(trimmed) {
                    violations.push(AuditViolation {
                        code: "PROSE-001-MD-AUTHORITY".to_string(),
                        message: format!(
                            "{}:{} cites markdown as authority: `{}` — code must cite \
                             machine-readable authority (contract, Rust type, or test), \
                             never prose",
                            file.relative_path,
                            line_idx + 1,
                            path
                        ),
                        severity: AuditSeverity::Critical,
                        category: AuditCategory::Boundary,
                        span: Some(AuditSpan {
                            file_path: file.relative_path.clone(),
                            line_start: line_idx + 1,
                            line_end: line_idx + 1,
                            snippet: Some(trimmed.to_string()),
                        }),
                        remediation: Some(
                            "Move the rule into a contract, a Rust type, or a test. \
                             Markdown is for plans and designs, never execution authority."
                                .to_string(),
                        ),
                    });
                }
            }
        }
        violations
    }
}

/// Extract `.md` paths from a line of text.
///
/// Looks for backtick-quoted or bare paths ending in `.md`. Returns unique paths
/// in order of first appearance.
fn extract_markdown_paths(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    // Backtick-quoted paths: `path/to/file.md`
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        if let Some(close) = after.find('`') {
            let candidate = &after[..close];
            if candidate.ends_with(".md") && seen.insert(candidate.to_string()) {
                found.push(candidate.to_string());
            }
            rest = &after[close + 1..];
        } else {
            break;
        }
    }
    // Bare paths (not in backticks): word characters, slashes, dots, hyphens
    // ending in .md. Only if not already found in backticks.
    if found.is_empty() {
        for word in line.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_');
            if cleaned.ends_with(".md") && seen.insert(cleaned.to_string()) {
                found.push(cleaned.to_string());
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_file(path: &str, content: &str) -> SourceFile {
        SourceFile {
            relative_path: path.to_string(),
            absolute_path: std::path::PathBuf::from(path),
            line_count: content.lines().count(),
            content: content.to_string(),
        }
    }

    fn index_with(files: Vec<SourceFile>) -> WorkspaceSourceIndex {
        WorkspaceSourceIndex {
            rust_files: files,
            json_files: Vec::new(),
            manifests: Vec::new(),
            existing_paths: BTreeSet::new(),
            gitignore_patterns: Vec::new(),
            host: None,
            unit_files: Vec::new(),
            unit_contents: std::collections::BTreeMap::new(),
            exec_starts: Vec::new(),
            claim_store: None,
        }
    }

    #[test]
    fn detects_markdown_cited_as_authority() {
        let file = source_file(
            "crates/cynic-secrets/src/protocol.rs",
            "/// Authority: AGENTS.md secret classes table\npub struct Foo;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, "PROSE-001-MD-AUTHORITY");
        assert!(violations[0].message.contains("AGENTS.md"));
    }

    #[test]
    fn detects_specifications_keyword() {
        let file = source_file(
            "crates/cynic-host/src/foo.rs",
            "/// Specifications: protocols/machine-enrollment-federation.md\npub struct Bar;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("machine-enrollment-federation.md"));
    }

    #[test]
    fn ignores_markdown_without_authority_declaration() {
        let file = source_file(
            "crates/cynic-host/src/foo.rs",
            "/// See docs/design/architecture.md for details.\npub struct Baz;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn ignores_test_modules() {
        let file = source_file(
            "crates/cynic-host/tests/integration.rs",
            "/// Authority: AGENTS.md secret classes table\npub struct Qux;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn ignores_self_documentation() {
        let file = source_file(
            "crates/cynic-host/src/audit_engine/prose_citation_rule.rs",
            "/// Authority: AGENTS.md secret classes table\npub struct Quux;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn extracts_multiple_paths_from_one_line() {
        let file = source_file(
            "crates/cynic-host/src/foo.rs",
            "/// Authority: `AGENTS.md` and `protocols/federation.md`\npub struct Corge;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn bare_path_without_backticks_is_detected() {
        let file = source_file(
            "crates/cynic-host/src/foo.rs",
            "/// Authority: AGENTS.md\npub struct Grault;",
        );
        let index = index_with(vec![file]);
        let rule = CodeCitesProseRule;
        let violations = rule.evaluate(&index);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("AGENTS.md"));
    }
}
