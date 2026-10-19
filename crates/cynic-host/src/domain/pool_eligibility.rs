//! Pool eligibility — pure domain rules deciding whether a CYNIC node may
//! take build jobs.
//!
//! Two bricks, both mandatory (2026-10-19 incident: a runner advertised the
//! shared `cynic/build` label without the toolchain the workflow needs, so
//! the F-gate died in seconds on `Cannot find: node in PATH`):
//!
//! 1. **Declared pool labels** — the labels a node may serve live in the
//!    registry. A runner advertising a label the registry does not grant is
//!    an eligibility failure; a granted label the runner does not advertise
//!    is also a failure. Labels are never inferred from the runner.
//! 2. **Verified toolchain** — a node may only be offered a job whose
//!    required tools it has actually proven to run. Proof is an observation
//!    (the executable resolved in `PATH` at a version at least as new as the
//!    declared minimum). A declared-but-missing tool is a failure, never a
//!    hopeful assumption.
//!
//! This module is pure: it takes observations and returns a verdict. Probing
//! lives in `adapters/pool_toolchain_probe.rs`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Schema identifier of the eligibility report.
pub const POOL_ELIGIBILITY_SCHEMA: &str = "cynic.pool.eligibility.v1";

/// A tool a node must provide to serve a pool label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ToolRequirement {
    /// Tool name as resolved in `PATH` (e.g. `cargo`).
    pub name: String,
    /// Minimum version, when the registry declares one.
    #[serde(default)]
    pub min_version: Option<String>,
    /// False when the tool is optional (probed, but not a blocker).
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

/// What was actually observed about a tool on a host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ToolObservation {
    pub name: String,
    pub present: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

/// Per-tool verdict, combining requirement and observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ToolCheck {
    pub name: String,
    pub required: bool,
    pub present: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// Why the tool fails the requirement, when it does.
    pub detail: Option<String>,
}

/// The pool membership a node declares in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PoolDeclaration {
    /// Labels the node is allowed to serve (e.g. `cynic/build`).
    pub labels: Vec<String>,
    /// Tools the node must prove before serving those labels.
    #[serde(default)]
    pub required_tools: Vec<ToolRequirement>,
}

/// Base enrollment evidence for a node.
///
/// A node must be enrolled before it may claim a role or be attributed a
/// pool label: the enrollment record is the proof that the machine is known,
/// reachable and accountable. Role attribution without it is a claim, not a
/// fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EnrollmentEvidence {
    /// Enrollment status from the registry: `stub`, `candidate`, `observed`.
    pub status: String,
    /// True when the node carries an enrollment stamp (date + enrolled_by).
    pub enrolled: bool,
    /// Who performed the enrollment, when stamped.
    pub enrolled_by: Option<String>,
    /// Enrollment date (ISO-8601), when stamped.
    pub enrolled_at: Option<String>,
}

impl EnrollmentEvidence {
    /// Base enrollment is satisfied by a stamp and a status that is not `stub`.
    pub fn is_base_enrolled(&self) -> bool {
        self.enrolled && self.status != "stub"
    }
}

/// Verdict for one node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PoolEligibility {
    pub schema: String,
    pub node_id: String,
    /// Labels the registry grants to this node.
    pub declared_labels: Vec<String>,
    /// Labels the runner actually advertises, as observed.
    pub observed_labels: Vec<String>,
    /// Granted labels the runner does not advertise.
    pub missing_labels: Vec<String>,
    /// Advertised labels the registry does not grant.
    pub undeclared_labels: Vec<String>,
    /// Base enrollment evidence observed for the node.
    pub enrollment: EnrollmentEvidence,
    /// True when the node is base-enrolled (stamped and past `stub`).
    pub base_enrolled: bool,
    /// One entry per requirement, with its observation.
    pub tools: Vec<ToolCheck>,
    /// True when the node may serve its declared labels.
    pub eligible: bool,
    /// Human-readable blockers, empty when eligible.
    pub reasons: Vec<String>,
}

