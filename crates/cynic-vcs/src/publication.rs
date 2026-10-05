//! Publication rendering — pure law for projecting the private tree toward the
//! public repository.
//!
//! Authority: `contracts/publication.json` (`cynic.publication.v1`, normative).
//! This module owns the *decision*: given the observed tracked path set and the
//! contract's path rules and redactions, which files are published and which
//! literals must be substituted. It performs no I/O and owns no transport — the
//! host adapter supplies the observed tracked set and file bytes, and applies
//! the rendered tree.
//!
//! Two defects of the contract's own test this module makes refutable:
//! 1. the reference `glob_matches` recurses forever on any literal match
//!    (`"".split('/')` yields `[""]`, never empty), so the executable contract
//!    could never pass; the matcher here is total;
//! 2. the declared redaction covers DNS identities but not IPv4 literals, which
//!    live in published paths. A plan therefore reports uncovered literals
//!    instead of asserting a cleanliness the contract does not encode.
//!
//! Schema: `cynic.source_publication.v1`

use serde::{Deserialize, Serialize};

pub const SOURCE_PUBLICATION_SCHEMA: &str = "cynic.source_publication.v1";
pub const SOURCE_PUBLICATION_PLAN_SCHEMA: &str = "cynic.source_publication_plan.v1";

/// Minimum length of an exclusion reason. Mirrors the contract's own test: an
/// exclusion without a stated reason is indistinguishable from an oversight.
pub const MIN_REASON_LEN: usize = 20;

/// Path disposition declared by the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationPathStatus {
    /// Published after redactions are applied.
    Public,
    /// Absent from the public repository. Requires a reason.
    Exclude,
    /// Published for now, after redactions. Requires a removal condition.
    Temporary,
    /// Published with substitutions applied. Requires `example.invalid` targets.
    Redact,
}

