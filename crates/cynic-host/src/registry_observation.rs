use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct RegistryObservationReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub registry_path: String,
    pub observation_path: String,
    pub node_id: String,
    pub observed_section: String,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}

pub fn project_registry_observation(
    registry_path: &str,
    observation_path: &str,
    apply: bool,
) -> Result<RegistryObservationReport, String> {
    let observation = read_json(observation_path, "observation")?;
    if observation["schema"] != "cynic.runtime.node_observation.v1" {
        return Err("observation schema must be cynic.runtime.node_observation.v1".to_string());
    }
    let node_id = observation["node_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "observation node_id is missing".to_string())?
        .to_string();
    if observation["observed_at_unix_ms"].as_u64().is_none() {
        return Err("observation observed_at_unix_ms is missing".to_string());
    }
    let mut registry = read_json(registry_path, "registry")?;
    let root = registry
        .as_object_mut()
        .ok_or_else(|| "registry root must be a JSON object".to_string())?;
    let observed = root
        .entry("observed")
        .or_insert_with(|| serde_json::json!({"nodes": {}}));
    let observed_nodes = observed
        .get_mut("nodes")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "registry observed.nodes must be a JSON object".to_string())?;
    observed_nodes.insert(node_id.clone(), observation);

    if apply {
        atomic_write_json(registry_path, &registry)?;
    }
    Ok(RegistryObservationReport {
        schema: "cynic.registry.observation_projection.v1",
        status: if apply { "applied" } else { "planned" },
        mutates: apply,
        registry_path: registry_path.to_string(),
        observation_path: observation_path.to_string(),
        node_id,
        observed_section: "observed.nodes".to_string(),
        blockers: if apply {
            Vec::new()
        } else {
            vec!["explicit_apply_required".to_string()]
        },
        rollback_intent: "restore the previous registry JSON from the captured pre-apply snapshot",
    })
}

fn read_json(path: &str, label: &str) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {label} {path}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {label} {path}: {error}"))
}

fn atomic_write_json(path: &str, value: &Value) -> Result<(), String> {
    let target = Path::new(path);
    let parent = target
        .parent()
        .ok_or_else(|| "registry path has no parent directory".to_string())?;
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "registry path has no valid filename".to_string())?;
    let temporary = parent.join(format!(".{file_name}.cynic-observation.tmp"));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize observed registry: {error}"))?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("write temporary registry projection: {error}"))?;
    fs::rename(&temporary, target)
        .map_err(|error| format!("atomically replace registry projection: {error}"))
}

#[cfg(test)]
mod tests {
    use super::project_registry_observation;

    #[test]
    fn observation_projection_is_planned_without_apply() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = dir.path().join("registry.json");
        let observation = dir.path().join("observation.json");
        std::fs::write(&registry, r#"{"nodes":[]}"#).expect("registry");
        std::fs::write(
            &observation,
            r#"{"schema":"cynic.runtime.node_observation.v1","node_id":"cortex-1","observed_at_unix_ms":1,"role":"cortex_host"}"#,
        )
        .expect("observation");

        let report = project_registry_observation(
            registry.to_str().expect("registry path"),
            observation.to_str().expect("observation path"),
            false,
        )
        .expect("projection");
        assert_eq!(report.status, "planned");
        assert!(
            !std::fs::read_to_string(&registry)
                .expect("read registry")
                .contains("cortex-1")
        );
    }

    #[test]
    fn observation_projection_applies_only_observed_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = dir.path().join("registry.json");
        let observation = dir.path().join("observation.json");
        std::fs::write(
            &registry,
            r#"{"nodes":[{"name":"cynic-host-cortex","role":"cortex_host"}]}"#,
        )
        .expect("registry");
        std::fs::write(
            &observation,
            r#"{"schema":"cynic.runtime.node_observation.v1","node_id":"cortex-1","observed_at_unix_ms":1,"role":"cortex_host"}"#,
        )
        .expect("observation");

        project_registry_observation(
            registry.to_str().expect("registry path"),
            observation.to_str().expect("observation path"),
            true,
        )
        .expect("projection");
        let content = std::fs::read_to_string(&registry).expect("read registry");
        assert!(content.contains("cortex-1"));
        assert!(content.contains("cynic-host-cortex"));
    }
}
