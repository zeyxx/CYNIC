//! Path reference rules — the declared arborescence versus the observed tree.
//!
//! WHY: every other rule in this engine audits *form* — module reachability,
//! line counts, layer edges, ownership markers. None of them audit *reference*.
//! A path can be declared by a normative contract, cited from production Rust,
//! and point at nothing, and the repository stays green. That is the one failure
//! class with no detector, and it is silent: an unreadable tree still reports
//! zero findings.
//!
//! HOW: the scanner observes reality (`WorkspaceSourceIndex::existing_paths` for
//! what exists, `gitignore_patterns` for what git is told to discard). These
//! rules are pure functions over that observation. No rule performs I/O.
//!
//! FALSE POSITIVES are the failure mode that kills an audit, so extraction is
//! deliberately narrow: a candidate is validated only when its first path
//! segment is a real top-level entry of this repository. A string like
//! `source/repositories/CYNIC` or `lab/images/ubuntu.img` lives in the declared
//! *runtime* space (`/var/lib/cynic`), not the git tree, and is not this rule's
//! business. Anything absolute, templated, or rooted outside the git tree is
//! reported as `runtime_space` and never counted as a violation.

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::{SourceFile, WorkspaceSourceIndex};

/// The path SSOT. `infra/directory-tree.json` is rank 4 in the authority order
/// and declares itself the single source of truth for all filesystem paths.
const PATH_SSOT: &str = "infra/directory-tree.json";

/// Top-level directories of the git tree that may anchor a validated path.
///
/// Deliberately explicit rather than derived: deriving it would accept any
/// string, which is precisely the false-positive failure this module exists to
/// avoid. Dot-directories are included because a cited a cited `.cortex` or `.hermes` path that resolves
/// to nothing is a real divergence, and because
/// the scanner observes them even though it does not read their content.
const REPO_DIRS: &[&str] = &[
    ".agents",
    ".cortex",
    ".github",
    ".hermes",
    "apps",
    "archive",
    "bounty",
    "contracts",
    "crates",
    "docs",
    "evidence",
    "infra",
    "programs",
    "protocols",
    "recruiter",
    "references",
    "scripts",
    "specs",
    "tests",
    "vault",
    "verify",
];

/// Top-level files of the git tree that may be declared whole.
const REPO_FILES: &[&str] = &["Cargo.toml", "Makefile", "rust-toolchain.toml"];

/// Authority files whose declared paths are checked for existence.
const AUTHORITY_FILES: &[&str] = &[
    "contracts/build-placement.json",
    "contracts/foundation.json",
    "contracts/framework.json",
    "contracts/observability-loop.json",
    "contracts/source-lifecycle.json",
    "infra/directory-tree.json",
    "infra/registry.json",
    "infra/runtime-authority.json",
];

/// A candidate path extracted from a source file, with its provenance.
struct Candidate {
    path: String,
    file: String,
    line: usize,
}

impl Candidate {
    fn violation(
        &self,
        code: &str,
        message: String,
        remediation: &str,
    ) -> AuditViolation {
        AuditViolation {
            code: code.to_string(),
            message,
            severity: AuditSeverity::Critical,
            category: AuditCategory::PathSanitation,
            span: Some(AuditSpan {
                file_path: self.file.clone(),
                line_start: self.line,
                line_end: self.line,
                snippet: Some(self.path.clone()),
            }),
            remediation: Some(remediation.to_string()),
        }
    }
}

