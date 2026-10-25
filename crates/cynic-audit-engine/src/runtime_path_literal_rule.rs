//! `PATH-004` — an absolute path literal in Rust must be derivable from the
//! canonical path registry.
//!
//! `infra/directory-tree.json` declares, in `anti_rules[0]`: *"No path
//! hardcoding outside this document — use runtime_paths resolution."* That rule
//! had no executor. This is it.
//!
//! ## Why the severity gradient exists
//!
//! Measured on this workspace: 275 absolute `runtime` path literals in
//! production Rust, 8 in tests. A gate that fails on all of them at once is
//! unactionable — nobody can clear 275 findings in one commit, so the gate gets
//! disabled, and a disabled gate is worse than no gate because it looks like
//! authority. So the rule reports at three levels:
//!
//! - **HIGH** — a literal in a *test* that the registry does not declare, and a
//!   literal in *production* code in a field that positions something on the
//!   fleet (`install_root`, `current_link`, `bin_dir`, `postflight_argv`, …).
//!   The production ones are the five divergences that make two components
//!   disagree about where the runtime binary lives. The test ones are the
//!   canaries: a test asserting a path the registry contradicts pins the
//!   divergence in place.
//! - **TRACK** — every other production literal. Reported as `Warning` with a
//!   monotonic guard: the count may not increase. Red on arrival, and
//!   incapable of getting redder.
//!
//! ## KNOWN LIMIT — read before trusting a test-scope finding
//!
//! This rule reads one line at a time. It therefore cannot see that a literal
//! in a fixture and the `assert_eq!` several lines below it are the two halves
//! of one statement. Consequence: a test-scope finding MAY name a legitimate
//! fixture — measured at least 4 of the 20 on this workspace
//! (`runtime_layout.rs` injecting a layout, `build_attestation.rs` writing a
//! registry fixture). That is why test-scope findings are `High` and not
//! `Critical`: this rule does not yet earn the right to block a build.
//!
//! Closing the gap needs statement grouping — collect the literal and any
//! assertion over it within one statement, then decide on the pair rather than
//! on either line. Until then, treat every test-scope finding as a lead to
//! confirm by reading the test, not as a proven violation. A gate that accuses
//! correct tests is a gate that gets switched off.
//!
//! ## What counts as a violation
//!
//! A literal is a violation when it names a *runtime placement root* — the
//! prefixes the registry itself owns. Literals that merely *inspect* a prefix
//! (a gate detecting operator-home paths, like `SEC-001`) are not violations;
//! neither is the registry's own text, which is the source of truth.
//!
//! ## What this rule deliberately does not do
//!
//! It does not rewrite code. Migration is 275 edits across 12 crates in a
//! shared worktree; doing it in the same change that starts measuring it would
//! make both untestable. Measure first, then migrate against a known number.

use std::collections::BTreeSet;

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

/// The canonical path registry. Rank 4 in the authority order.
const PATH_SSOT: &str = "infra/directory-tree.json";

/// Prefixes the registry owns. A literal under one of these is a placement
/// claim; a literal under `/usr/local/lib` or `/etc/sudoers.d` is an OS
/// convention the registry does not speak about.
///
/// `/opt/cynic` is here because production code declared
/// `install_root: "/opt/cynic/runtime"` in `cynic-protocol/src/runtime_seed.rs`
/// — a path the registry never mentions (`grep -c '/opt'` returns 0). It is a
/// placement claim about a tree that does not exist in the authority.
const OWNED_PREFIXES: &[&str] = &[
    "/var/lib/cynic",
    "/var/log/cynic",
    "/var/cache/cynic",
    "/etc/cynic",
    "/opt/cynic",
    "/usr/local/bin/cynic",
];

/// Fields that position an artifact on the fleet. A literal in one of these is
/// a coordination claim: two components holding different values for it will
/// each be locally correct and globally incompatible.
const PLACEMENT_FIELDS: &[&str] = &[
    "install_root",
    "current_link",
    "previous_link",
    "bin_dir",
    "release_dir",
    "postflight_argv",
    "dns_hosts_path",
    "dns_corefile_path",
    "dns_service_path",
    "state_root",
    "config_root",
    "policy_root",
    "home_dir",
];