impl PublicationPathStatus {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "public" => Some(Self::Public),
            "exclude" => Some(Self::Exclude),
            "temporary" => Some(Self::Temporary),
            "redact" => Some(Self::Redact),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Exclude => "exclude",
            Self::Temporary => "temporary",
            Self::Redact => "redact",
        }
    }

    /// Whether a path with this status reaches the public tree.
    pub fn is_published(self) -> bool {
        matches!(self, Self::Public | Self::Temporary | Self::Redact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationPathRule {
    pub pattern: String,
    pub status: PublicationPathStatus,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationSubstitution {
    pub match_pattern: String,
    pub replace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationRedaction {
    pub name: String,
    pub kind: String,
    pub applies_to: String,
    pub must_preserve_compilation: bool,
    pub substitutions: Vec<PublicationSubstitution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationContract {
    pub schema: String,
    pub status: String,
    pub path_rules: Vec<PublicationPathRule>,
    pub redactions: Vec<PublicationRedaction>,
}

impl PublicationContract {
    /// Parse the contract from its JSON text.
    ///
    /// Lenient by design: unknown keys are ignored (the contract carries prose
    /// such as `design_note` and `measured_occurrences` that is not law here),
    /// but a missing or mistyped field this module depends on is an error, never
    /// a silent default — a publication decision built on an absent rule would
    /// copy by omission.
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| format!("publication contract is not JSON: {e}"))?;

        let schema = value
            .get("schema")
            .and_then(|v| v.as_str())
            .ok_or("publication contract has no `schema`")?
            .to_string();
        let status = value
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or("publication contract has no `status`")?
            .to_string();

        let raw_rules = value
            .get("path_rules")
            .and_then(|v| v.as_array())
            .ok_or("publication contract has no `path_rules` array")?;

        let mut path_rules = Vec::with_capacity(raw_rules.len());
        for (idx, rule) in raw_rules.iter().enumerate() {
            let pattern = rule
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("path_rules[{idx}] has no `path`"))?
                .to_string();
            let status_raw = rule
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("path_rules[{idx}] (`{pattern}`) has no `status`"))?;
            let status = PublicationPathStatus::parse(status_raw).ok_or_else(|| {
                format!("path_rules[{idx}] (`{pattern}`) uses unknown status `{status_raw}`")
            })?;
            path_rules.push(PublicationPathRule {
                pattern,
                status,
                reason: rule
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                until: rule
                    .get("until")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }

        let mut redactions = Vec::new();
        if let Some(obj) = value.get("redactions").and_then(|v| v.as_object()) {
            for (name, spec) in obj {
                let substitutions = spec
                    .get("substitutions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| {
                                Some(PublicationSubstitution {
                                    match_pattern: s.get("match")?.as_str()?.to_string(),
                                    replace: s.get("replace")?.as_str()?.to_string(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                redactions.push(PublicationRedaction {
                    name: name.clone(),
                    kind: spec
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("literal_substitution")
                        .to_string(),
                    applies_to: spec
                        .get("applies_to")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    must_preserve_compilation: spec
                        .get("must_preserve_compilation")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    substitutions,
                });
            }
        }

        Ok(Self {
            schema,
            status,
            path_rules,
            redactions,
        })
    }

    /// The first rule whose pattern matches `path`, in declaration order.
    ///
    /// Declaration order is significant: the contract layers a broad `**` rule
    /// under narrower ones, and shadowing is reported separately rather than
    /// resolved silently.
    pub fn rule_for(&self, path: &str) -> Option<&PublicationPathRule> {
        self.path_rules
            .iter()
            .find(|r| glob_matches(&r.pattern, path))
    }
}

/// Match a contract glob against a repository-relative path.
///
/// Supports `**` (any number of segments, including zero), `*` (exactly one
/// segment) and literal segments. Hand-rolled rather than pulling in a
/// dependency: a new crate dependency needs its own admission, and the required
/// semantics are three cases.
pub fn glob_matches(pattern: &str, path: &str) -> bool {
    fn segments(s: &str) -> Vec<&str> {
        if s.is_empty() {
            Vec::new()
        } else {
            s.split('/').collect()
        }
    }
    glob_segments(&segments(pattern), &segments(path))
}

fn glob_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| glob_segments(rest, &path[skip..])),
        Some((&head, rest)) => {
            if path.is_empty() || (head != "*" && head != path[0]) {
                return false;
            }
            glob_segments(rest, &path[1..])
        }
    }
}

/// Per-path publication decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationPathDecision {
    pub path: String,
    pub status: PublicationPathStatus,
    /// The rule pattern that decided it, or `null` when undeclared.
    pub rule_pattern: Option<String>,
}

/// A literal observed in a published path that no declared redaction covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncoveredLiteral {
    pub literal: String,
    pub paths: Vec<String>,
}

/// The non-mutating publication plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationPlan {
    pub schema: String,
    pub contract_schema: String,
    pub contract_status: String,
    pub published: Vec<PublicationPathDecision>,
    pub excluded: Vec<PublicationPathDecision>,
    /// Tracked paths with no rule. Copied by omission would be a decision
    /// nobody made, so these block publication outright.
    pub undeclared: Vec<String>,
    /// Rules that match no tracked path: dead or misnamed, both drift.
    pub dead_rules: Vec<String>,
    /// Literals present in published paths but covered by no redaction.
    pub uncovered_literals: Vec<UncoveredLiteral>,
    pub blockers: Vec<String>,
    pub published_count: usize,
    pub excluded_count: usize,
    pub tracked_count: usize,
}

impl PublicationPlan {
    pub fn is_admitted(&self) -> bool {
        self.blockers.is_empty()
    }
}

/// Pure publication decision.
///
/// `files` carries `(path, content)` for every tracked path whose status is
/// published, so the plan can report literals the contract's redactions do not
/// cover. Content is read by the caller (adapter); this function is pure.
pub fn plan_publication(
    contract: &PublicationContract,
    tracked: &[String],
    files: &[(String, String)],
) -> PublicationPlan {
    let mut published = Vec::new();
    let mut excluded = Vec::new();
    let mut undeclared = Vec::new();
    let mut blockers = Vec::new();

    for path in tracked {
        match contract.rule_for(path) {
            Some(rule) => {
                let decision = PublicationPathDecision {
                    path: path.clone(),
                    status: rule.status,
                    rule_pattern: Some(rule.pattern.clone()),
                };
                if rule.status.is_published() {
                    published.push(decision);
                } else {
                    excluded.push(decision);
                }
            }
            None => {
                undeclared.push(path.clone());
                blockers.push(format!("undeclared_path:{path}"));
            }
        }
    }

    let mut dead_rules = Vec::new();
    for rule in &contract.path_rules {
        if rule.status.is_published() && !tracked.iter().any(|p| glob_matches(&rule.pattern, p)) {
            dead_rules.push(rule.pattern.clone());
            blockers.push(format!("dead_rule:{}", rule.pattern));
        }
    }

    for rule in &contract.path_rules {
        match rule.status {
            PublicationPathStatus::Exclude => {
                let reason_len = rule.reason.as_deref().unwrap_or("").len();
                if reason_len < MIN_REASON_LEN {
                    blockers.push(format!("exclude_without_reason:{}", rule.pattern));
                }
            }
            PublicationPathStatus::Temporary => {
                let until_len = rule.until.as_deref().unwrap_or("").len();
                if until_len < MIN_REASON_LEN {
                    blockers.push(format!("temporary_without_until:{}", rule.pattern));
                }
            }
            _ => {}
        }
    }

    // Two literal rules on one path are an unresolvable ambiguity.
    let mut seen_literals: Vec<&str> = Vec::new();
    for rule in &contract.path_rules {
        if rule.pattern.contains(['*', '?']) {
            continue;
        }
        if seen_literals.contains(&rule.pattern.as_str()) {
            blockers.push(format!("duplicate_literal_rule:{}", rule.pattern));
        } else {
            seen_literals.push(&rule.pattern);
        }
    }

    for redaction in &contract.redactions {
        if !redaction.must_preserve_compilation {
            blockers.push(format!(
                "redaction_missing_compilation_guard:{}",
                redaction.name
            ));
        }
        if redaction.substitutions.is_empty() {
            blockers.push(format!("redaction_without_substitution:{}", redaction.name));
        }
        for sub in &redaction.substitutions {
            if !is_reserved_target(&sub.replace) {
                blockers.push(format!(
                    "redaction_target_not_reserved:{}:{}",
                    redaction.name, sub.replace
                ));
            }
        }
    }

    let uncovered_literals = uncovered_literals(contract, files);
    for uncovered in &uncovered_literals {
        blockers.push(format!(
            "uncovered_literal:{}:{}",
            uncovered.literal,
            uncovered.paths.len()
        ));
    }

    let published_count = published.len();
    let excluded_count = excluded.len();
    PublicationPlan {
        schema: SOURCE_PUBLICATION_PLAN_SCHEMA.to_string(),
        contract_schema: contract.schema.clone(),
        contract_status: contract.status.clone(),
        published,
        excluded,
        undeclared,
        dead_rules,
        uncovered_literals,
        blockers,
        published_count,
        excluded_count,
        tracked_count: tracked.len(),
    }
}

/// Literals present in published paths that no declared redaction covers.
///
/// The contract's `infra_identities` redaction targets DNS names; IPv4 literals
/// of the private network live in published paths and are not covered. This
/// reports them as data instead of asserting a cleanliness the contract does not
/// encode.
pub fn uncovered_literals(
    contract: &PublicationContract,
    files: &[(String, String)],
) -> Vec<UncoveredLiteral> {
    let redacts_ipv4 = contract.redactions.iter().any(|r| {
        r.substitutions
            .iter()
            .any(|s| substitution_targets_ipv4(&s.match_pattern))
    });
    if redacts_ipv4 {
        return Vec::new();
    }

    let mut by_literal: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (path, content) in files {
        for literal in private_ipv4_literals(content) {
            by_literal.entry(literal).or_default().push(path.clone());
        }
    }
    by_literal
        .into_iter()
        .map(|(literal, mut paths)| {
            paths.sort();
            paths.dedup();
            UncoveredLiteral { literal, paths }
        })
        .collect()
}

/// Whether a redaction replacement targets a reserved, non-resolving identity.
///
/// `example.invalid` is reserved by RFC 2606 and never resolves, so a test aimed
/// at real infrastructure fails loudly instead of silently. The IPv4 redaction
/// instead targets an RFC 5737 documentation range (`192.0.2.0/24`,
/// `198.51.100.0/24`, `203.0.113.0/24`), also non-routable by reservation. A
/// replacement that could resolve would turn a redacted identity into a live one.
fn is_reserved_target(replace: &str) -> bool {
    if replace.contains("example.invalid") {
        return true;
    }
    let first = replace
        .split('.')
        .next()
        .and_then(|s| s.parse::<u16>().ok());
    let second = replace
        .split('.')
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok());
    matches!(
        (first, second),
        (Some(192), Some(0)) | (Some(198), Some(51)) | (Some(203), Some(0))
    )
}