/// Reduce a raw string to a validated repository-relative path, or `None`.
///
/// The gate is the first segment: it must be a real top-level directory or a
/// real top-level file. That single test rejects every absolute, templated, and
/// runtime-space path, and it is why this rule can claim zero false positives on
/// the current tree.
///
/// Trailing slashes are normalised away so that a directory declared as `infra/`
/// is compared against the observed `infra` rather than reported missing.
fn validate(raw: &str) -> Option<String> {
    let candidate = raw.trim().trim_matches('"').trim().trim_end_matches('/');
    if candidate.is_empty() || candidate.len() > 200 {
        return None;
    }
    // Absolute, home-relative, templated, globbed, or multi-line: not a git path.
    if candidate.starts_with('/')
        || candidate.starts_with('~')
        || candidate.starts_with('$')
        || candidate.contains("${")
        || candidate.contains('*')
        || candidate.contains(' ')
        || candidate.contains('\n')
    {
        return None;
    }
    // A format placeholder is a template, not a declaration: `archive/{}` is not
    // a path, it is a `format!` with a missing argument.
    if candidate.contains('{') || candidate.contains('}') {
        return None;
    }
    // A Rust item path that happens to begin with a crate-directory name, such
    // as `crates/cynic-protocol::agent_workload`, is a module reference, not a
    // filesystem path.
    if candidate.contains("::") {
        return None;
    }
    // A JSON pointer or dotted key appended to a path — `infra/registry.json:x.y`
    // — addresses inside the file, not a file on disk.
    if let Some((head, _pointer)) = candidate.split_once(':') {
        return validate(head);
    }
    // A substitution placeholder such as `known-peers.<ts>.json` is a template.
    // The absence of its expansion is not waste.
    if candidate.contains('<') || candidate.contains('>') {
        return None;
    }
    // Parent traversal cannot be resolved against the observed set without
    // normalisation this rule deliberately does not perform, so it is rejected
    // rather than guessed at.
    if candidate.split('/').any(|segment| segment == "..") {
        return None;
    }
    let first = candidate.split('/').next()?;
    // Both lists are tested because normalisation has already removed the
    // trailing separator: `infra/` and `Cargo.toml` are equally single-segment
    // by now, and both are real top-level entries. `source` and `README.md` are
    // in neither, which is what keeps runtime space and stray prose out.
    (REPO_DIRS.contains(&first) || REPO_FILES.contains(&first)).then(|| candidate.to_string())
}


/// Paths the authority places outside the git tree.
///
/// `infra/directory-tree.json` is explicit about this: the `git_sources` layer
/// "live[s] in the git repository, never on the runtime filesystem", while
/// `installed_system` and the runtime layers do not. A path declared in a
/// non-git layer is *supposed* to be absent here, so its absence is not waste —
/// it is the contract working. Reading this from the SSOT rather than from a
/// hardcoded list is the whole point: the answer changes when the contract does.
fn runtime_space(index: &WorkspaceSourceIndex) -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    let Some(ssot) = index
        .json_files
        .iter()
        .find(|file| file.relative_path == PATH_SSOT)
    else {
        return found;
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(&ssot.content) else {
        return found;
    };
    let Some(layers) = document.get("layers").and_then(|v| v.as_object()) else {
        return found;
    };
    for (layer, body) in layers {
        if layer == "git_sources" {
            continue;
        }
        collect_relative_paths(body, &mut found);
    }
    found
}

/// Recursively collect every relative path-like string in a JSON value.
fn collect_relative_paths(value: &serde_json::Value, found: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::String(text) => {
            let candidate = text.trim().trim_end_matches('/');
            if candidate.contains('/')
                && !candidate.starts_with('/')
                && !candidate.contains(' ')
                && !candidate.contains('*')
                && !candidate.contains('<')
            {
                found.insert(candidate.to_string());
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_relative_paths(item, found);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, item) in fields {
                // Keys declare paths too, exactly as in the git layer, and they
                // carry the same trailing-slash convention as the values:
                // `"vault/models/"` and `"vault/models"` are one path.
                let key = key.trim_end_matches('/');
                if key.contains('/') && !key.contains(' ') && !key.contains('<') {
                    found.insert(key.to_string());
                }
                collect_relative_paths(item, found);
            }
        }
        _ => {}
    }
}

/// Does a cited path resolve, allowing for the three ways a real citation differs
/// from a bare repo-relative string?
///
/// 1. The declaring contract places it in runtime space.
/// 2. It omits a crate and/or `src/` prefix, as a crate-internal reference does.
/// 3. It names a module directory where the build expects `mod.rs`.
fn resolves(index: &WorkspaceSourceIndex, path: &str, citing_file: &str) -> bool {
    if index.existing_paths.contains(path) {
        return true;
    }
    // 3. `crates/cynic-kernel/src/infra/tasks` is the directory; the build
    //    expects the module file beside it.
    if let Some(stem) = path.strip_suffix(".rs") {
        let as_module = format!("{stem}/mod.rs");
        if index.existing_paths.contains(&as_module) || index.existing_paths.contains(stem) {
            return true;
        }
    } else if index.existing_paths.contains(&format!("{path}/mod.rs")) {
        return true;
    }
    // 2. The citation omits a crate and/or `src/` prefix. A repo-internal
    //    reference may be written relative to its crate, so a suffix match on
    //    any observed path counts as a resolution. This is what separates
    //    `infra/tasks` (a real module) from `infra/reconnectable_storage.rs`
    //    (gone from every crate in the tree).
    let needle = format!("/{path}");
    index
        .existing_paths
        .iter()
        .any(|existing| existing.ends_with(&needle))
}