pub struct RuntimePathLiteralRule;

impl AuditRule for RuntimePathLiteralRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "PATH-004".to_string(),
            name: "runtime_path_literal".to_string(),
            description:
                "An absolute runtime-placement path literal in Rust must be derivable from \
                 infra/directory-tree.json. A literal in a test that the registry does not \
                 declare is critical: it pins a divergence. A literal in a placement field is \
                 high: two components disagreeing on it are each locally correct. Other \
                 production literals are tracked with a monotonic non-regression guard."
                    .to_string(),
            category: AuditCategory::PathSanitation,
            // High, not Critical, and the reason is a known limit of this rule:
            // it reads one line at a time, so it cannot see that a literal in a
            // fixture and the `assert_eq!` three lines below it belong to the
            // same statement. Until the rule groups statements, a test-scope
            // finding MAY name a legitimate fixture. Claiming Critical here
            // would mean accusing tests that are correct, and a gate that
            // accuses correct code gets switched off. The canary this rule was
            // written for — a test asserting a value production computed, that
            // the registry contradicts — is reported and named; it is not yet
            // proven to block on its own.
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let registry_roots = declared_placement_roots(index);
        let mut violations = Vec::new();

        for file in &index.rust_files {
            // The detector's own crate necessarily contains the prefixes it
            // searches for: rule code names guarded prefixes (OWN-001),
            // documents the runtime space with examples (path_rules.rs), or
            // defines the detection patterns themselves
            // (CYNIC_BINARY_PREFIXES). Rule code is pure detection, it
            // places nothing — counting it reports self-reference. The
            // `src/bin/` tools are excluded from this exclusion: they write
            // real files, so their literals stay scrutinised.
            if file.relative_path.starts_with("crates/cynic-audit-engine/src/")
                && !file.relative_path.contains("/bin/")
            {
                continue;
            }
            // The canonical path module is the axiom, not a claim: it names
            // each owned root exactly once so every other file can reference
            // it instead of restating it. Flagging it would demand the roots
            // be named nowhere, which is the opposite of single-source-of-truth.
            if file.relative_path == "crates/cynic-types/src/paths.rs" {
                continue;
            }

            let mut cfg_test_depth: Option<i32> = None;
            let mut depth: i32 = 0;
            let mut pending_cfg_test = false;

            for (idx, line) in file.content.lines().enumerate() {
                if line.contains("cfg(test)") {
                    pending_cfg_test = true;
                }
                let opens = line.matches('{').count() as i32;
                let closes = line.matches('}').count() as i32;
                if pending_cfg_test && opens > 0 {
                    cfg_test_depth = Some(depth);
                    pending_cfg_test = false;
                }
                depth += opens - closes;
                if let Some(entry_depth) = cfg_test_depth {
                    if depth <= entry_depth {
                        cfg_test_depth = None;
                    }
                }

                let in_test_scope = cfg_test_depth.is_some();
                let in_test_file = file.relative_path.contains("/tests/");

                // A test module declared under `#[cfg(test)]` in its PARENT
                // carries no in-file `cfg(test)` marker: the attribute is on
                // the `mod foo_tests;` line in the parent, not inside the
                // module. A rule that only reads the file's own body
                // therefore classifies the module as production code and
                // reports its fixtures as placements. Five such modules
                // existed in `src/` (23 tests between them), and the
                // `admitted_tests.rs` one produced a false PATH-004
                // finding against a real, registry-declared path.
                //
                // The name is the declaration's own convention: `mod
                // admitted_tests;` under `#[cfg(test)]` in `bridge.rs`
                // names the module `admitted_tests.rs`. A file whose name
                // ends in `_tests.rs` is a test module wherever it sits.
                let is_test_module_file = file
                    .relative_path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.ends_with("_tests.rs"));

                // A line that tests or filters a prefix is inspection, not
                // placement. `SEC-001` and this rule both need to *name* the
                // paths they reject. Same for a line that rewrites one:
                // redacting `/var/lib/cynic` out of a log (`replace(prefix,
                // "[REDACTED_PATH]")`) treats the prefix as sensitive data,
                // not as a destination. And a clap `default_value` is an
                // overridable operator convention documented in --help, not
                // a coded placement: deriving CLI defaults from the registry
                // would couple argument parsing to registry I/O. Placing is
                // constructing or writing a path, never defaulting, testing,
                // or redacting it. A `*_hint` field is operator-facing help
                // text (`update_hint: "... --target /usr/local/bin/cynic"`),
                // quoted so the operator can paste it, never executed as a
                // placement by this component.
                let inspects = line.contains("starts_with(")
                    || line.contains("ends_with(")
                    || line.contains("trim_start_matches(")
                    || line.contains("OWNED_PREFIXES")
                    || line.contains("owned_literal(")
                    || line.contains(".replace(")
                    || line.contains("default_value")
                    || line.contains("hint")
                    // An error message guides the operator, it places
                    // nothing (`Err("registry path required: ... install
                    // /etc/cynic/registry.json ...")`). Describing where a
                    // file should be is the opposite of placing it silently.
                    || line.contains("Err(")
                    // A string opening on `#` is an embedded shell comment
                    // (`"# or from repo after rebuild: ..." --target ...`),
                    // operator-to-operator text, never a placement.
                    || line.contains("\"#")
                    // A named `DEFAULT_*`/`CANONICAL_*` constant is a declared
                    // convention (`DEFAULT_REGISTRY_PATH =
                    // "/etc/cynic/registry.json"`), reviewed by name at the
                    // definition site. A hidden placement never announces
                    // itself in UPPER_SNAKE.
                    || (line.contains("const ")
                        && (line.contains("DEFAULT") || line.contains("CANONICAL")));

                if inspects {
                    continue;
                }

                // A literal inside a comment documents the world, it places
                // nothing: `//! Volume: mount /var/lib/cynic` describes the
                // mount model, it is not a competing placement claim. Real
                // placements are code; code that only exists in a comment
                // places nothing by definition.
                let trimmed = line.trim_start();
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }

                // A line naming TWO owned prefixes is a table, not a placement:
                // redaction lists (`["/home/user", "/var/lib/cynic", ...]`)
                // and detection tables enumerate prefixes to treat them all
                // the same. A real placement names one destination; nothing
                // constructs a path out of two different roots on one line.
                let prefix_hits: usize = OWNED_PREFIXES
                    .iter()
                    .map(|prefix| line.matches(prefix).count())
                    .sum();
                if prefix_hits >= 2 {
                    continue;
                }

                let literal = match owned_literal(line) {
                    Some(literal) => literal,
                    None => continue,
                };

                // A literal the registry itself declares is the registry's
                // shape appearing in code, not a competing claim. Anything else
                // is a claim about a tree the registry does not describe.
                let registry_backed = registry_roots
                    .iter()
                    .any(|root| literal == *root || literal.starts_with(&format!("{root}/")));

                let in_placement_field = PLACEMENT_FIELDS.iter().any(|field| {
                    line.contains(&format!("{field}:")) || line.contains(&format!("{field}: "))
                });

                // A test that INJECTS a layout is testing the injection, not
                // claiming a fleet placement. `HostRuntimeLayout { helper_dir:
                // "/opt/cynic/libexec" }` asserts that the resolver uses what it
                // is given; flagging it would forbid the test from existing and
                // would hide the divergence it is actually guarding.
                let injects_layout = (line.contains("helper_dir:")
                    || line.contains("config_dir:")
                    || line.contains("sudoers_dir:")
                    || line.contains("service_unit_dir:")
                    || line.contains("install_root:")
                    || line.contains("state_root:")
                    || line.contains("runtime_home_dir:"))
                    && (line.contains(": \"") || line.contains(":\""));

                // A fixture builds a synthetic world and asserts against it. A
                // canary asserts the value PRODUCTION computed. Both name a
                // path literal; only the second one can pin a divergence,
                // because the first would fail on any value the test itself
                // supplied. The distinguishing shape is the assertion, not the
                // path: `assert_eq!(plan.current_link, ...)` reads a value the
                // resolver produced, while `.to_string()` inside a struct being
                // constructed is input.
                let asserts_computed_value = line.contains("assert_eq!")
                    || line.contains("assert!")
                    || line.contains("assert_ne!")
                    || line.contains("expect(");

                // A file is a test when it says so in three ways: an in-file
                // `#[cfg(test)]` scope, a `/tests/` path segment, or the
                // `_tests.rs` module convention. The third case is why
                // `is_test_module_file` joins the other two.
                let is_test = in_test_scope || in_test_file || is_test_module_file;

                let (code, severity, remediation) = if is_test {
                    if registry_backed || (injects_layout && !asserts_computed_value) {
                        // A test naming a registry-declared path, or injecting a
                        // synthetic layout to prove the resolver honours it
                        // without asserting a value production computed, is
                        // doing its job.
                        continue;
                    }
                    let detail = if injects_layout {
                        "This test injects a layout and then asserts a value against it. If \
                         the resolver computed a different placement the test would still \
                         pass, so it cannot detect the divergence it appears to guard. Assert \
                         against the canonical registry instead."
                    } else {
                        "This test asserts a runtime path the canonical registry does not \
                         declare, so it pins the divergence in place: correcting the code \
                         breaks the test, and correcting the test removes the only thing \
                         that would have noticed. Repoint it at the registry."
                    };
                    (
                        "PATH-004-TEST-PINS-DIVERGENCE",
                        AuditSeverity::Warning,
                        detail,
                    )
                } else if in_placement_field || !registry_backed {
                    (
                        "PATH-004-UNREGISTERED-PLACEMENT",
                        AuditSeverity::Warning,
                        "Derive this path from infra/directory-tree.json. A placement field \
                         holding a literal is a coordination claim: two components with \
                         different values here are each locally correct and globally \
                         incompatible.",
                    )
                } else {
                    (
                        "PATH-004-LITERAL-TRACKED",
                        AuditSeverity::Warning,
                        "Tracked debt. Resolve against the registry when touching this \
                         file; the count must not increase.",
                    )
                };

                violations.push(AuditViolation {
                    code: code.to_string(),
                    message: format!(
                        "{}:{} embeds runtime path {} which the canonical registry \
                         ({PATH_SSOT}) does not declare",
                        file.relative_path,
                        idx + 1,
                        literal
                    ),
                    severity,
                    category: AuditCategory::PathSanitation,
                    span: Some(AuditSpan {
                        file_path: file.relative_path.clone(),
                        line_start: idx + 1,
                        line_end: idx + 1,
                        snippet: Some(line.trim().to_string()),
                    }),
                    remediation: Some(remediation.to_string()),
                });
            }
        }

        violations
    }
}

