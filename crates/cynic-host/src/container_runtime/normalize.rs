use super::types::{
    DockerContainerObservation, DockerDaemonObservation, DockerImageObservation, DockerRuntimeKind,
    DockerStorageReport, DockerVolumeObservation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DockerStorageEvidence {
    pub info_json: Option<String>,
    pub system_df: Option<String>,
    pub containers_jsonl: Option<String>,
    pub images_jsonl: Option<String>,
    pub volumes_jsonl: Option<String>,
    pub failures: Vec<String>,
}

pub fn normalize_docker_evidence(
    evidence: &DockerStorageEvidence,
    observed_at_unix_ms: u128,
) -> DockerStorageReport {
    let daemon = parse_daemon(evidence.info_json.as_deref());
    let images = parse_images(evidence.images_jsonl.as_deref());
    let containers = parse_containers(evidence.containers_jsonl.as_deref());
    let volumes = parse_volumes(evidence.volumes_jsonl.as_deref());
    let summary = parse_system_df(evidence.system_df.as_deref());
    let mut residual_unknowns = evidence.failures.clone();
    if evidence.info_json.is_none() {
        residual_unknowns.push("docker info evidence is missing".into());
    }
    if evidence.system_df.is_none() {
        residual_unknowns.push("docker system df evidence is missing".into());
    }
    if images.iter().any(|image| image.unique_size_bytes.is_none()) {
        residual_unknowns.push("shared image layer sizes are not fully attributable".into());
    }
    if volumes.iter().any(|volume| volume.size_bytes.is_none()) {
        residual_unknowns.push("one or more Docker volume sizes are unknown".into());
    }
    DockerStorageReport {
        schema: "cynic.host.docker_storage_report.v1".into(),
        observed_at_unix_ms,
        daemon,
        images,
        containers,
        volumes,
        image_bytes: summary.image_bytes,
        container_bytes: summary.container_bytes,
        volume_bytes: summary.volume_bytes,
        reclaimable_bytes: summary.reclaimable_bytes,
        residual_unknowns,
    }
}

#[derive(Default)]
struct DockerSummary {
    image_bytes: Option<u64>,
    container_bytes: Option<u64>,
    volume_bytes: Option<u64>,
    reclaimable_bytes: Option<u64>,
}

fn parse_daemon(raw: Option<&str>) -> DockerDaemonObservation {
    let value = raw.and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    DockerDaemonObservation {
        reachable: value.is_some(),
        runtime: DockerRuntimeKind::Docker,
        server_version: value.as_ref().and_then(|v| string(v, "ServerVersion")),
        root_dir: value.as_ref().and_then(|v| string(v, "DockerRootDir")),
        storage_driver: value.as_ref().and_then(|v| string(v, "Driver")),
        snapshotter: None,
    }
}

fn parse_images(raw: Option<&str>) -> Vec<DockerImageObservation> {
    json_lines(raw)
        .into_iter()
        .filter_map(|value| {
            let id = string(&value, "ID")?;
            let repository = string(&value, "Repository");
            let reconstructible = repository
                .as_ref()
                .filter(|value| value.as_str() != "<none>")
                .map(|_| true);
            Some(DockerImageObservation {
                id,
                repository: repository.filter(|value| value != "<none>"),
                tag: string(&value, "Tag").filter(|value| value != "<none>"),
                logical_size_bytes: string(&value, "Size").and_then(|value| parse_size(&value)),
                unique_size_bytes: string(&value, "UniqueSize")
                    .and_then(|value| parse_size(&value)),
                container_refs: string(&value, "Containers")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_default(),
                reconstructible,
            })
        })
        .collect()
}

fn parse_containers(raw: Option<&str>) -> Vec<DockerContainerObservation> {
    json_lines(raw)
        .into_iter()
        .filter_map(|value| {
            let id = string(&value, "ID")?;
            let labels = string(&value, "Labels").unwrap_or_default();
            let mounts = string(&value, "Mounts").unwrap_or_default();
            Some(DockerContainerObservation {
                id,
                name: string(&value, "Names").unwrap_or_else(|| "unknown".into()),
                image_id: string(&value, "ImageID"),
                state: string(&value, "State").unwrap_or_else(|| "unknown".into()),
                project: label(&labels, "com.docker.compose.project")
                    .or_else(|| label(&labels, "kurtosis_enclave_name")),
                writable_layer_bytes: string(&value, "Size")
                    .and_then(|value| value.split(" (virtual").next().and_then(parse_size)),
                log_bytes: None,
                volume_refs: mounts
                    .split(',')
                    .filter(|mount| !mount.is_empty())
                    .map(str::to_string)
                    .collect(),
                created_at_unix_ms: None,
            })
        })
        .collect()
}

fn parse_volumes(raw: Option<&str>) -> Vec<DockerVolumeObservation> {
    json_lines(raw)
        .into_iter()
        .filter_map(|value| {
            let name = string(&value, "Name")?;
            let labels = string(&value, "Labels").unwrap_or_default();
            let anonymous = label(&labels, "com.docker.volume.anonymous").is_some();
            Some(DockerVolumeObservation {
                name,
                driver: string(&value, "Driver"),
                size_bytes: string(&value, "Size").and_then(|value| parse_size(&value)),
                active_links: string(&value, "Links")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_default(),
                project: label(&labels, "com.docker.compose.project"),
                persistent: Some(!anonymous),
            })
        })
        .collect()
}

fn parse_system_df(raw: Option<&str>) -> DockerSummary {
    let mut summary = DockerSummary::default();
    for line in raw.unwrap_or_default().lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(kind) = fields.first() else { continue };
        let (size_index, reclaim_index) = match *kind {
            "Images" | "Containers" | "Local"
                if *kind == "Local" && fields.get(1) == Some(&"Volumes") =>
            {
                (3, 4)
            }
            "Images" | "Containers" => (3, 4),
            _ => continue,
        };
        let size = fields.get(size_index).and_then(|value| parse_size(value));
        let reclaimable = fields
            .get(reclaim_index)
            .and_then(|value| parse_size(value.trim_end_matches('(')));
        match *kind {
            "Images" => summary.image_bytes = size,
            "Containers" => summary.container_bytes = size,
            "Local" => summary.volume_bytes = size,
            _ => {}
        }
        summary.reclaimable_bytes = Some(
            summary
                .reclaimable_bytes
                .unwrap_or_default()
                .saturating_add(reclaimable.unwrap_or_default()),
        );
    }
    summary
}