/// Every delimited span on a line, with its line number, that validates as a path.
///
/// Shared by both sources because the distinction that matters is not
/// key-versus-value: a path registry declares paths in both positions
/// (`"key_files": { "infra/registry.json": … }` is as much a declaration as any
/// value), and restricting to values silently missed half the surface.
///
/// Both `"` and `` ` `` are scanned. In Rust the dominant citation form is a
/// backticked path inside a doc comment — ``// Authority: `protocols/h1-intent.md` §Digest ambiguity`` —
/// so scanning only double quotes misses most of what the rule exists to find.
/// `` ` `` spans that are not paths (`crate::module::Type`, code fences) are
/// rejected by `validate`.
fn quoted_paths(file: &SourceFile) -> Vec<Candidate> {
    let mut found = Vec::new();
    // Test modules legitimately name paths that do not exist: a regression
    // fixture for this very rule is one. They are excluded from the scan, which
    // is why the rule can converge to zero rather than reporting itself forever.
    // Integration tests are excluded for the same reason, and this matches the
    // convention `HotspotThresholdRule` already uses for the same file class.
    if file.relative_path.contains("/tests/") {
        return found;
    }
    // This file documents the rule with deliberately absent example paths, so it
    // reports itself unless excluded. A detector that flags its own fixtures
    // cannot converge, and a gate that cannot converge gets switched off.
    if file.relative_path.ends_with("audit_engine/path_rules.rs") {
        return found;
    }
    let mut in_test_module = false;
    for (index, line) in file.content.lines().enumerate() {
        if line.contains("#[cfg(test)]") {
            in_test_module = true;
        }
        if in_test_module {
            continue;
        }
        for delimiter in ['"', '`'] {
            let mut rest = line;
            while let Some(open) = rest.find(delimiter) {
                let after = &rest[open + 1..];
                let Some(close) = after.find(delimiter) else {
                    break;
                };
                if let Some(path) = validate(&after[..close]) {
                    found.push(Candidate {
                        path,
                        file: file.relative_path.clone(),
                        line: index + 1,
                    });
                }
                rest = &after[close + 1..];
            }
        }
    }
    found
}


/// `PATH-001` — a path declared by a normative contract must exist.
///
/// This is the rule that makes `infra/directory-tree.json` falsifiable. Today it
/// reports `crates/cynic-host/src/lifecycle/runtime_paths.rs`, a path the
/// canonical registry declares and the repository does not contain.
pub struct DeclaredPathExistenceRule;

impl AuditRule for DeclaredPathExistenceRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "PATH-001".to_string(),
            name: "declared_path_existence".to_string(),
            description:
                "Every repository-relative path declared by a normative authority file must exist \
                 in the working tree. A declared path that resolves to nothing is a silent \
                 divergence between the contract and observed state."
                .to_string(),
            category: AuditCategory::PathSanitation,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();
        let runtime = runtime_space(index);
        for file in &index.json_files {
            if !AUTHORITY_FILES.contains(&file.relative_path.as_str()) {
                continue;
            }
            for candidate in quoted_paths(file) {
                if index.existing_paths.contains(&candidate.path) {
                    continue;
                }
                // Declared outside the git tree on purpose: not waste.
                if runtime.contains(&candidate.path) {
                    continue;
                }
                violations.push(candidate.violation(
                    "PATH-001-DECLARED-ABSENT",
                    format!(
                        "{} declares path {} which does not exist in the working tree",
                        file.relative_path, candidate.path
                    ),
                    "Correct the declaring contract, or materialise the path it names.",
                ));
            }
        }
        violations
    }
}

/// `PATH-002` — a repository path cited from production Rust must exist.
///
/// A literal in a `//` comment is treated as a load-bearing claim, because the
/// repository's own convention is to cite `Authority: <path>` in comments and
/// then honour it. Comments are the majority of citations here; excluding them
/// would hide the whole failure class.
pub struct RustPathReferenceRule;

impl AuditRule for RustPathReferenceRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "PATH-002".to_string(),
            name: "rust_path_reference".to_string(),
            description:
                "Every repository-relative path literal in Rust source, including doc comments, \
                 must resolve. A default constructed from an absent directory is a live path into \
                 nothing."
                .to_string(),
            category: AuditCategory::PathSanitation,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();
        let runtime = runtime_space(index);
        for file in &index.rust_files {
            for candidate in quoted_paths(file) {
                if runtime.contains(&candidate.path) {
                    continue;
                }
                if resolves(index, &candidate.path, &file.relative_path) {
                    continue;
                }
                violations.push(candidate.violation(
                    "PATH-002-CITED-ABSENT",
                    format!(
                        "{}:{} cites path {} which does not exist in the working tree",
                        candidate.file, candidate.line, candidate.path
                    ),
                    "Correct the citation, or materialise the path it names.",
                ));
            }
        }
        violations
    }
}