/// Extract the first version-looking token from a version banner.
///
/// Tool banners are prose: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`. The
/// version to compare is the first dotted numeric token in the line.
pub fn extract_version(banner: &str) -> Option<String> {
    for token in banner.split_whitespace() {
        let candidate = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if candidate.is_empty() {
            continue;
        }
        if candidate.contains('.') && candidate.split('.').all(|part| part.parse::<u64>().is_ok()) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Compare dotted version strings. `None` when either side is unparsable.
pub fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<Vec<u64>> {
        let core = s.trim().trim_start_matches('v');
        let core = core.split(['-', '+']).next()?;
        if core.is_empty() {
            return None;
        }
        core.split('.')
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<u64>>>()
    };
    let va = parse(a)?;
    let vb = parse(b)?;
    let len = va.len().max(vb.len());
    for index in 0..len {
        let left = va.get(index).copied().unwrap_or(0);
        let right = vb.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    Some(std::cmp::Ordering::Equal)
}

/// Evaluate whether a node may serve its declared pool labels.
///
/// `observed_labels` comes from the runner itself; `observations` from a
/// toolchain probe. Both are evidence — neither is assumed.
pub fn evaluate_pool_eligibility(
    node_id: &str,
    declaration: &PoolDeclaration,
    enrollment: &EnrollmentEvidence,
    observed_labels: &[String],
    observations: &[ToolObservation],
) -> PoolEligibility {
    let declared: BTreeSet<&str> = declaration.labels.iter().map(|s| s.as_str()).collect();
    let observed: BTreeSet<&str> = observed_labels.iter().map(|s| s.as_str()).collect();

    let missing_labels: Vec<String> = declared
        .iter()
        .filter(|label| !observed.contains(*label))
        .map(|label| label.to_string())
        .collect();
    let undeclared_labels: Vec<String> = observed
        .iter()
        .filter(|label| !declared.contains(*label))
        .map(|label| label.to_string())
        .collect();

    let mut reasons = Vec::new();
    let base_enrolled = enrollment.is_base_enrolled();
    if !base_enrolled {
        reasons.push(if !enrollment.enrolled {
            "node is not enrolled: no base enrollment record (enroll before claiming a role)"
                .to_string()
        } else {
            format!(
                "enrollment is {}: base enrollment must be past stub before a role is attributed",
                enrollment.status
            )
        });
    }

    let mut tools = Vec::new();
    for requirement in &declaration.required_tools {
        let observation = observations.iter().find(|o| o.name == requirement.name);
        let mut check = ToolCheck {
            name: requirement.name.clone(),
            required: requirement.required,
            present: observation.map(|o| o.present).unwrap_or(false),
            path: observation.and_then(|o| o.path.clone()),
            version: observation.and_then(|o| o.version.clone()),
            detail: None,
        };
        if !check.present {
            check.detail = Some(format!("{} not found in PATH", requirement.name));
        } else if let Some(minimum) = &requirement.min_version {
            match observation.and_then(|o| o.version.as_deref()) {
                Some(banner) => {
                    let extracted = extract_version(banner);
                    match extracted
                        .as_deref()
                        .and_then(|version| compare_versions(version, minimum))
                    {
                        Some(std::cmp::Ordering::Less) => {
                            check.present = false;
                            check.detail = Some(format!(
                                "{} is older than required {minimum}",
                                extracted.unwrap_or_default()
                            ));
                        }
                        Some(_) => {}
                        None => {
                            check.present = false;
                            check.detail = Some(format!(
                                "version {banner} is not comparable to required {minimum}"
                            ));
                        }
                    }
                }
                None => {
                    check.present = false;
                    check.detail = Some(format!("version unknown, required {minimum}"));
                }
            }
        }
        if requirement.required && !check.present {
            reasons.push(format!(
                "required tool unavailable: {}",
                check.detail.clone().unwrap_or_default()
            ));
        }
        tools.push(check);
    }

    if !missing_labels.is_empty() {
        reasons.push(format!(
            "declared labels not advertised by the runner: {}",
            missing_labels.join(", ")
        ));
    }
    if !undeclared_labels.is_empty() {
        reasons.push(format!(
            "runner advertises undeclared labels: {}",
            undeclared_labels.join(", ")
        ));
    }

    PoolEligibility {
        schema: POOL_ELIGIBILITY_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        declared_labels: declaration.labels.clone(),
        observed_labels: observed_labels.to_vec(),
        missing_labels,
        undeclared_labels,
        enrollment: enrollment.clone(),
        base_enrolled,
        tools,
        eligible: reasons.is_empty(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn requirement(name: &str) -> ToolRequirement {
        ToolRequirement {
            name: name.to_string(),
            min_version: None,
            required: true,
        }
    }

    fn enrolled() -> EnrollmentEvidence {
        EnrollmentEvidence {
            status: "observed".to_string(),
            enrolled: true,
            enrolled_by: Some("hermes".to_string()),
            enrolled_at: Some("2026-10-19".to_string()),
        }
    }

    fn stub_enrollment() -> EnrollmentEvidence {
        EnrollmentEvidence {
            status: "stub".to_string(),
            enrolled: false,
            enrolled_by: None,
            enrolled_at: None,
        }
    }

    fn observation(name: &str, present: bool, version: Option<&str>) -> ToolObservation {
        ToolObservation {
            name: name.to_string(),
            present,
            path: present.then(|| format!("/usr/bin/{name}")),
            version: version.map(|value| value.to_string()),
        }
    }

    #[test]
    fn version_comparison_orders_dotted_versions() {
        assert_eq!(
            compare_versions("1.2.3", "1.2.3"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_versions("1.10.0", "1.9.9"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_versions("v22.23.3", "22.0.0"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(compare_versions("not-a-version", "1.0.0"), None);
    }

    #[test]
    fn unenrolled_node_cannot_serve_a_label() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &stub_enrollment(),
            &labels(&["cynic/build"]),
            &[],
        );
        assert!(!report.eligible);
        assert!(!report.base_enrolled);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("not enrolled")));
    }

    #[test]
    fn stub_enrollment_with_a_stamp_still_blocks() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &EnrollmentEvidence {
                status: "stub".to_string(),
                enrolled: true,
                enrolled_by: Some("hermes".to_string()),
                enrolled_at: Some("2026-10-19".to_string()),
            },
            &labels(&["cynic/build"]),
            &[],
        );
        assert!(!report.eligible);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("must be past stub")));
    }

    #[test]
    fn version_token_is_extracted_from_a_prose_banner() {
        assert_eq!(
            extract_version("cargo 1.95.0 (f2d3ce0bd 2026-03-21)").as_deref(),
            Some("1.95.0")
        );
        assert_eq!(extract_version("v22.23.3").as_deref(), Some("22.23.3"));
        assert_eq!(extract_version("no version here"), None);
    }

    #[test]
    fn missing_required_tool_makes_the_node_ineligible() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![requirement("node")],
        };
        let report = evaluate_pool_eligibility(
            "compute-01",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build"]),
            &[observation("node", false, None)],
        );
        assert!(!report.eligible);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("required tool unavailable")));
    }

    #[test]
    fn tool_older_than_minimum_is_ineligible() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![ToolRequirement {
                name: "cargo".to_string(),
                min_version: Some("1.95.0".to_string()),
                required: true,
            }],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build"]),
            &[observation("cargo", true, Some("cargo 1.90.0"))],
        );
        assert!(!report.eligible);
        assert!(report.tools[0]
            .detail
            .clone()
            .unwrap_or_default()
            .contains("older than required"));
    }

    #[test]
    fn tool_newer_than_minimum_is_eligible() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![ToolRequirement {
                name: "cargo".to_string(),
                min_version: Some("1.95.0".to_string()),
                required: true,
            }],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build"]),
            &[observation("cargo", true, Some("cargo 1.99.0"))],
        );
        assert!(report.eligible, "reasons: {:?}", report.reasons);
    }

    #[test]
    fn declared_label_absent_from_the_runner_is_ineligible() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build", "cynic/linux"]),
            required_tools: vec![],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build"]),
            &[],
        );
        assert!(!report.eligible);
        assert_eq!(report.missing_labels, vec!["cynic/linux".to_string()]);
    }

    #[test]
    fn undeclared_label_on_the_runner_is_reported() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build", "rogue"]),
            &[],
        );
        assert!(!report.eligible);
        assert_eq!(report.undeclared_labels, vec!["rogue".to_string()]);
    }

    #[test]
    fn matching_labels_and_proven_tools_are_eligible() {
        let declaration = PoolDeclaration {
            labels: labels(&["cynic/build"]),
            required_tools: vec![requirement("cargo")],
        };
        let report = evaluate_pool_eligibility(
            "n1",
            &declaration,
            &enrolled(),
            &labels(&["cynic/build"]),
            &[observation("cargo", true, Some("cargo 1.95.0"))],
        );
        assert!(report.eligible, "reasons: {:?}", report.reasons);
        assert!(report.reasons.is_empty());
    }
}
