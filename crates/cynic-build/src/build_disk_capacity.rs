//! Pure disk-capacity decision for the build runner.
//!
//! The builder disk is the substrate every build depends on. A checkout that
//! cannot be materialized is a build that fails with ENOSPC halfway through,
//! which reads to the operator like a broken toolchain and is not one.
//!
//! The decision is pure: the adapter observes the disk, this module decides.

use serde::{Deserialize, Serialize};

/// Schema identifier of the disk capacity decision.
pub const DISK_CAPACITY_DECISION_SCHEMA: &str = "cynic.build_runner.disk_capacity.v1";

/// Outcome of the pre-trigger disk gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskCapacityDecision {
    /// The runner has room; the checkout may be materialized.
    Proceed,
    /// The runner is below the minimum free bytes. The build must not start:
    /// materializing another checkout would fail mid-fetch.
    Refused {
        available_bytes: u64,
        minimum_bytes: u64,
    },
    /// The disk state could not be observed. Fail closed: an unknown disk is
    /// not a free disk, and materializing a checkout on an unknown disk is
    /// how the ENOSPC incident of 2026-09-08 happened.
    Unknown { reason: String },
}

/// The full decision record, with the read-back that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiskCapacityAssessment {
    pub schema: &'static str,
    pub decision: DiskCapacityDecision,
    /// Bytes observed free on the runner, when the read-back succeeded.
    pub available_bytes: Option<u64>,
    /// Minimum free bytes the policy requires.
    pub minimum_available_bytes: u64,
}

/// Decide whether a checkout may be materialized on the runner.
///
/// `available_bytes` is `None` when the read-back failed. The decision is
/// fail-closed: an unobserved disk blocks the build with an explicit
/// unknown, never a silent proceed.
pub fn decide_disk_capacity(
    available_bytes: Option<u64>,
    minimum_available_bytes: u64,
) -> DiskCapacityAssessment {
    let decision = match available_bytes {
        None => DiskCapacityDecision::Unknown {
            reason: "runner_disk_space_unobserved".to_string(),
        },
        Some(bytes) if bytes < minimum_available_bytes => DiskCapacityDecision::Refused {
            available_bytes: bytes,
            minimum_bytes: minimum_available_bytes,
        },
        Some(_) => DiskCapacityDecision::Proceed,
    };
    DiskCapacityAssessment {
        schema: DISK_CAPACITY_DECISION_SCHEMA,
        decision,
        available_bytes,
        minimum_available_bytes,
    }
}

/// Parse the available bytes from a `df -k` read-back of the checkout base.
///
/// Two shapes are accepted, because the probe and the parser must agree:
/// - full form (`df -k <path>`): header plus ≥5 columns; the fourth is the
///   available space in 1 KiB blocks. The line whose mount point is the
///   queried path wins; otherwise the first filesystem is the fallback.
/// - single-column form (`df -k --output=avail <path>`): the only field
///   after the header is the available KiB.
///
/// A missing or unparsable line yields `None`: an unknown disk is not a
/// measured disk.
pub fn parse_df_available_bytes(df_output: &str, query_path: &str) -> Option<u64> {
    let mut fallback: Option<u64> = None;
    for line in df_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        if parts.len() == 1 {
            if fallback.is_none() {
                if let Ok(available_kb) = parts[0].parse::<u64>() {
                    fallback = Some(available_kb * 1024);
                }
            }
            continue;
        }
        if parts.len() < 5 {
            continue;
        }
        let available_kb: u64 = match parts[3].parse() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let mount_point = parts.last().copied().unwrap_or_default();
        if mount_point == query_path {
            return Some(available_kb * 1024);
        }
        if fallback.is_none() {
            fallback = Some(available_kb * 1024);
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_healthy_disk_proceeds() {
        let assessment =
            decide_disk_capacity(Some(20 * 1024 * 1024 * 1024), 4 * 1024 * 1024 * 1024);
        assert_eq!(assessment.decision, DiskCapacityDecision::Proceed);
        assert_eq!(assessment.available_bytes, Some(20 * 1024 * 1024 * 1024));
    }

    #[test]
    fn a_full_disk_is_refused_with_the_measured_numbers() {
        let assessment = decide_disk_capacity(Some(1024 * 1024 * 1024), 4 * 1024 * 1024 * 1024);
        assert_eq!(
            assessment.decision,
            DiskCapacityDecision::Refused {
                available_bytes: 1024 * 1024 * 1024,
                minimum_bytes: 4 * 1024 * 1024 * 1024,
            }
        );
    }

    #[test]
    fn an_unobserved_disk_fails_closed() {
        let assessment = decide_disk_capacity(None, 4 * 1024 * 1024 * 1024);
        assert_eq!(
            assessment.decision,
            DiskCapacityDecision::Unknown {
                reason: "runner_disk_space_unobserved".to_string()
            }
        );
    }

    #[test]
    fn the_boundary_is_inclusive_on_the_minimum() {
        let minimum = 4 * 1024 * 1024 * 1024;
        let assessment = decide_disk_capacity(Some(minimum), minimum);
        assert_eq!(assessment.decision, DiskCapacityDecision::Proceed);
    }

    #[test]
    fn single_column_output_avail_parses() {
        let df = "Avail\n88144916\n";
        assert_eq!(
            parse_df_available_bytes(df, "/var/lib/cynic"),
            Some(88144916 * 1024)
        );
    }

    #[test]
    fn df_available_bytes_reads_the_mount_point_line() {
        let output = "Filesystem     1K-blocks     Used Available Use% Mounted on\n\
             /dev/sda1      100000000 50000000  50000000  50% /var/lib/cynic\n\
             /dev/sdb1      200000000 100000000 100000000  50% /other\n";
        assert_eq!(
            parse_df_available_bytes(output, "/var/lib/cynic"),
            Some(50_000_000 * 1024)
        );
    }

    #[test]
    fn df_available_bytes_falls_back_to_the_first_filesystem() {
        let output = "Filesystem     1K-blocks     Used Available Use% Mounted on\n\
             /dev/sda1      100000000 50000000  50000000  50% /\n";
        assert_eq!(
            parse_df_available_bytes(output, "/var/lib/cynic/checkouts"),
            Some(50_000_000 * 1024)
        );
    }

    #[test]
    fn an_unparsable_df_yields_no_measurement() {
        assert_eq!(
            parse_df_available_bytes("garbage\n", "/var/lib/cynic"),
            None
        );
        assert_eq!(parse_df_available_bytes("", "/var/lib/cynic"), None);
    }
}
