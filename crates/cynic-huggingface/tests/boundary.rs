use cynic_huggingface::{
    ExternalExportReport, HfDatasetSpec, HfEndpointSpec, HfJobHandle, HfJobInfo, HfJobKind,
    HfJobSpec, HfJobStatus, HfRepoType, HfRepositoryRef, HfScheduledJobRequest, HfSpaceSpec,
    HfWebhookSpec, ProjectionReadiness, map_endpoint_state, map_space_stage, map_stage_str,
};

#[test]
fn repository_identity_requires_identity() {
    let repo = HfRepositoryRef::validated("acme", "ops", Some("main".into())).unwrap();
    assert_eq!(repo.slug(), "acme/ops");

    assert!(HfRepositoryRef::validated("", "ops", None).is_err());
    assert!(HfRepositoryRef::validated("acme", "", None).is_err());
    assert!(HfRepositoryRef::validated("acme", "ops", Some("".into())).is_err());
}

#[test]
fn external_export_requires_projection_redaction() {
    let unredacted = ExternalExportReport {
        source_content_hash: "source".into(),
        payload_hash: "payload".into(),
        manifest_path: "manifest.json".into(),
        redacted_at_projection_layer: false,
        bytes: 12,
    };
    assert!(!unredacted.publisher_ready());

    let redacted = ExternalExportReport {
        redacted_at_projection_layer: true,
        ..unredacted
    };
    assert!(redacted.publisher_ready());
}

#[test]
fn job_handle_remains_typed_and_stable() {
    let spec = HfJobHandle {
        idempotency_key: "key".into(),
        remote_id: "job-1".into(),
        observed_status: HfJobStatus::Queued,
    };
    assert_eq!(spec.observed_status, HfJobStatus::Queued);
    assert_eq!(spec.idempotency_key, "key");
}

#[test]
fn dataset_projection_readiness_is_binary() {
    assert!(
        ProjectionReadiness {
            has_redaction: true,
            has_manifest: true,
            canonical_bytes: 1,
        }
        .export_ready()
    );
    assert!(
        !ProjectionReadiness {
            has_redaction: true,
            has_manifest: false,
            canonical_bytes: 1,
        }
        .export_ready()
    );
}

#[test]
fn dataset_spec_names_kind() {
    let spec = HfDatasetSpec {
        repo: "acme/ops".into(),
        subset: None,
        revision: None,
        format_version: 1,
    };
    assert_eq!(spec.canonical_repo_kind(), "dataset");
}

#[test]
fn repo_type_maps_to_cli_arg() {
    assert_eq!(HfRepoType::Model.as_arg(), "model");
    assert_eq!(HfRepoType::Dataset.as_arg(), "dataset");
    assert_eq!(HfRepoType::Space.as_arg(), "space");
}

#[test]
fn job_stage_mapping_is_total_and_never_fabricates_health() {
    assert_eq!(map_stage_str("queued"), HfJobStatus::Queued);
    assert_eq!(map_stage_str("pending"), HfJobStatus::Queued);
    assert_eq!(map_stage_str("RUNNING"), HfJobStatus::Running);
    assert_eq!(map_stage_str("succeeded"), HfJobStatus::Completed);
    assert_eq!(map_stage_str("failed"), HfJobStatus::Failed);
    assert_eq!(map_stage_str("cancelled"), HfJobStatus::Cancelled);
    // Unknown stages surface as Unknown — never a healthy value.
    assert_eq!(map_stage_str("mystery-stage"), HfJobStatus::Unknown);
    assert_eq!(map_stage_str(""), HfJobStatus::Unknown);
}

#[test]
fn job_info_keeps_raw_payload_and_unknown_stage() {
    let raw = serde_json::json!({
        "id": "job-9",
        "stage": "weird-future-stage",
        "flavor": "a10g-small",
    });
    let info = HfJobInfo::from_raw(&raw).unwrap();
    assert_eq!(info.remote_id, "job-9");
    assert_eq!(info.status, HfJobStatus::Unknown);
    assert_eq!(info.stage_raw, "weird-future-stage");
    assert_eq!(info.flavor.as_deref(), Some("a10g-small"));
    assert_eq!(info.raw, raw);

    assert!(HfJobInfo::from_raw(&serde_json::json!({"no-id": true})).is_none());
}

#[test]
fn job_spec_requires_payload() {
    assert!(HfJobSpec::uv_script("", vec![]).is_err());
    let spec = HfJobSpec::uv_script("train.py", vec!["--epochs".into(), "3".into()]).unwrap();
    assert!(matches!(spec.kind, HfJobKind::UvScript { .. }));
    assert!(spec.detach);

    assert!(HfScheduledJobRequest::validated("", spec.clone()).is_err());
    let sched = HfScheduledJobRequest::validated("@hourly", spec).unwrap();
    assert_eq!(sched.schedule, "@hourly");
}

#[test]
fn staged_contracts_validate_their_invariants() {
    assert!(HfSpaceSpec::validated("", None).is_err());
    assert!(HfSpaceSpec::validated("acme/demo", Some("gradio".into())).is_ok());

    assert!(HfWebhookSpec::validated(None, None).is_err());
    assert!(HfWebhookSpec::validated(Some("https://x".into()), Some("job-1".into())).is_err());
    assert!(HfWebhookSpec::validated(Some("https://x".into()), None).is_ok());

    assert!(HfEndpointSpec::validated("", "acme/m").is_err());
    assert!(HfEndpointSpec::validated("ep-1", "acme/m").is_ok());

    assert_eq!(
        map_space_stage("paused"),
        cynic_huggingface::HfSpaceStatus::Paused
    );
    assert_eq!(
        map_space_stage("???"),
        cynic_huggingface::HfSpaceStatus::Unknown
    );
    assert_eq!(
        map_endpoint_state("scaled-to-zero"),
        cynic_huggingface::HfEndpointStatus::ScaledToZero
    );
    assert_eq!(
        map_endpoint_state("???"),
        cynic_huggingface::HfEndpointStatus::Unknown
    );
}