/// The placement roots the registry declares, derived from the registry itself
/// rather than restated. `layers.runtime_state.tree.root` plus every
/// `installed_system` root, expanded to the placement prefixes they own.
///
/// Returns an empty set when the registry is absent — an absent SSOT is
/// `PATH-001`'s finding to report, and inventing a root list here would let a
/// missing registry silently pass everything.
fn declared_placement_roots(index: &WorkspaceSourceIndex) -> BTreeSet<String> {
    let Some(registry) = index
        .json_files
        .iter()
        .find(|file| file.relative_path == PATH_SSOT)
    else {
        return BTreeSet::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&registry.content) else {
        return BTreeSet::new();
    };
    let mut roots = BTreeSet::new();
    let Some(layers) = value.get("layers").and_then(|layers| layers.as_object()) else {
        return roots;
    };
    for layer in layers.values() {
        if let Some(root) = layer
            .get("tree")
            .and_then(|tree| tree.get("root"))
            .and_then(|root| root.as_str())
        {
            roots.insert(root.trim_end_matches('/').to_string());
        }
    }
    roots
}

/// The first owned-prefix literal embedded as a value in `line`, if any.
///
/// A prefix inside a longer string still counts: `"$RUNTIME_INSTALL_ROOT/current"`
/// names the owned root by variable, and `"root:/var/lib/cynic"` still asserts
/// the absolute placement.
fn owned_literal(line: &str) -> Option<&str> {
    OWNED_PREFIXES
        .iter()
        .find(|prefix| line.contains(**prefix))
        .copied()
}
