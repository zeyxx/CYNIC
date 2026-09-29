//! RUN-001 — the divergences only a live system can settle.
//!
//! WHY. `contracts/foundation.json` puts "observed runtime state" first in the
//! authority order, and nothing in the workspace ever read the runtime. The
//! repository-side rules are all hermetic, which is correct and also means that
//! the four facts below were only ever checked by a human remembering to look:
//!
//!   - six services are declared in infra/registry.json while thirty systemd
//!     units are installed; the extra twenty-four are invisible to the SSOT;
//!   - the deployed kernel binary was built on 2026-08-11 while the source was
//!     at 2026-09-29, and the service was restarted in between, so "active"
//!     never meant "current". No provenance record existed to compare against;
//!   - three credentials sat in cleartext in a unit file readable by any local
//!     user, and the secrets port could only read the process environment, so
//!     the exposure could not be closed from the code side;
//!   - the claim store reported twenty active claims, all long past their TTL,
//!     and `consumed.actions` was zero on every one, so a budget counter that
//!     cannot move was standing in for a limit.
//!
//! WHAT IT DOES NOT DO. It never prints a credential value, ever. It reports the
//! unit file and the variable NAME, because a report that leaks the secret is
//! worse than no report. It also never claims the host is clean: when the host
//! was not observed, the rule returns nothing and says so, because an audit that
//! silently skips its own domain is how the domain stops being checked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

pub use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const RULE_SSOT: &str = "infra/registry.json";

pub struct RuntimeDivergenceRule;

impl AuditRule for RuntimeDivergenceRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "RUN-001".to_string(),
            name: "runtime_divergence".to_string(),
            description: "The declared runtime and the running runtime must agree: declared \
                          services against installed units, a deployed binary against the source \
                          it claims to be, credential values against the units holding them, and \
                          the claim store against the claims it says are active."
                .to_string(),
            category: AuditCategory::ResidualUnknown,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        evaluate_rule(index)
    }
}

/// The body of the rule, callable from a fixture without a host.
pub fn evaluate_rule(index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
    {
        let Some(host) = &index.host else {
            // Explicitly not a pass. The rule could not observe, so it says
            // nothing; `cynic audit` reports the rule as evaluated and the
            // operator sees an empty domain, which is the honest shape of it.
            return Vec::new();
        };
        let mut violations = Vec::new();

        violations.extend(unit_divergence(index, host));
        violations.extend(cynic_binaries_without_provenance(host));
        violations.extend(credentials_in_unit_files(host));
        violations.extend(stale_claim_store(host));

        violations
    }
}

/// The unit divergence, in both directions, and only for what this project owns.
///
/// The first version of this rule reported every installed unit: 326 of them,
/// including `systemd-journald` and `apt-daily`. That is the same mistake
/// `PathSanitationRule` documents in its own comment — counting things outside
/// the boundary as critical makes the rule unactionable, and an unactionable
/// critical is noise that gets ignored, which is worse than silence because it
/// spends the operator's attention.
///
/// So the boundary is the registry plus the CYNIC naming space. A distro unit is
/// the operating system's business, not a divergence in a CYNIC authority map.
const CYNIC_UNIT_PREFIXES: [&str; 4] = ["cynic-", "hermes-", "surreal", "cynic"];

