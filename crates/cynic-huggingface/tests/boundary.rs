use cynic_huggingface::{
    ExternalExportReport, HfDatasetSpec, HfJobHandle, HfJobStatus, HfRepositoryRef,
    ProjectionReadiness,
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
    assert!(ProjectionReadiness {
        has_redaction: true,
        has_manifest: true,
        canonical_bytes: 1,
    }
    .export_ready());
    assert!(!ProjectionReadiness {
        has_redaction: true,
        has_manifest: false,
        canonical_bytes: 1,
    }
    .export_ready());
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