/// `PATH-003` — the path SSOT must not be listed in `.gitignore`.
///
/// The file is currently tracked, so the ignore rule is inert today. It is not
/// inert after a `git rm --cached`, a re-add, or any materialisation outside
/// this worktree, at which point rank 4 of the authority order vanishes with no
/// error. A tracked file is not evidence that it will stay tracked.
pub struct PathSsotIgnoreRule;

impl AuditRule for PathSsotIgnoreRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "PATH-003".to_string(),
            name: "path_ssot_not_ignored".to_string(),
            description:
                "The canonical path registry must not appear in .gitignore. Being tracked today \
                 does not make the ignore rule inert tomorrow."
                .to_string(),
            category: AuditCategory::PathSanitation,
            severity: AuditSeverity::Critical,
            automated_remediation_available: true,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        // Nothing to check if the SSOT is absent: PATH-001 already reports that.
        if !index.existing_paths.contains(PATH_SSOT) {
            return Vec::new();
        }
        index
            .gitignore_patterns
            .iter()
            .filter_map(|pattern| {
                let bare = pattern.trim_start_matches('/').trim_end_matches('/');
                let ignores_ssot = bare == PATH_SSOT
                    || bare == "directory-tree.json"
                    || bare == "infra/";
                ignores_ssot.then(|| AuditViolation {
                    code: "PATH-003-SSOT-IGNORED".to_string(),
                    message: format!(
                        ".gitignore pattern {pattern} matches the canonical path registry {PATH_SSOT}"
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::PathSanitation,
                    span: Some(AuditSpan {
                        file_path: ".gitignore".to_string(),
                        line_start: 1,
                        line_end: 1,
                        snippet: Some(pattern.clone()),
                    }),
                    remediation: Some(
                        "Remove the pattern. The path SSOT is versioned desired state, not a runtime artifact."
                            .to_string(),
                    ),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Candidate, validate};

    #[test]
    fn accepts_repository_relative_paths() {
        assert_eq!(
            validate("crates/cynic-host/src/lib.rs").as_deref(),
            Some("crates/cynic-host/src/lib.rs")
        );
        assert_eq!(validate("contracts/foundation.json").as_deref(), Some("contracts/foundation.json"));
    }

    #[test]
    fn rejects_runtime_space_paths() {
        // The declared runtime tree must never be reported as a missing git path.
        assert!(validate("/var/lib/cynic/evidence").is_none());
        assert!(validate("/etc/cynic/secrets").is_none());
        assert!(validate("~/cynic/vault").is_none());
        assert!(validate("${GIT_WORKING_TREE}/crates").is_none());
    }

    #[test]
    fn rejects_non_paths() {
        assert!(validate("cynic_protocol::AuditSeverity").is_none());
        assert!(validate("source/repositories/CYNIC").is_none());
        assert!(validate("crates/*").is_none());
        assert!(validate("").is_none());
        assert!(validate("mode").is_none());
        // A directory declared with a trailing slash must normalise, not vanish
        // and not be reported missing.
        assert_eq!(validate("infra/").as_deref(), Some("infra"));
        // Single-segment declarations are limited to real top-level files.
        assert_eq!(validate("Cargo.toml").as_deref(), Some("Cargo.toml"));
        assert!(validate("README.md").is_none());
        // Format placeholders and parent traversal are templates, not paths.
        assert!(validate("archive/{}").is_none());
        assert!(validate("crates/../Cargo.toml").is_none());
        // A Rust item path that starts with a crate-directory name is a module
        // reference. This exact shape is emitted by
        // `infra/runtime-authority.json` and was a false positive.
        assert!(validate("crates/cynic-protocol::agent_workload").is_none());
    }

    #[test]
    fn candidate_span_points_at_the_citation() {
        let candidate = Candidate {
            path: "contracts/consumer-profiles".to_string(),
            file: "crates/cynic-mcp/src/helpers_split/misc.rs".to_string(),
            line: 86,
        };
        let violation = candidate.violation("CODE", "message".to_string(), "fix");
        let span = violation.span.expect("span");
        assert_eq!(span.line_start, 86);
        assert_eq!(span.file_path, "crates/cynic-mcp/src/helpers_split/misc.rs");
    }
}
