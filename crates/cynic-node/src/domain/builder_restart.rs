//! Pure builder restart domain logic — zero I/O.

/// Pure restart decision — the testable core, no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderRestartDecision {
    AlreadyConverged,
    Proceed { old_hash: String, new_hash: String },
    Refused { reason: String },
}

pub fn decide_builder_restart(
    running_hash: &str,
    installed_hash: Option<&str>,
    desired: &str,
) -> BuilderRestartDecision {
    let well_formed = desired.len() == 64 && desired.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !well_formed {
        return BuilderRestartDecision::Refused {
            reason: "bad desired digest: want 64 hex chars".to_string(),
        };
    }
    if running_hash.eq_ignore_ascii_case(desired) {
        return BuilderRestartDecision::AlreadyConverged;
    }
    match installed_hash {
        Some(installed) if installed.eq_ignore_ascii_case(desired) => {
            BuilderRestartDecision::Proceed {
                old_hash: running_hash.to_string(),
                new_hash: desired.to_string(),
            }
        }
        _ => BuilderRestartDecision::Refused {
            reason: "installed binary does not match desired; install first, then signal"
                .to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNNING: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const INSTALLED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn restart_refuses_malformed_desired_digest() {
        assert!(matches!(
            decide_builder_restart(RUNNING, Some(INSTALLED), "not-hex"),
            BuilderRestartDecision::Refused { .. }
        ));
    }

    #[test]
    fn restart_converged_when_running_matches_desired() {
        assert_eq!(
            decide_builder_restart(RUNNING, Some(INSTALLED), RUNNING),
            BuilderRestartDecision::AlreadyConverged
        );
    }

    #[test]
    fn restart_proceeds_when_installed_matches_desired() {
        assert_eq!(
            decide_builder_restart(RUNNING, Some(INSTALLED), INSTALLED),
            BuilderRestartDecision::Proceed {
                old_hash: RUNNING.to_string(),
                new_hash: INSTALLED.to_string(),
            }
        );
    }

    #[test]
    fn restart_refuses_when_installed_differs_from_desired() {
        // Install first, then signal — never the reverse. A bounce into
        // the same old binary is a no-op with downtime.
        assert!(matches!(
            decide_builder_restart(RUNNING, Some(RUNNING), INSTALLED),
            BuilderRestartDecision::Refused { .. }
        ));
        assert!(matches!(
            decide_builder_restart(RUNNING, None, INSTALLED),
            BuilderRestartDecision::Refused { .. }
        ));
    }
}