/// Whether a redaction `match` pattern is anchored on an IPv4 literal.
///
/// Heuristic and deliberately narrow: an IPv4 pattern needs three escaped dots
/// (`\.`) separating four octet groups. A DNS pattern such as
/// `[a-z0-9-]+\.synapse\.cynic(:[0-9]+)?` contains a digit class (the port) and a
/// single dot, so a digit-class-plus-dot test would misclassify it as IPv4 and
/// wrongly suppress the uncovered-literal report. Counting the dots avoids that.
/// It answers "did anyone declare an IPv4 redaction", not "is the redaction
/// correct" — the rendered read-back owns that.
fn substitution_targets_ipv4(pattern: &str) -> bool {
    pattern.matches("\\.").count() >= 3
}

/// Distinct private IPv4 literals in `text`, sorted.
///
/// Private ranges: `10/8`, `172.16/12`, `192.168/16`. Loopback and link-local
/// are excluded — they carry no topology.
pub fn private_ipv4_literals(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // An octet must not be preceded by a digit or a dot-digit (avoid
        // matching a suffix of a longer number).
        if i > 0 && (bytes[i - 1].is_ascii_digit() || bytes[i - 1] == b'.') {
            i += 1;
            continue;
        }
        if let Some((literal, end)) = scan_ipv4(bytes, i) {
            if is_private(&literal) {
                found.insert(literal);
            }
            i = end;
        } else {
            i += 1;
        }
    }
    found.into_iter().collect()
}

