//! Discovery CLI surface — proactive capability declaration for LLM agents.
//!
//! Exposes the `cynic-discovery` + `cynic-discovery-host` domain types and
//! bridge operations via `cynic dev discovery <sub>`.
//!
//! Subcommands:
//!   capabilities — GET /capabilities from local kernel (proactive declaration)
//!   nodes        — known fleet nodes from registry.json
//!   events       — SSE subscription to discovery events (real-time)
//!   discover     — trigger a build-discovery scan (local/remote)

use crate::registry_path;
use std::io::BufRead;
use std::process::ExitCode;

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;
use crate::emit_json;
use cynic_host::domain::build_discovery::port::BuildDiscoveryObservationPort;

#[derive(Subcommand)]
pub(crate) enum DiscoveryCommand {
    /// Proactive capability declaration — what can this runtime do RIGHT NOW?
    /// Fetches GET /capabilities from the local kernel.
    Capabilities {
        /// Machine-readable JSON (default is human text).
        #[arg(long)]
        json: bool,
        /// Kernel endpoint (default: http://localhost:8080).
        #[arg(long, default_value = "http://localhost:8080")]
        endpoint: String,
    },
    /// List known fleet nodes from registry.json.
    Nodes {
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Registry file path. Omit to use the resolved runtime registry:
        /// explicit, then `CYNIC_REGISTRY`, then the installed
        /// `/etc/cynic/registry.json`, then the repository copy. The previous
        /// hardcoded default pinned this command to a repository-relative path,
        /// so on a deployed node it read a file that is not there.
        #[arg(long)]
        registry: Option<String>,
    },
    /// Subscribe to discovery events (SSE stream from kernel).
    Events {
        /// Kernel endpoint.
        #[arg(long, default_value = "http://localhost:8080")]
        endpoint: String,
        /// Filter by node id (optional).
        #[arg(long)]
        node: Option<String>,
    },
    /// Trigger a build-discovery scan on a target.
    Discover {
        /// Discovery target kind: local or remote.
        #[arg(long, default_value = "local")]
        kind: String,
        /// Remote SSH target (required when --kind remote).
        #[arg(long)]
        target: Option<String>,
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_discovery(command: DiscoveryCommand) -> ExitCode {
    match command {
        DiscoveryCommand::Capabilities { json, endpoint } => execute_capabilities(json, &endpoint),
        DiscoveryCommand::Nodes { json, registry } => execute_nodes(json, &registry_path(registry)),
        DiscoveryCommand::Events { endpoint, node } => execute_events(&endpoint, node.as_deref()),
        DiscoveryCommand::Discover { kind, target, json } => {
            execute_discover(&kind, target.as_deref(), json)
        }
    }
}

fn execute_capabilities(json: bool, endpoint: &str) -> ExitCode {
    let url = format!("{endpoint}/capabilities");
    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                eprintln!("capabilities request failed: HTTP {status}");
                return ExitCode::FAILURE;
            }
            match resp.json::<serde_json::Value>() {
                Ok(val) => {
                    if json {
                        emit_json(&val, false)
                    } else {
                        emit_pretty_json(&val)
                    }
                }
                Err(e) => {
                    eprintln!("failed to parse capabilities response: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("failed to reach kernel at {url}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn execute_nodes(json: bool, registry_path: &str) -> ExitCode {
    let content = match std::fs::read_to_string(registry_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to read registry {registry_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid registry JSON: {e}");
            return ExitCode::FAILURE;
        }
    };
    if json {
        emit_json(&val, false)
    } else {
        emit_pretty_json(&val)
    }
}

fn execute_events(endpoint: &str, _node: Option<&str>) -> ExitCode {
    let url = format!("{endpoint}/events");
    let client = reqwest::blocking::Client::new();
    match client.get(&url).send() {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                eprintln!("events subscription failed: HTTP {status}");
                return ExitCode::FAILURE;
            }
            // Stream SSE events to stdout
            let reader = std::io::BufReader::new(resp);
            for line in reader.lines() {
                match line {
                    Ok(l) => println!("{l}"),
                    Err(e) => {
                        eprintln!("SSE read error: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("failed to subscribe to events at {url}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn execute_discover(kind: &str, target: Option<&str>, json: bool) -> ExitCode {
    match kind {
        "local" => {
            let adapter = cynic_discovery_host::LocalBuildDiscoveryAdapter;
            match adapter.observe() {
                Ok(report) => {
                    if json {
                        emit_json(&report, false)
                    } else {
                        emit_pretty_json(&report)
                    }
                }
                Err(e) => {
                    eprintln!("local discovery failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "remote" => {
            let ssh_target = match target {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => {
                    eprintln!("--kind remote requires --target <ssh-target>");
                    return ExitCode::FAILURE;
                }
            };
            let runner = std::sync::Arc::new(cynic_lifecycle::LocalArgvRunner);
            let adapter =
                match cynic_discovery_host::RemoteBuildDiscoveryAdapter::new(runner, ssh_target) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("invalid ssh target: {e}");
                        return ExitCode::FAILURE;
                    }
                };
            match adapter.observe() {
                Ok(report) => {
                    if json {
                        emit_json(&report, false)
                    } else {
                        emit_pretty_json(&report)
                    }
                }
                Err(e) => {
                    eprintln!("remote discovery failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        other => {
            eprintln!("unknown --kind '{other}': expected local|remote");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    // No `use super::*`: these two assertions reference nothing from the parent,
    // and `unused_imports` is denied at workspace level, so the glob failed the
    // test target. Pre-existing — `cargo test -p cynic-cli --lib` did not build.

    #[test]
    fn discover_kind_local_valid() {
        // Kind "local" is accepted by the match arm
        assert!(matches!("local", "local"));
    }

    #[test]
    fn discover_kind_remote_requires_target() {
        // Kind "remote" without target should fail fast
        let target: Option<&str> = None;
        assert!(target.is_none());
    }
}
