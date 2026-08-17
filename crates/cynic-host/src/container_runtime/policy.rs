use super::types::{DockerContainerObservation, DockerStorageReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerRetentionClass {
    ProtectedActive,
    ProtectedPersistent,
    ReconstructibleCandidate,
    HumanReviewRequired,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerRetentionAssessment {
    pub object_kind: String,
    pub object_id: String,
    pub class: DockerRetentionClass,
    pub reclaimable_bytes: Option<u64>,
    pub reasons: Vec<String>,
    pub mutation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerRetentionReport {
    pub schema: String,
    pub daemon_reachable: bool,
    pub assessments: Vec<DockerRetentionAssessment>,
    pub residual_unknowns: Vec<String>,
    pub mutation_authorized: bool,
}

pub fn assess_retention(report: &DockerStorageReport) -> DockerRetentionReport {
    let mut assessments = Vec::new();
    let mut unknowns = report.residual_unknowns.clone();

    for container in &report.containers {
        assessments.push(assess_container(container));
    }
    for image in &report.images {
        let (class, reasons) = if image.container_refs > 0 {
            (
                DockerRetentionClass::ProtectedPersistent,
                vec![format!(
                    "referenced by {} container(s)",
                    image.container_refs
                )],
            )
        } else if image.reconstructible == Some(true) {
            (
                DockerRetentionClass::ReconstructibleCandidate,
                vec!["no container reference and reconstructible provenance observed".into()],
            )
        } else {
            unknowns.push(format!(
                "image {} has no reconstructibility proof",
                image.id
            ));
            (
                DockerRetentionClass::HumanReviewRequired,
                vec!["no container reference, but reconstructibility is unproven".into()],
            )
        };
        assessments.push(DockerRetentionAssessment {
            object_kind: "image".into(),
            object_id: image.id.clone(),
            class,
            reclaimable_bytes: image.unique_size_bytes,
            reasons,
            mutation_authorized: false,
        });
    }
    for volume in &report.volumes {
        let (class, reason) = if volume.active_links > 0 {
            (
                DockerRetentionClass::ProtectedPersistent,
                format!("linked by {} container(s)", volume.active_links),
            )
        } else {
            (
                DockerRetentionClass::HumanReviewRequired,
                "volume is unlinked but persistence intent is not proven".into(),
            )
        };
        assessments.push(DockerRetentionAssessment {
            object_kind: "volume".into(),
            object_id: volume.name.clone(),
            class,
            reclaimable_bytes: volume.size_bytes,
            reasons: vec![reason],
            mutation_authorized: false,
        });
    }

    DockerRetentionReport {
        schema: "cynic.host.docker_retention_report.v1".into(),
        daemon_reachable: report.daemon.reachable,
        assessments,
        residual_unknowns: unknowns,
        mutation_authorized: false,
    }
}

fn assess_container(container: &DockerContainerObservation) -> DockerRetentionAssessment {
    let state = container.state.to_ascii_lowercase();
    let active = matches!(state.as_str(), "running" | "restarting" | "paused");
    let (class, reasons) = if active {
        (
            DockerRetentionClass::ProtectedActive,
            vec![format!("container state is {}", container.state)],
        )
    } else if !container.volume_refs.is_empty() || container.project.is_some() {
        (
            DockerRetentionClass::ProtectedPersistent,
            vec!["stopped container retains project or volume relationship".into()],
        )
    } else {
        (
            DockerRetentionClass::HumanReviewRequired,
            vec!["stopped container has no complete ownership proof".into()],
        )
    };
    DockerRetentionAssessment {
        object_kind: "container".into(),
        object_id: container.id.clone(),
        class,
        reclaimable_bytes: container
            .writable_layer_bytes
            .zip(container.log_bytes)
            .map(|(layer, logs)| layer.saturating_add(logs)),
        reasons,
        mutation_authorized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_runtime::types::{DockerDaemonObservation, DockerRuntimeKind};

    fn report() -> DockerStorageReport {
        DockerStorageReport {
            schema: "fixture".into(),
            observed_at_unix_ms: 1,
            daemon: DockerDaemonObservation {
                reachable: true,
                runtime: DockerRuntimeKind::Docker,
                server_version: None,
                root_dir: None,
                storage_driver: None,
                snapshotter: None,
            },
            images: vec![],
            containers: vec![DockerContainerObservation {
                id: "stopped".into(),
                name: "stopped".into(),
                image_id: None,
                state: "exited".into(),
                project: None,
                writable_layer_bytes: Some(10),
                log_bytes: Some(2),
                volume_refs: vec![],
                created_at_unix_ms: None,
            }],
            volumes: vec![],
            image_bytes: None,
            container_bytes: None,
            volume_bytes: None,
            reclaimable_bytes: None,
            residual_unknowns: vec![],
        }
    }

    #[test]
    fn stopped_container_never_authorizes_deletion() {
        let retention = assess_retention(&report());
        assert_eq!(
            retention.assessments[0].class,
            DockerRetentionClass::HumanReviewRequired
        );
        assert!(!retention.assessments[0].mutation_authorized);
        assert!(!retention.mutation_authorized);
    }
}
