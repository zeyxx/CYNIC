//! Per-crate stack sizing: the runner stops reserving 8 GiB for crates that
//! never asked for it.
//!
//! THE MEASURED FAILURE. On 2026-10-19 the builder refused to spawn a
//! compiler thread:
//!
//! ```text
//! error: process didn't exit successfully: `sccache rustc -vV` (exit status: 101)
//! thread 'main' panicked at src/jobserver.rs:111:18:
//! failed to spawn helper thread: Os { code: 11, kind: WouldBlock,
//! message: "Resource temporarily unavailable" }
//! ```
//!
//! `WouldBlock` on a thread spawn is the kernel refusing a virtual-memory
//! commitment. The builder's own counters said why:
//! `Committed_AS = 366 991 272 kB` against `CommitLimit = 16 517 696 kB` —
//! **22x the quota**. The runner set `RUST_MIN_STACK=8589934592` (8 GiB)
//! for EVERY rustc thread of the WHOLE workspace, while
//! `infra/rust-toolchain.json` records a `min_stack` for only 6 of the 48
//! crates. The other 42 were reserving 8 GiB they never needed.
//!
//! WHY THE REGISTRY IS THE AUTHORITY. `infra/rust-toolchain.json` already
//! records, per crate, the stack that crate's test harness was measured to
//! need (`cynic-node` at 8 GiB, `cynic-kernel` at 4 GiB, the rest at the
//! 256 MiB default). `docs/design/CI_SOVEREIGNTY.md` states the rule that
//! makes it binding: the 8 GiB stack is safe on a host of the ">= 32 GiB RAM
//! class", and the builder is an 8 GiB host — so the 8 GiB stack must apply
//! to the crates that need it, not to the workspace.
//!
//! WHAT THIS DECIDES. The stack size the runner reserves for a build,
//! derived from the crates that build actually touches. Pure: the caller
//! supplies the registry data, this module decides the number.

use serde::{Deserialize, Serialize};

/// The stack the workspace reserves when no crate demands more.
pub const DEFAULT_MIN_STACK: u64 = 268_435_456; // 256 MiB

/// The registry seed (`infra/rust-toolchain.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolchainRegistry {
    #[serde(default)]
    pub crates: std::collections::BTreeMap<String, ToolchainEntry>,
}

/// One crate's toolchain facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolchainEntry {
    #[serde(default)]
    pub toolchain: Option<String>,
    /// The stack this crate's build was measured to need, as a decimal
    /// string (the registry stores bytes as a string).
    #[serde(default)]
    pub min_stack: Option<String>,
}

impl ToolchainRegistry {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// The stack a crate needs, or the workspace default when the registry
    /// says nothing about it.
    pub fn min_stack_for(&self, crate_name: &str) -> u64 {
        self.crates
            .get(crate_name)
            .and_then(|entry| entry.min_stack.as_deref())
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_MIN_STACK)
    }
}

/// The stack to reserve for a build touching these crates.
///
/// The answer is the MAXIMUM over the touched crates: a build that includes
/// `cynic-node` must reserve the 8 GiB that `cynic-node` was measured to
/// need, and a build that touches none of them reserves only the default.
///
/// WHY THE MAXIMUM AND NOT A SUM. The reservation is per THREAD, and the
/// threads are shared across the crates in one build. Summing would reserve
/// 8 GiB six times over for a six-crate build; the maximum is what the
/// largest single crate actually needs.
pub fn decide_min_stack(registry: &ToolchainRegistry, crates: &[String]) -> u64 {
    crates
        .iter()
        .map(|crate_name| registry.min_stack_for(crate_name))
        .max()
        .unwrap_or(DEFAULT_MIN_STACK)
}

/// The crates a build touches, from an optional package scope.
///
/// `None` means the whole workspace: every member is touched, so the
/// workspace's maximum applies. A package scope touches that package and
/// its path dependencies — which the caller resolves, because the
/// dependency graph is the workspace's business, not this module's.
pub fn crates_in_scope(package: Option<&str>, workspace_members: &[String]) -> Vec<String> {
    match package {
        Some(package) => vec![package.to_string()],
        None => workspace_members.to_vec(),
    }
}

/// The virtual-memory budget a reservation implies, for the admission gate.
///
/// One rustc thread reserves `min_stack` of virtual address space. The
/// commit quota is what the kernel enforces, so the budget that matters is
/// `threads * min_stack`. The thread count is what the builder runs with
/// (`CARGO_BUILD_JOBS`), plus the cargo jobserver's own threads.
pub fn committed_bytes(min_stack: u64, threads: u64) -> u64 {
    min_stack.saturating_mul(threads)
}

