use std::path::Path;

use serde_json::{Map, Value, json};

pub fn build_lifecycle_runtime_paths(registry: &Value) -> Result<Value, String> {
    let lifecycle = kernel_release(registry)?;
    let lifecycle_name = "kernel_release";
    let runtime_paths = lifecycle
        .get("runtime_paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let activation_requirements = lifecycle
        .get("activation_requirements")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if runtime_paths.is_empty() && activation_requirements.is_empty() {
        return Ok(json!({
            "status": "not_applicable",
            "lifecycle": lifecycle_name,
            "paths": [],
            "blocked": false,
            "blockers": [],
            "reason": "lifecycle kernel_release does not declare runtime_paths or activation_requirements"
        }));
    }
    let required_status = lifecycle
        .get("required_node_status")
        .and_then(Value::as_str)
        .ok_or_else(|| "lifecycle required_node_status missing".to_string())?;
    let runtime_path_proofs = lifecycle
        .get("runtime_path_proofs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let mut paths = Vec::new();
    let mut blockers = Vec::new();

    for node in nodes {
        let Some(node_name) = node.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(role) = node.get("role").and_then(Value::as_str) else {
            continue;
        };
        let required_filesystem = activation_requirement_filesystem(lifecycle, role);
        if required_filesystem.is_empty() {
            continue;
        }
        let current_status = node
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if current_status != required_status {
            blockers.push(json!({
                "node": node_name,
                "role": role,
                "required_status": required_status,
                "current_status": current_status,
                "code": format!("node_status:{node_name}:{current_status}")
            }));
        }
        for path_name in required_filesystem {
            let Some(path_value) = runtime_paths.get(&path_name).and_then(Value::as_str) else {
                continue;
            };
            let proof = runtime_path_proofs
                .get(&path_name)
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            paths.push(json!({
                "node": node_name,
                "role": role,
                "path": path_name,
                "value": path_value,
                "proof": proof,
                "required_status": required_status,
                "current_status": current_status,
                "materialized_by": "lifecycle_activation",
                "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths"
            }));
        }
    }

    let status = if blockers.is_empty() { "ok" } else { "blocked" };
    Ok(json!({
        "status": status,
        "lifecycle": lifecycle_name,
        "paths": paths,
        "blocked": status != "ok",
        "blockers": blockers
    }))
}

pub fn verify_lifecycle_runtime_paths(
    node_name: &str,
    registry: &Value,
    home: Option<&str>,
    project_slug: Option<&str>,
) -> Result<Value, String> {
    if node_name.is_empty() {
        return Err("node_name is required".to_string());
    }
    let paths_payload = build_lifecycle_runtime_paths(registry)?;
    let paths = paths_payload
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| "paths payload missing".to_string())?;
    let home_path = home.unwrap_or("");
    let project_slug = project_slug.unwrap_or("CYNIC");
    let mut checks = Vec::new();
    for path in paths {
        if path.get("node").and_then(Value::as_str) != Some(node_name) {
            continue;
        }
        let role = path
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let path_name = path
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "runtime path name missing".to_string())?;
        let value = path
            .get("value")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("runtime path value missing for {path_name}"))?;
        let expanded = expand_systemd_user_path(value, home_path, project_slug);
        let proof = path
            .get("proof")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let marker = proof.get("marker").and_then(Value::as_str).unwrap_or("");
        let marker_path = if marker.is_empty() {
            String::new()
        } else {
            format!("{expanded}/{marker}")
        };
        let marker_exists = !marker_path.is_empty() && Path::new(&marker_path).exists();
        let git_probe = if proof.get("kind").and_then(Value::as_str) == Some("git_worktree") {
            git_worktree_probe(&expanded)
        } else {
            Map::new()
        };
        let sentinel_checks = proof
            .get("sentinels")
            .and_then(Value::as_array)
            .map(|sentinels| {
                sentinels
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|sentinel| {
                        let expanded_sentinel = format!("{expanded}/{sentinel}");
                        json!({
                            "path": sentinel,
                            "expanded": expanded_sentinel,
                            "exists": Path::new(&expanded_sentinel).exists()
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let sentinels_verified = !sentinel_checks.is_empty()
            && sentinel_checks
                .iter()
                .all(|item| item.get("exists").and_then(Value::as_bool).unwrap_or(false));
        let git_probe_ok = !git_probe.is_empty()
            && git_probe
                .values()
                .all(|value| value.as_bool().unwrap_or(false));
        let ok = proof.get("kind").and_then(Value::as_str) == Some("git_worktree")
            && marker_exists
            && git_probe_ok
            && sentinels_verified;
        checks.push(json!({
            "node": node_name,
            "role": role,
            "path": path_name,
            "value": value,
            "expanded": expanded,
            "proof_kind": proof.get("kind").and_then(Value::as_str),
            "marker": if marker.is_empty() { None::<String> } else { Some(marker.to_string()) },
            "marker_exists": marker_exists,
            "git_probe": git_probe,
            "sentinels": sentinel_checks,
            "ok": ok
        }));
    }
    if checks.is_empty() {
        return Err(format!("node {node_name} has no lifecycle runtime paths"));
    }
    let status = if checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool).unwrap_or(false))
    {
        "ok"
    } else {
        "blocked"
    };
    let verified_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let runtime_path_evidence = checks
        .iter()
        .filter_map(|check| {
            let path_name = check.get("path").and_then(Value::as_str)?;
            let sentinels_verified = check
                .get("sentinels")
                .and_then(Value::as_array)
                .map(|items| {
                    !items.is_empty()
                        && items.iter().all(|item| {
                            item.get("exists")
                                .and_then(Value::as_bool)
                                .unwrap_or(false)
                        })
                })
                .unwrap_or(false);
            Some((
                path_name.to_string(),
                json!({
                    "verified": check.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    "verified_at": verified_at,
                    "value": check.get("value").and_then(Value::as_str).unwrap_or(""),
                    "expanded": check.get("expanded").and_then(Value::as_str).unwrap_or(""),
                    "proof_kind": check.get("proof_kind").and_then(Value::as_str).unwrap_or(""),
                    "marker_exists": check.get("marker_exists").and_then(Value::as_bool).unwrap_or(false),
                    "git_probe": check.get("git_probe").cloned().unwrap_or(Value::Object(Map::new())),
                    "sentinels_verified": sentinels_verified
                }),
            ))
        })
        .collect::<Map<String, Value>>();
    Ok(json!({
        "status": status,
        "lifecycle": paths_payload.get("lifecycle").cloned().unwrap_or(Value::String("kernel_release".to_string())),
        "node": node_name,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "runtime_paths": runtime_path_evidence
            }
        }
    }))
}