fn json_lines(raw: Option<&str>) -> Vec<Value> {
    raw.unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn label(labels: &str, wanted: &str) -> Option<String> {
    labels.split(',').find_map(|entry| {
        let (key, value) = entry.split_once('=')?;
        (key == wanted).then_some(value.to_string())
    })
}

fn parse_size(raw: &str) -> Option<u64> {
    let raw = raw.trim().trim_end_matches(',');
    if raw == "N/A" || raw == "-" || raw.is_empty() {
        return None;
    }
    let split = raw.find(|character: char| !(character.is_ascii_digit() || character == '.'))?;
    let number: f64 = raw[..split].parse().ok()?;
    let unit = raw[split..].to_ascii_lowercase();
    let multiplier = if unit.starts_with("tib") {
        1024_f64.powi(4)
    } else if unit.starts_with("tb") {
        1_000_000_000_000_f64
    } else if unit.starts_with("gib") {
        1024_f64.powi(3)
    } else if unit.starts_with("gb") {
        1_000_000_000_f64
    } else if unit.starts_with("mib") {
        1024_f64.powi(2)
    } else if unit.starts_with("mb") {
        1_000_000_f64
    } else if unit.starts_with("kib") {
        1024_f64
    } else if unit.starts_with("kb") {
        1_000_f64
    } else {
        1_f64
    };
    Some((number * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_docker_jsonl_and_summary() {
        let report = normalize_docker_evidence(
            &DockerStorageEvidence {
                info_json: Some(r#"{"ServerVersion":"29","DockerRootDir":"/var/lib/docker","Driver":"overlay2"}"#.into()),
                system_df: Some("Images 2 1 10GB 3GB (30%)\nContainers 2 1 4GB 1GB (25%)\nLocal Volumes 2 1 2GB 0B (0%)".into()),
                containers_jsonl: Some(r#"{"ID":"c1","Names":"app","State":"exited","Labels":"com.docker.compose.project=demo","Mounts":"data","Size":"1GB (virtual 2GB)"}"#.into()),
                images_jsonl: Some(r#"{"ID":"sha256:i1","Repository":"app","Tag":"latest","Size":"2GB","Containers":"1"}"#.into()),
                volumes_jsonl: Some(r#"{"Name":"data","Driver":"local","Links":"0","Labels":"com.docker.volume.anonymous="}"#.into()),
                failures: vec![],
            },
            1,
        );
        assert!(report.daemon.reachable);
        assert_eq!(report.images.len(), 1);
        assert_eq!(report.containers[0].project.as_deref(), Some("demo"));
        assert_eq!(report.image_bytes, Some(10_000_000_000));
        assert_eq!(report.reclaimable_bytes, Some(4_000_000_000));
    }
}