/// Whether a reservation fits inside a commit quota.
///
/// This is the gate that would have caught the failure: the runner knows the
/// stack it is about to reserve and the quota the host enforces, so it can
/// refuse a build it cannot hold BEFORE the kernel refuses a thread spawn
/// halfway through. An admission refusal is a verdict; a `WouldBlock` in the
/// middle of a build is a mystery.
pub fn fits_commit_quota(min_stack: u64, threads: u64, commit_limit_bytes: u64) -> bool {
    committed_bytes(min_stack, threads) <= commit_limit_bytes
}

/// The commit limit of a host, read from `/proc/meminfo` (`CommitLimit`).
///
/// The value is in kibibytes. A missing or unparseable line is `None`:
/// an unknown quota, never a zero.
pub fn parse_commit_limit(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("CommitLimit:")?;
        let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
        Some(kib.saturating_mul(1024))
    })
}

/// The committed virtual memory of a host, read from `/proc/meminfo`
/// (`Committed_AS`). The value is in kibibytes.
pub fn parse_committed_as(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("Committed_AS:")?;
        let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
        Some(kib.saturating_mul(1024))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> ToolchainRegistry {
        let mut crates = std::collections::BTreeMap::new();
        for name in ["cynic-node", "cynic-cli", "cynic-host", "cynic-inference"] {
            crates.insert(
                name.to_string(),
                ToolchainEntry {
                    toolchain: Some("1.95.0".to_string()),
                    min_stack: Some("8589934592".to_string()),
                },
            );
        }
        crates.insert(
            "cynic-kernel".to_string(),
            ToolchainEntry {
                toolchain: Some("1.95.0".to_string()),
                min_stack: Some("4294967296".to_string()),
            },
        );
        ToolchainRegistry { crates }
    }

    #[test]
    fn a_crate_without_an_entry_gets_the_workspace_default() {
        let registry = registry();
        assert_eq!(registry.min_stack_for("cynic-data"), DEFAULT_MIN_STACK);
    }

    #[test]
    fn a_crate_with_an_entry_gets_its_measured_stack() {
        let registry = registry();
        assert_eq!(registry.min_stack_for("cynic-node"), 8_589_934_592);
        assert_eq!(registry.min_stack_for("cynic-kernel"), 4_294_967_296);
    }

    #[test]
    fn the_stack_is_the_maximum_over_the_touched_crates() {
        let registry = registry();
        let crates = vec!["cynic-data".to_string(), "cynic-node".to_string()];
        assert_eq!(decide_min_stack(&registry, &crates), 8_589_934_592);
    }

    #[test]
    fn a_workspace_of_small_crates_reserves_only_the_default() {
        let registry = registry();
        let crates = vec![
            "cynic-data".to_string(),
            "cynic-fleet".to_string(),
            "cynic-intel-futarchy".to_string(),
        ];
        assert_eq!(decide_min_stack(&registry, &crates), DEFAULT_MIN_STACK);
    }

    #[test]
    fn an_empty_scope_reserves_only_the_default() {
        assert_eq!(decide_min_stack(&registry(), &[]), DEFAULT_MIN_STACK);
    }

    #[test]
    fn a_package_scope_touches_only_that_package() {
        let members = vec!["cynic-node".to_string(), "cynic-data".to_string()];
        assert_eq!(
            crates_in_scope(Some("cynic-node"), &members),
            vec!["cynic-node".to_string()]
        );
        assert_eq!(crates_in_scope(None, &members).len(), 2);
    }

    #[test]
    fn the_commit_budget_is_the_stack_times_the_threads() {
        assert_eq!(committed_bytes(DEFAULT_MIN_STACK, 4), 1_073_741_824);
    }

    #[test]
    fn a_reservation_that_exceeds_the_quota_is_refused() {
        // 8 GiB x 8 threads = 64 GiB against a 16 GiB quota: refused.
        assert!(!fits_commit_quota(8_589_934_592, 8, 16 * 1024 * 1024 * 1024));
        // 256 MiB x 8 threads = 2 GiB: fits.
        assert!(fits_commit_quota(DEFAULT_MIN_STACK, 8, 16 * 1024 * 1024 * 1024));
    }

    #[test]
    fn the_commit_quota_is_read_from_meminfo() {
        let meminfo = "MemTotal:       8039124 kB\nCommitLimit:    16517696 kB\nCommitted_AS:  366991272 kB\n";
        assert_eq!(
            parse_commit_limit(meminfo),
            Some(16_517_696u64 * 1024)
        );
        assert_eq!(
            parse_committed_as(meminfo),
            Some(366_991_272u64 * 1024)
        );
    }

    #[test]
    fn an_unreadable_quota_is_unknown_not_zero() {
        assert_eq!(parse_commit_limit("MemTotal: 1 kB\n"), None);
        assert_eq!(parse_committed_as("MemTotal: 1 kB\n"), None);
    }
}
