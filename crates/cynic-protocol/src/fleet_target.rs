//! Names of fleet targets, as `infra/registry.json` declares them.
//!
//! The registry has carried `transports.ssh.target` for every node from the start, and
//! nothing read it. Resolution was duplicated instead: six argv sites in
//! `cynic-host/src/agent/cleanup.rs` and a CLI default in `build_surface.rs` all spelled
//! the same host, and the copy that actually resolved it was `~/.ssh/config` — outside
//! the repository, outside every contract, outside every audit.
//!
//! This holds the *name*, not the address. Placement stays in the registry, which is
//! where it belongs and where it is already correct. The name has one home and is pinned
//! to the registry by `registry_names_match_the_pinned_targets`, so the two cannot drift
//! apart without a test going red.
//!
//! It lives in cynic-protocol rather than cynic-identity because cynic-identity would be
//! a new dependency for the consumer, and a new dependency is a larger decision than this
//! fact deserves. Being visible where the callers already are beats being ideally placed.

/// The build runner. The only host CYNIC builds on.
pub const BUILD_RUNNER_SSH_TARGET: &str = "cynic-builder-ssh";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pinned_name_is_the_one_the_registry_declares() {
        let registry =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../infra/registry.json"))
                .expect("registry.json is readable");
        assert!(
            registry.contains(&format!("\"target\": \"{BUILD_RUNNER_SSH_TARGET}\"")),
            "the pinned runner name no longer appears in infra/registry.json; one of the two \
             has moved and the pin must be updated deliberately"
        );
    }
}
