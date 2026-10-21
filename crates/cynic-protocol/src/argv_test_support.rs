//! Test support for the argv membrane.
//!
//! WHY THIS EXISTS.
//!
//! The membrane is fail-closed: without an installed policy nothing is
//! admitted. That is the correct behaviour for a process, and it is the
//! wrong behaviour for a test that means to spawn a real process — the
//! test would fail with "no execution policy is installed" and prove
//! nothing about the behaviour it claims to cover.
//!
//! So a test that really spawns must install a policy that admits what
//! it spawns. This module is that helper, and it lives in the port
//! crate because the port owns the slot.
//!
//! WHY IT IS NOT `#[cfg(test)]`.
//!
//! The slot is process-wide and write-once. A test in `cynic-host` or
//! `cynic-sandbox` that spawns through `LocalArgvRunner` needs the same
//! helper, and a `#[cfg(test)]` module is not visible to the integration
//! tests of another crate. The helper is therefore a public item of the
//! port crate, and it can only ever widen the ruleset for the calling
//! process — it cannot weaken a structural refusal.

use crate::argv_allow::AllowRule;
use crate::execution_policy::ExecutionPolicy;

/// Install a policy that admits the given programs, and nothing else.
///
/// The programs are matched on their base name, so `/usr/bin/git` and
/// `git` are the same rule. A test that spawns `/usr/bin/echo` declares
/// `echo`.
///
/// This does NOT open the structural refusals. A local build is still
/// refused on a host that may not compile, and the `ssh` refusal still
/// applies. Those are decided by the policy itself, not by the ruleset.
pub fn install_permissive_policy_for(programs: &[&str]) -> usize {
    let rules: Vec<AllowRule> = programs
        .iter()
        .map(|program| AllowRule {
            program: (*program).to_string(),
            arg_prefix: Vec::new(),
            reason: "test fixture: the test spawns this program".to_string(),
        })
        .collect();
    let count = rules.len();
    crate::argv::install_execution_policy(ExecutionPolicy::new(rules));
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helper_installs_one_rule_per_program() {
        let count = install_permissive_policy_for(&["git", "true"]);
        assert_eq!(count, 2);
        let policy = crate::argv::installed_execution_policy().expect("installed");
        assert_eq!(policy.rules.len(), 2);
    }
}
