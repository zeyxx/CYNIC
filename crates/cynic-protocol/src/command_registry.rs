//! Versioned command registry reader.
//!
//! The argv a builder executes for a canonical action is data, not code:
//! it lives in `infra/command-registry.json` (schema
//! `cynic.command_registry.v1`) and every composition site reads it from
//! here. This module is the single reader: pure functions over the embedded
//! registry document, so a command change is a data PR with proof, never a
//! code edit replicated across N tables.
//!
//! Scope: the registry is the authority for the FORWARD direction
//! (action → argv). The reverse direction (argv → action) stays handwritten
//! in the scheduler domain: it reads argv observed in the world (slice-scoped
//! cargo invocations, legacy make argv in old evidence) which the registry
//! never composes.

use std::collections::BTreeMap;

/// The registry document, embedded at build time. A parse failure here is a
/// broken build, surfaced by the `registry_parses` test, never a runtime
/// fallback: there is no second source.
const COMMAND_REGISTRY_JSON: &str = include_str!("../../../infra/command-registry.json");

fn registry_commands() -> Result<BTreeMap<String, Vec<String>>, String> {
    let document: serde_json::Value =
        serde_json::from_str(COMMAND_REGISTRY_JSON).map_err(|error| error.to_string())?;
    let commands = document
        .get("commands")
        .and_then(|commands| commands.as_object())
        .ok_or_else(|| "command registry has no commands object".to_string())?;
    let mut table = BTreeMap::new();
    for (action, entry) in commands {
        if let Some(argv) = entry.get("argv") {
            if argv.is_null() {
                continue;
            }
            let words = argv
                .as_array()
                .ok_or_else(|| format!("command registry argv not an array: {action}"))?;
            let mut composed = Vec::with_capacity(words.len());
            for word in words {
                composed.push(
                    word.as_str()
                        .ok_or_else(|| format!("command registry argv not strings: {action}"))?
                        .to_string(),
                );
            }
            table.insert(action.clone(), composed);
        }
    }
    Ok(table)
}

/// Argv the builder executes for `action`, or `None` when the registry names
/// no composable argv (unwired actions, unknown actions). Pure.
pub fn registry_argv(action: &str) -> Option<Vec<String>> {
    registry_commands().ok()?.remove(action)
}

/// Every action the registry composes, sorted. Pure. For lock-tests that pin
/// composition sites to this registry.
pub fn registry_composed_actions() -> Result<Vec<String>, String> {
    Ok(registry_commands()?.into_keys().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_parses_and_covers_the_native_vocabulary() {
        let actions = registry_composed_actions().expect("registry parses");
        for action in [
            "cargo:check",
            "cargo:build",
            "cargo:build-release",
            "cargo:test",
            "cargo:clippy",
            "cargo:fmt-check",
            "audit:crate",
            "audit:all",
        ] {
            assert!(
                actions.contains(&action.to_string()),
                "{action} is composed by the registry"
            );
        }
        assert_eq!(
            registry_argv("cargo:build"),
            Some(vec!["cargo".to_string(), "build".to_string()])
        );
        assert_eq!(
            registry_argv("cargo:build-release"),
            Some(vec![
                "cargo".to_string(),
                "build".to_string(),
                "--release".to_string()
            ])
        );
        // Unwired and unknown actions compose nothing.
        assert_eq!(registry_argv("cargo:run"), None);
        assert_eq!(registry_argv("make:build"), None);
        assert_eq!(registry_argv(""), None);
    }
}