fn kernel_release(registry: &Value) -> Result<&Value, String> {
    registry
        .get("lifecycle")
        .and_then(|value| value.get("kernel_release"))
        .ok_or_else(|| "registry lifecycle.kernel_release missing".to_string())
}

fn activation_requirement_filesystem(lifecycle: &Value, role: &str) -> Vec<String> {
    lifecycle
        .get("activation_requirements")
        .and_then(|value| value.get(role))
        .and_then(|value| value.get("filesystem"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn expand_systemd_user_path(value: &str, home: &str, project_slug: &str) -> String {
    let expanded = value.replace("${CYNIC_PROJECT_SLUG}", project_slug);
    if let Some(rest) = expanded.strip_prefix("%h/") {
        return format!("{home}/{rest}");
    }
    if expanded == "%h" {
        return home.to_string();
    }
    expanded
}

fn git_worktree_probe(expanded: &str) -> Map<String, Value> {
    let git_dir = Path::new(expanded).join(".git");
    [
        ("git_dir", git_dir.as_path()),
        ("git_head", git_dir.join("HEAD").as_path()),
        ("git_config", git_dir.join("config").as_path()),
        ("git_objects", git_dir.join("objects").as_path()),
    ]
    .into_iter()
    .map(|(name, path)| (name.to_string(), Value::Bool(path.exists())))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn registry_fixture() -> Value {
        json!({
            "nodes": [
                {"name": "cynic-vm-anvil", "role": "build_runner", "status": "planned"},
                {"name": "cynic-vm-core", "role": "kernel_primary", "status": "planned"}
            ],
            "lifecycle": {
                "kernel_release": {
                    "required_node_status": "active",
                    "runtime_paths": {
                        "repo_root": "/var/lib/cynic/repos/${CYNIC_PROJECT_SLUG}"
                    },
                    "runtime_path_proofs": {
                        "repo_root": {
                            "kind": "git_worktree",
                            "marker": ".git",
                            "sentinels": ["AGENTS.md", "infra/registry.json"]
                        }
                    },
                    "activation_requirements": {
                        "build_runner": {"filesystem": ["repo_root"]},
                        "kernel_primary": {"filesystem": ["repo_root"]}
                    }
                }
            }
        })
    }

    #[test]
    fn runtime_paths_are_registry_derived() {
        let payload = build_lifecycle_runtime_paths(&registry_fixture()).expect("runtime paths");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["paths"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn runtime_paths_are_not_applicable_without_activation_contract() {
        let payload = build_lifecycle_runtime_paths(&json!({
            "nodes": [],
            "lifecycle": {
                "kernel_release": {
                    "build_runner_role": "build_runner",
                    "deploy_target_role": "kernel_primary",
                    "host_bootstrap": {}
                }
            }
        }))
        .expect("runtime paths");
        assert_eq!(payload["status"], "not_applicable");
        assert_eq!(payload["blocked"], Value::Bool(false));
    }

    #[test]
    fn verify_runtime_paths_accepts_materialized_git_worktree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_root = dir.path().join(".cynic/repos/CYNIC");
        fs::create_dir_all(repo_root.join(".git/objects")).expect("objects");
        fs::create_dir_all(repo_root.join(".git/refs")).expect("refs");
        fs::write(repo_root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
        fs::write(
            repo_root.join(".git/config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .expect("config");
        fs::write(repo_root.join("AGENTS.md"), "# agents\n").expect("agents");
        fs::create_dir_all(repo_root.join("infra")).expect("infra");
        fs::write(repo_root.join("infra/registry.json"), "{}\n").expect("registry");

        let mut registry = registry_fixture();
        if let Some(lifecycle) = registry
            .get_mut("lifecycle")
            .and_then(|value| value.get_mut("kernel_release"))
            .and_then(|value| value.as_object_mut())
        {
            lifecycle.insert(
                "runtime_paths".into(),
                json!({
                    "repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"
                }),
            );
        }

        let payload = verify_lifecycle_runtime_paths(
            "cynic-vm-anvil",
            &registry,
            dir.path().to_str(),
            Some("CYNIC"),
        )
        .expect("verify");
        assert_eq!(payload["status"], "ok");
        assert_eq!(
            payload["activation_evidence_patch"]["activation_evidence"]["runtime_paths"]["repo_root"]
                ["verified"],
            Value::Bool(true)
        );
    }
}
