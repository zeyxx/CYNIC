use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::adapters::dns_renderer::SynapseDnsRenderer;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::observe;
use crate::ports::network_interfaces_port::{DnsBundleRendererPort, RegistryTopologyPort};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkShadowProofReport {
    schema: &'static str,
    generated_at: String,
    registry: String,
    repo_root: String,
    python: ShadowProofPython,
    rust: ShadowProofRust,
    drift: Vec<ShadowDrift>,
    pub(crate) status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ShadowProofPython {
    dns_bundle_dir: String,
    dns_bundle_files: Value,
    mesh_status: Value,
}

#[derive(Debug, Clone, Serialize)]
struct ShadowProofRust {
    dns_bundle: RustDnsBundle,
    mesh_status: RustMeshStatus,
}

#[derive(Debug, Clone, Serialize)]
struct RustDnsBundle {
    activation_plan: Value,
    synapse_hosts: String,
    corefile: String,
    resolved_dropin: String,
}

#[derive(Debug, Clone, Serialize)]
struct RustMeshStatus {
    runtime_status: String,
    dns_status: String,
    blocker_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ShadowDrift {
    kind: &'static str,
    detail: String,
}

pub(crate) fn build_shadow_proof(
    registry: &str,
    resolve_dns: bool,
    observe_runtime: bool,
) -> Result<NetworkShadowProofReport, Box<dyn Error>> {
    let repo_root = repo_root()?;
    let tempdir = tempfile::tempdir()?;
    let python_bundle_dir = tempdir.path().join("python-dns-bundle");
    std::fs::create_dir_all(&python_bundle_dir)?;

    let python_bundle = run_python_json(
        &repo_root,
        &[
            "organs/fleet/src/entrypoints/cli.py",
            "dns",
            "bundle",
            "--registry",
            registry,
            "--output-dir",
            python_bundle_dir.to_string_lossy().as_ref(),
        ],
    )?;
    let mut mesh_args = vec![
        "organs/fleet/src/entrypoints/cli.py",
        "mesh",
        "status",
        "--registry",
        registry,
    ];
    if resolve_dns {
        mesh_args.push("--resolve-dns");
    }
    if observe_runtime {
        mesh_args.push("--observe-runtime");
    }
    let python_mesh = run_python_json(&repo_root, &mesh_args)?;

    let registry_adapter = LocalRegistryAdapter::new(registry);
    let topology = registry_adapter.fetch_static_topology()?;
    let renderer = SynapseDnsRenderer;
    let activation_plan = renderer.activation_plan()?;
    let synapse_hosts = renderer.render_hosts(&topology)?;
    let corefile = renderer.render_corefile()?;
    let resolved_dropin = renderer.render_resolved_dropin()?;
    let rust_observation = observe::observe_payload(Some(registry));

    let rust = ShadowProofRust {
        dns_bundle: RustDnsBundle {
            activation_plan: serde_json::to_value(activation_plan)?,
            synapse_hosts: synapse_hosts.clone(),
            corefile: corefile.clone(),
            resolved_dropin: resolved_dropin.clone(),
        },
        mesh_status: RustMeshStatus {
            runtime_status: serde_json::to_value(&rust_observation)?["network_mesh_status"]["runtime_status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            dns_status: serde_json::to_value(&rust_observation)?["network_mesh_status"]["dns_status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            blocker_count: rust_observation.blockers.len(),
        },
    };

    let drift = compare_python_and_rust(
        &python_bundle_dir,
        &python_bundle,
        &python_mesh,
        &rust,
    )?;
    let status = if drift.is_empty() { "aligned" } else { "drift_detected" };

    Ok(NetworkShadowProofReport {
        schema: "cynic.node_network_shadow_proof.v1",
        generated_at: Utc::now().to_rfc3339(),
        registry: registry.to_string(),
        repo_root: repo_root.to_string_lossy().to_string(),
        python: ShadowProofPython {
            dns_bundle_dir: python_bundle_dir.to_string_lossy().to_string(),
            dns_bundle_files: python_bundle,
            mesh_status: python_mesh,
        },
        rust,
        drift,
        status,
    })
}

fn compare_python_and_rust(
    python_bundle_dir: &Path,
    python_bundle: &Value,
    python_mesh: &Value,
    rust: &ShadowProofRust,
) -> Result<Vec<ShadowDrift>, Box<dyn Error>> {
    let mut drift = Vec::new();
    let hosts = std::fs::read_to_string(python_bundle_dir.join("synapse.hosts"))?;
    let corefile = std::fs::read_to_string(python_bundle_dir.join("Corefile"))?;
    let resolved = std::fs::read_to_string(python_bundle_dir.join("cynic-synapse-dns.conf"))?;

    if normalize_text(&hosts) != normalize_text(&rust.dns_bundle.synapse_hosts) {
        drift.push(ShadowDrift {
            kind: "dns_hosts_mismatch",
            detail: "Python and Rust rendered different synapse.hosts content.".to_string(),
        });
    }
    if normalize_text(&corefile) != normalize_text(&rust.dns_bundle.corefile) {
        drift.push(ShadowDrift {
            kind: "dns_corefile_mismatch",
            detail: "Python and Rust rendered different Corefile content.".to_string(),
        });
    }
    if normalize_text(&resolved) != normalize_text(&rust.dns_bundle.resolved_dropin) {
        drift.push(ShadowDrift {
            kind: "dns_resolved_dropin_mismatch",
            detail: "Python and Rust rendered different resolved drop-in content.".to_string(),
        });
    }

    let python_bundle_schema = python_bundle
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if python_bundle_schema != "cynic.organ_fleet.dns_activation_bundle.v1" {
        drift.push(ShadowDrift {
            kind: "python_bundle_schema_unexpected",
            detail: format!("unexpected python dns bundle schema: {python_bundle_schema}"),
        });
    }

    let python_runtime_status = python_mesh
        .get("runtime_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if python_runtime_status != rust.mesh_status.runtime_status {
        drift.push(ShadowDrift {
            kind: "mesh_runtime_status_mismatch",
            detail: format!(
                "python runtime_status={} rust runtime_status={}",
                python_runtime_status, rust.mesh_status.runtime_status
            ),
        });
    }

    let python_dns_status = python_mesh
        .get("dns_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if python_dns_status != rust.mesh_status.dns_status {
        drift.push(ShadowDrift {
            kind: "mesh_dns_status_mismatch",
            detail: format!(
                "python dns_status={} rust dns_status={}",
                python_dns_status, rust.mesh_status.dns_status
            ),
        });
    }

    Ok(drift)
}

fn normalize_text(value: &str) -> String {
    value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn run_python_json(repo_root: &Path, args: &[&str]) -> Result<Value, Box<dyn Error>> {
    let output = Command::new("python3")
        .args(args)
        .current_dir(repo_root)
        .output()?;
    match serde_json::from_slice(&output.stdout) {
        Ok(value) => Ok(value),
        Err(parse_error) => {
            if !output.status.success() {
                return Err(format!(
                    "python command failed (status={}): stderr={} stdout={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim(),
                    String::from_utf8_lossy(&output.stdout).trim(),
                )
                .into());
            }
            Err(Box::new(parse_error))
        }
    }
}

fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_ignores_trailing_space_and_blank_edges() {
        assert_eq!(normalize_text("a \n b\n\n"), "a\n b");
    }
}