fn scan_ipv4(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut octets = Vec::with_capacity(4);
    for octet_index in 0..4 {
        if octet_index > 0 {
            if i >= bytes.len() || bytes[i] != b'.' {
                return None;
            }
            i += 1;
        }
        let octet_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let len = i - octet_start;
        if len == 0 || len > 3 {
            return None;
        }
        let value: u16 = std::str::from_utf8(&bytes[octet_start..i])
            .ok()?
            .parse()
            .ok()?;
        if value > 255 {
            return None;
        }
        octets.push(value);
    }
    // Reject a trailing digit (e.g. `10.0.0.1234` is not `10.0.0.123`).
    if i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        return None;
    }
    let literal = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
    Some((literal, i))
}

fn is_private(literal: &str) -> bool {
    let mut parts = literal.split('.');
    let (a, b) = match (
        parts.next().and_then(|s| s.parse::<u16>().ok()),
        parts.next().and_then(|s| s.parse::<u16>().ok()),
    ) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract_with(
        rules: Vec<PublicationPathRule>,
        redactions: Vec<PublicationRedaction>,
    ) -> PublicationContract {
        PublicationContract {
            schema: SOURCE_PUBLICATION_SCHEMA.to_string(),
            status: "normative".to_string(),
            path_rules: rules,
            redactions,
        }
    }

    fn rule(pattern: &str, status: PublicationPathStatus) -> PublicationPathRule {
        PublicationPathRule {
            pattern: pattern.to_string(),
            status,
            reason: Some("a substantive reason long enough to pass".to_string()),
            until: Some("a removal condition long enough to pass".to_string()),
        }
    }

    #[test]
    fn literal_glob_terminates_and_matches() {
        // The contract's reference matcher recurses forever here.
        assert!(glob_matches("Cargo.toml", "Cargo.toml"));
        assert!(!glob_matches("Cargo.toml", "Cargo.lock"));
        assert!(glob_matches("README.md", "README.md"));
    }

    #[test]
    fn double_star_matches_zero_or_more_segments() {
        assert!(glob_matches("crates/**", "crates/cynic-vcs/src/lib.rs"));
        assert!(glob_matches("crates/**", "crates/x"));
        assert!(!glob_matches("crates/**", "docs/x"));
        assert!(glob_matches("**", "anything/at/all.rs"));
    }

    #[test]
    fn single_star_matches_exactly_one_segment() {
        assert!(glob_matches("crates/*", "crates/cynic-vcs"));
        assert!(!glob_matches("crates/*", "crates/cynic-vcs/src"));
    }

    #[test]
    fn contract_parses_and_resolves_rules_in_order() {
        let json = r#"{
            "schema": "cynic.publication.v1",
            "status": "normative",
            "path_rules": [
                { "path": "crates/**", "status": "public" },
                { "path": "docs/**", "status": "exclude", "reason": "session notes and third-party coursework unrelated to the runtime" }
            ],
            "redactions": {
                "infra_identities": {
                    "kind": "literal_substitution",
                    "applies_to": "every published path, without exception",
                    "must_preserve_compilation": true,
                    "substitutions": [
                        { "match": "[a-z0-9-]+\\.synapse\\.cynic(:[0-9]+)?", "replace": "<node>.example.invalid" }
                    ]
                }
            }
        }"#;
        let contract = PublicationContract::from_json(json).expect("parses");
        assert_eq!(contract.path_rules.len(), 2);
        assert_eq!(contract.redactions.len(), 1);
        assert_eq!(
            contract
                .rule_for("crates/cynic-vcs/src/lib.rs")
                .unwrap()
                .status,
            PublicationPathStatus::Public
        );
        assert_eq!(
            contract.rule_for("docs/foo.md").unwrap().status,
            PublicationPathStatus::Exclude
        );
        assert!(contract.rule_for("deny.toml").is_none());
    }

    #[test]
    fn undeclared_path_blocks_publication() {
        let contract = contract_with(
            vec![rule("crates/**", PublicationPathStatus::Public)],
            vec![],
        );
        let tracked = vec!["crates/a.rs".to_string(), "deny.toml".to_string()];
        let plan = plan_publication(&contract, &tracked, &[]);
        assert!(!plan.is_admitted());
        assert_eq!(plan.undeclared, vec!["deny.toml".to_string()]);
        assert!(
            plan.blockers
                .iter()
                .any(|b| b == "undeclared_path:deny.toml")
        );
    }

    #[test]
    fn dead_rule_blocks_publication() {
        let contract = contract_with(
            vec![
                rule("crates/**", PublicationPathStatus::Public),
                rule("ghost/**", PublicationPathStatus::Public),
            ],
            vec![],
        );
        let plan = plan_publication(&contract, &["crates/a.rs".to_string()], &[]);
        assert!(plan.blockers.iter().any(|b| b == "dead_rule:ghost/**"));
    }

    #[test]
    fn exclusion_without_reason_blocks() {
        let mut r = rule("docs/**", PublicationPathStatus::Exclude);
        r.reason = Some("short".to_string());
        let contract = contract_with(vec![r], vec![]);
        let plan = plan_publication(&contract, &[], &[]);
        assert!(
            plan.blockers
                .iter()
                .any(|b| b == "exclude_without_reason:docs/**")
        );
    }

    #[test]
    fn redaction_target_must_be_reserved_domain() {
        let bad = PublicationRedaction {
            name: "x".to_string(),
            kind: "literal_substitution".to_string(),
            applies_to: "every published path".to_string(),
            must_preserve_compilation: true,
            substitutions: vec![PublicationSubstitution {
                match_pattern: "a".to_string(),
                replace: "prod.internal".to_string(),
            }],
        };
        let contract = contract_with(vec![], vec![bad]);
        let plan = plan_publication(&contract, &[], &[]);
        assert!(
            plan.blockers
                .iter()
                .any(|b| b.starts_with("redaction_target_not_reserved:"))
        );
    }

    #[test]
    fn private_ipv4_literals_are_detected_and_scoped() {
        let text =
            "host 10.10.10.119 and 192.168.1.5 and 172.16.4.4 plus public 8.8.8.8 and 10.0.0.1234";
        let found = private_ipv4_literals(text);
        assert!(found.contains(&"10.10.10.119".to_string()));
        assert!(found.contains(&"192.168.1.5".to_string()));
        assert!(found.contains(&"172.16.4.4".to_string()));
        assert!(!found.contains(&"8.8.8.8".to_string()));
        // 10.0.0.1234 must not yield 10.0.0.123
        assert!(!found.iter().any(|l| l == "10.0.0.123"));
    }

    #[test]
    fn uncovered_ipv4_literals_are_reported_when_no_redaction_covers_them() {
        let contract = contract_with(
            vec![rule("crates/**", PublicationPathStatus::Public)],
            vec![PublicationRedaction {
                name: "infra_identities".to_string(),
                kind: "literal_substitution".to_string(),
                applies_to: "every published path".to_string(),
                must_preserve_compilation: true,
                substitutions: vec![PublicationSubstitution {
                    match_pattern: "[a-z0-9-]+\\.synapse\\.cynic(:[0-9]+)?".to_string(),
                    replace: "<node>.example.invalid".to_string(),
                }],
            }],
        );
        let files = vec![(
            "crates/a.rs".to_string(),
            "builder 10.10.10.119".to_string(),
        )];
        let plan = plan_publication(&contract, &["crates/a.rs".to_string()], &files);
        assert_eq!(plan.uncovered_literals.len(), 1);
        assert_eq!(plan.uncovered_literals[0].literal, "10.10.10.119");
        assert!(
            plan.blockers
                .iter()
                .any(|b| b.starts_with("uncovered_literal:10.10.10.119"))
        );
    }

    #[test]
    fn ipv4_redaction_declaration_clears_uncovered_report() {
        let contract = contract_with(
            vec![rule("crates/**", PublicationPathStatus::Public)],
            vec![PublicationRedaction {
                name: "infra_ipv4".to_string(),
                kind: "literal_substitution".to_string(),
                applies_to: "every published path".to_string(),
                must_preserve_compilation: true,
                substitutions: vec![PublicationSubstitution {
                    match_pattern:
                        "(10|172\\.(1[6-9]|2[0-9]|3[01])|192\\.168)\\.[0-9]{1,3}\\.[0-9]{1,3}"
                            .to_string(),
                    replace: "192.0.2.1".to_string(),
                }],
            }],
        );
        let files = vec![("crates/a.rs".to_string(), "10.10.10.119".to_string())];
        let plan = plan_publication(&contract, &["crates/a.rs".to_string()], &files);
        assert!(plan.uncovered_literals.is_empty());
    }
}
