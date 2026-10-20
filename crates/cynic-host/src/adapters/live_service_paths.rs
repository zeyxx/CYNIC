//! Detect the paths a live system unit executes from.
//!
//! A daemon whose binary lives inside a reclaimable tree is a trap: the
//! retention policy would delete the binary of a running service. The
//! builder's soak daemon executed
//! `/var/lib/cynic/shared-target/debug/cynic` — inside the Cargo cache the
//! policy was free to reclaim.
//!
//! The detection reads the unit files and resolves the `ExecStart` path
//! against each candidate path. It is a read-only observation: it never
//! touches a service, it only reports what a service is executing from.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A unit file and the executable it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitExecutable {
    /// The unit name, e.g. `cynic-supervise-soak-node.service`.
    pub unit: String,
    /// The absolute path the unit executes.
    pub executable: PathBuf,
}

/// The unit directories systemd reads, in the order it reads them.
const SYSTEM_UNIT_DIRS: &[&str] = &["/etc/systemd/system", "/usr/lib/systemd/system"];
const USER_UNIT_DIRS: &[&str] = &["/etc/systemd/user", "/usr/lib/systemd/user"];

/// Read every `ExecStart` executable declared by the visible unit files.
///
/// The scan is filesystem-based rather than systemd-based: it works on a
/// host where systemd is not reachable from the calling process, and it
/// needs no privilege. A unit that cannot be read is skipped, not guessed.
pub fn observe_unit_executables() -> Vec<UnitExecutable> {
    let mut found = Vec::new();
    for root in SYSTEM_UNIT_DIRS.iter().chain(USER_UNIT_DIRS.iter()) {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_unit = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == "service");
            if !is_unit {
                continue;
            }
            let unit = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            for executable in exec_start_paths(&content) {
                found.push(UnitExecutable { unit, executable });
            }
        }
    }
    found
}

/// Extract the absolute paths named by the `ExecStart=` directives.
///
/// Only absolute paths are considered: a relative `ExecStart` is resolved by
/// systemd against a working directory this scan does not know, and a
/// guessed path is worse than none.
fn exec_start_paths(unit_content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in unit_content.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("ExecStart=") else {
            continue;
        };
        for token in rest.split_whitespace() {
            if token.starts_with('-') {
                continue;
            }
            if token.starts_with('/') {
                paths.push(PathBuf::from(token));
                break;
            }
        }
    }
    paths
}

/// True when a live unit executes a binary from inside `candidate`.
///
/// The check is containment-based: a unit running
/// `/var/lib/cynic/shared-target/debug/cynic` makes
/// `/var/lib/cynic/shared-target` a path that must not be reclaimed.
pub fn is_live_service_path(candidate: &Path, executables: &[UnitExecutable]) -> bool {
    executables.iter().any(|unit| {
        unit.executable.starts_with(candidate) || candidate.starts_with(&unit.executable)
    })
}

/// The set of candidate paths that at least one live unit executes from.
pub fn live_service_paths(executables: &[UnitExecutable]) -> BTreeSet<PathBuf> {
    executables
        .iter()
        .map(|unit| unit.executable.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absolute_exec_start_is_read() {
        let unit = "[Service]\nExecStart=/var/lib/cynic/shared-target/debug/cynic supervise run\n";
        assert_eq!(
            exec_start_paths(unit),
            vec![PathBuf::from("/var/lib/cynic/shared-target/debug/cynic")]
        );
    }

    #[test]
    fn a_relative_exec_start_is_not_guessed() {
        let unit = "[Service]\nExecStart=cynic supervise run\n";
        assert!(exec_start_paths(unit).is_empty());
    }

    #[test]
    fn a_supervisor_prefix_is_skipped() {
        let unit = "[Service]\nExecStart=-/usr/bin/foo --flag\n";
        assert_eq!(exec_start_paths(unit), vec![PathBuf::from("/usr/bin/foo")]);
    }

    #[test]
    fn a_unit_executing_inside_a_cache_marks_that_cache_live() {
        let executables = vec![UnitExecutable {
            unit: "cynic-supervise-soak-node.service".to_string(),
            executable: PathBuf::from("/var/lib/cynic/shared-target/debug/cynic"),
        }];
        assert!(is_live_service_path(
            Path::new("/var/lib/cynic/shared-target"),
            &executables
        ));
        assert!(is_live_service_path(
            Path::new("/var/lib/cynic/shared-target/debug"),
            &executables
        ));
        assert!(!is_live_service_path(
            Path::new("/var/lib/cynic/build-targets"),
            &executables
        ));
    }

    #[test]
    fn a_unit_outside_the_candidate_does_not_protect_it() {
        let executables = vec![UnitExecutable {
            unit: "other.service".to_string(),
            executable: PathBuf::from("/usr/local/bin/cynic"),
        }];
        assert!(!is_live_service_path(
            Path::new("/var/lib/cynic/shared-target"),
            &executables
        ));
    }
}
