//! Human-oriented directory for advanced operator surfaces.

#![allow(dead_code)]
use std::process::ExitCode;

const ADVANCED_AREAS: &[(&str, &str)] = &[
    (
        "runtime",
        "runtime evidence, host conformance, and internal adapters",
    ),
    (
        "network",
        "DNS, mesh, provider fallback, and bounded network plans",
    ),
    ("fleet", "fleet-level observation reports"),
    ("bootstrap", "node bootstrap and privileged host activation"),
    (
        "integrations",
        "external service observation and connection plans",
    ),
    ("deps", "ecosystem dependency observation and upgrades"),
    (
        "lifecycle",
        "promotion gates, canaries, and repair workflows",
    ),
    ("dev", "source, contract, and architecture audits"),
    ("lab", "isolated proof harnesses"),
    ("transport", "transport inspection and security audits"),
    ("solana", "read-only Solana observation planning"),
    ("audit", "source and host-boundary audits"),
];

pub(crate) fn execute_advanced_help() -> ExitCode {
    println!("Advanced CYNIC surfaces");
    println!(
        "Run `cynic doctor` before an unfamiliar operation; use `cynic errors` to decode a failure. Mutations require their explicit consent flag."
    );
    println!();
    for (command, purpose) in ADVANCED_AREAS {
        println!("  cynic {command:<13} {purpose}");
    }
    println!();
    println!("Machine-readable complete command manifest: cynic commands --json");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::ADVANCED_AREAS;

    #[test]
    fn advanced_directory_keeps_runtime_surfaces_discoverable() {
        assert!(
            ADVANCED_AREAS
                .iter()
                .any(|(command, _)| *command == "runtime")
        );
        assert!(
            ADVANCED_AREAS
                .iter()
                .any(|(command, _)| *command == "network")
        );
    }
}
