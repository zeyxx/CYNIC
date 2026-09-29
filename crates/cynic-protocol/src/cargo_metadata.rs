//! The one argument vector used to ask Cargo about the workspace.
//!
//! Six adapters across `cynic-host` each spelled this command, in three
//! orthographies, one of which added `--frozen` and five of which did not. Discovery
//! could therefore disagree with itself depending on which adapter answered, which is
//! the same failure as `CORE_CRATES` existing in the Makefile: a fact that belongs to
//! the port, copied into every place that needed it.
//!
//! `--frozen` is the choice, and it is measured, not preferred: it is `--locked` plus
//! `--offline`, it returns the 48 real workspace members on this tree, and it matches
//! the cache-integrity check `check.layers` already performs first. It also removes the
//! network from the answer, so two runs of discovery cannot disagree because the
//! registry moved underneath one of them.
//!
//! Pure: an argument vector, no process, no filesystem. Lives in the L0 crate so that
//! `cynic-host` and any future packaging adapter read the same one.

/// Arguments for `cargo metadata`, excluding the program name.
pub const CARGO_METADATA_ARGS: &[&str] = &["metadata", "--format-version", "1", "--no-deps", "--frozen"];

/// The full argv, including the program, for adapters that want one slice.
pub fn cargo_metadata_argv() -> Vec<String> {
    std::iter::once("cargo".to_string())
        .chain(CARGO_METADATA_ARGS.iter().map(|a| a.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vector_is_frozen_and_addresses_format_one() {
        assert!(
            CARGO_METADATA_ARGS.contains(&"--frozen"),
            "discovery must be offline and lock-checked, or two runs can disagree"
        );
        assert!(
            CARGO_METADATA_ARGS.windows(2).any(|w| w == ["--format-version", "1"]),
            "the split form is the one this tree is tested against; \
             the equals form is a second spelling of the same fact"
        );
        assert!(CARGO_METADATA_ARGS.contains(&"--no-deps"));
        assert_eq!(CARGO_METADATA_ARGS.first(), Some(&"metadata"));
    }

    #[test]
    fn argv_starts_with_the_program() {
        let argv = cargo_metadata_argv();
        assert_eq!(argv[0], "cargo");
        assert_eq!(argv.len(), CARGO_METADATA_ARGS.len() + 1);
    }
}