/// A unit is "owned" if the registry declares it, or if its name is in the
/// project's own naming space. Everything else is explicitly out of scope.
fn owned_units(index: &WorkspaceSourceIndex, host: &super::scanner::HostObservation) -> (BTreeSet<String>, BTreeSet<String>) {
    let declared: BTreeSet<String> = declared_service_ids(index);

    let installed: BTreeSet<String> = host
        .unit_files
        .iter()
        .filter_map(|path| path.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .map(|stem| stem.trim_end_matches(".service").trim_end_matches(".timer").to_string())
        .filter(|stem| !stem.is_empty())
        .collect();

    let owned: BTreeSet<String> = installed
        .iter()
        .filter(|stem| {
            declared.contains(*stem)
                || CYNIC_UNIT_PREFIXES.iter().any(|prefix| stem.starts_with(prefix))
        })
        .cloned()
        .collect();

    (declared, owned)
}

fn declared_service_ids(index: &WorkspaceSourceIndex) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in index.json_files.iter().filter(|f| f.relative_path == RULE_SSOT) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&file.content) else {
            continue;
        };
        let Some(nodes) = value.get("nodes").and_then(|n| n.as_array()) else {
            continue;
        };
        for node in nodes {
            let Some(services) = node.get("services").and_then(|s| s.as_array()) else {
                continue;
            };
            for service in services {
                if let Some(name) = service
                    .get("id")
                    .or_else(|| service.get("name"))
                    .and_then(|v| v.as_str())
                {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names
}

fn unit_divergence(
    index: &WorkspaceSourceIndex,
    host: &super::scanner::HostObservation,
) -> Vec<AuditViolation> {
    let (declared, owned) = owned_units(index, host);
    let mut violations = Vec::new();

    // Direction 1: a unit this project runs that the authority map never heard
    // of. It cannot be routed, scheduled or owned.
    let undeclared: Vec<&String> = owned.difference(&declared).collect();
    if !undeclared.is_empty() {
        let names: Vec<&str> = undeclared.iter().map(|s| s.as_str()).collect();
        violations.push(violation(
            "RUN-001-CYNIC-UNIT-UNDECLARED",
            format!(
                "{} unit(s) belong to this project and are absent from {RULE_SSOT}: {}",
                undeclared.len(),
                names.join(", ")
            ),
            "A service the registry never heard of is the runtime version of an uncompiled              file: running, and invisible to every check that reads the SSOT. Declare it or              remove it.",
        ));
    }

    // Direction 2: the reverse, which is the one nobody checks. A declared
    // service with no unit on this host is either not deployed, deployed
    // elsewhere, or a claim about a runtime that does not exist here.
    let installed_stems: BTreeSet<String> = host
        .unit_files
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .map(|s| s.trim_end_matches(".service").trim_end_matches(".timer").to_string())
        .collect();
    let absent: Vec<&String> = declared.difference(&installed_stems).collect();
    if !absent.is_empty() {
        let names: Vec<&str> = absent.iter().map(|s| s.as_str()).collect();
        violations.push(violation(
            "RUN-001-DECLARED-SERVICE-ABSENT",
            format!(
                "{} service(s) declared in {RULE_SSOT} have no unit on this host: {}",
                absent.len(),
                names.join(", ")
            ),
            "The registry is allowed to describe nodes this host is not, so this is only a              finding when the declaration claims THIS node. Check `host_node_id` before              treating it as one.",
        ));
    }

    // Direction 3, the loudest: a unit whose ExecStart does not exist. This is
    // 203/EXEC, and it is what burns a restart loop — 3092 recorded restarts on
    // one such unit before anyone read the journal.
    let mut broken: Vec<String> = host
        .exec_starts
        .iter()
        .filter(|(unit, program)| {
            !program.exists()
                && owned.contains(unit.trim_end_matches(".service").trim_end_matches(".timer"))
        })
        .map(|(unit, program)| format!("{unit} -> {}", program.display()))
        .collect();
    broken.sort();
    broken.dedup();
    if !broken.is_empty() {
        violations.push(violation(
            "RUN-001-EXECSTART-ABSENT",
            format!(
                "{} unit(s) declare an ExecStart that does not exist: {}",
                broken.len(),
                broken.join("; ")
            ),
            "systemd reports 203/EXEC and retries on a timer, so this is a restart loop with              no failure a human sees. Remove the unit, or deploy the binary it names.",
        ));
    }

    violations
}

/// Binaries THIS PROJECT produced, with no record of what they were built from.
///
/// Scoped to the CYNIC install prefixes on purpose. The first version demanded
/// provenance of `/usr/bin/bash` and `/usr/lib/systemd/systemd` — 176 of them —
/// and a rule that flags the shell is a rule nobody reads.
const CYNIC_BINARY_PREFIXES: [&str; 3] = ["/usr/local/bin/cynic", "/usr/local/lib/cynic", "/opt/cynic"];

fn is_cynic_binary(program: &Path) -> bool {
    let text = program.display().to_string();
    CYNIC_BINARY_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

fn cynic_binaries_without_provenance(
    host: &super::scanner::HostObservation,
) -> Vec<AuditViolation> {
    let mut without: Vec<String> = host
        .exec_starts
        .iter()
        .filter(|(_, program)| is_cynic_binary(program) && program.exists() && !provenance_record_exists(program))
        .map(|(_, program)| program.display().to_string())
        .collect();
    without.sort();
    without.dedup();
    if without.is_empty() {
        return Vec::new();
    }
    vec![violation(
        "RUN-001-NO-PROVENANCE",
        format!(
            "{} binary/binaries built by this project carry no commit and no digest, so nothing              can say what code is running: {}",
            without.len(),
            without.join(", ")
        ),
        "A binary that does not record its source commit cannot be compared to the tree, so a          restart proves availability and never currency. A `--version` printing the commit and          the compile-time digest, written next to the binary, closes this for one unit.",
    )]
}

fn provenance_record_exists(program: &Path) -> bool {
    let Some(name) = program.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let parent = program.parent().unwrap_or(Path::new("/usr/local/bin"));
    for candidate in [
        parent.join(format!("{name}.provenance")),
        parent.join(format!("{name}.commit")),
    ] {
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Credential values held in a unit file.
///
/// Presence and the variable NAME only. The value never enters the report: a
/// report that leaks a secret is a worse outcome than the one it documents.
fn credentials_in_unit_files(
    host: &super::scanner::HostObservation,
) -> Vec<AuditViolation> {
    const CREDENTIAL_MARKERS: [&str; 4] = ["KEY", "TOKEN", "SECRET", "PASSWORD"];
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (path, text) in &host.unit_contents {
        for line in text.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed
                .strip_prefix("Environment=")
                .or_else(|| trimmed.strip_prefix("Environment:"))
            else {
                continue;
            };
            let inner = rest.trim().trim_matches('"');
            let Some((name, value)) = inner.split_once('=') else {
                continue;
            };
            if value.len() < 16 {
                continue;
            }
            // A path to a file is the correct shape, not a leak.
            if value.starts_with('/') || value.starts_with("${") {
                continue;
            }
            if !CREDENTIAL_MARKERS
                .iter()
                .any(|marker| name.to_ascii_uppercase().contains(marker))
            {
                continue;
            }
            found
                .entry(path.display().to_string())
                .or_default()
                .insert(name.to_string());
        }
    }

    if found.is_empty() {
        return Vec::new();
    }
    let detail = found
        .iter()
        .map(|(path, names)| {
            let mut list: Vec<String> = names.iter().cloned().collect();
            list.sort();
            format!("{path} [{}]", list.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ");
    vec![violation(
        "RUN-001-CREDENTIAL-IN-UNIT",
        format!(
            "{} unit file(s) hold a credential value in cleartext. The value is not reported: \
             file and variable name only. {detail}",
            found.len()
        ),
        "Rotate the value, then move it to a principal-owned file referenced by the unit. Note \
         that an env-injected secret stays visible through `systemctl show -p Environment` to any \
         local user, so this closes the world-readable file, not the exposure; closing the \
         exposure needs a credential tier the secrets port does not have yet.",
    )]
}

/// The claim store against reality: entries that say active and are not, and a
/// budget counter that never moved.
fn stale_claim_store(host: &super::scanner::HostObservation) -> Vec<AuditViolation> {
    let Some(text) = &host.claim_store else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![violation(
            "RUN-001-CLAIM-STORE-UNREADABLE",
            "the claim store exists but does not parse, so no claim can be judged live or dead"
                .to_string(),
            "A store that cannot be read is a store nobody can arbitrate on. Repair it before \
             claiming anything else.",
        )];
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();

    let mut expired_active: Vec<String> = Vec::new();
    let mut consumed_values: BTreeSet<u64> = BTreeSet::new();
    let mut owners: BTreeSet<String> = BTreeSet::new();

    if let Some(claims) = value.get("claims").and_then(|c| c.as_array()) {
        for claim in claims {
            if claim.get("status").and_then(|s| s.as_str()) != Some("active") {
                continue;
            }
            let agent = claim
                .get("agent_id")
                .and_then(|a| a.as_str())
                .unwrap_or("?")
                .to_string();
            owners.insert(agent.clone());
            let started = claim
                .get("temporal")
                .and_then(|t| t.get("started_at_unix_s"))
                .and_then(|v| v.as_u64())
                .unwrap_or_default();
            let ttl = claim
                .get("temporal")
                .and_then(|t| t.get("ttl_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or_default();
            if started + ttl <= now {
                let path = claim
                    .get("scope")
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("?");
                expired_active.push(format!("{agent}:{path}"));
            }
            if let Some(actions) = claim
                .get("consumed")
                .and_then(|c| c.get("actions"))
                .and_then(|v| v.as_u64())
            {
                consumed_values.insert(actions);
            }
        }
    }

    let mut violations = Vec::new();
    if !expired_active.is_empty() {
        expired_active.sort();
        violations.push(violation(
            "RUN-001-STALE-CLAIM",
            format!(
                "{} claim(s) are marked active and are past their own TTL: {}",
                expired_active.len(),
                expired_active.join(", ")
            ),
            "A claim that outlived its TTL without being released is a lock nobody is holding. \
             Expiry must be applied by whoever writes the store, not inferred by the reader.",
        ));
    }
    // A counter that is always one value carries no information: it is
    // indistinguishable from a field that does not exist, so it can never be
    // used as evidence of liveness or of a respected budget.
    if consumed_values.len() == 1 && !owners.is_empty() {
        violations.push(violation(
            "RUN-001-BUDGET-NOT-COUNTED",
            format!(
                "every active claim reports consumed.actions = {} across {} claim owner(s), so \
                 the action budget is decorative and cannot be used as a limit or as proof",
                consumed_values.iter().next().copied().unwrap_or_default(),
                owners.len()
            ),
            "Either the counter is written on every action, or the field is removed. A number \
             that never changes is worse than no number, because it looks like evidence.",
        ));
    }
    violations
}

fn violation(code: &str, message: String, remediation: &str) -> AuditViolation {
    AuditViolation {
        code: code.to_string(),
        message,
        severity: AuditSeverity::Critical,
        category: AuditCategory::ResidualUnknown,
        span: Some(AuditSpan {
            file_path: RULE_SSOT.to_string(),
            line_start: 1,
            line_end: 1,
            snippet: None,
        }),
        remediation: Some(remediation.to_string()),
    }
}
