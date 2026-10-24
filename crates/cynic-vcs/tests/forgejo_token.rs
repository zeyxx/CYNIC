//! Unit tests for Forgejo token operations.
use cynic_vcs::forgejo_token::*;
use cynic_vcs::providers::forgejo_token::*;
use cynic_vcs::providers::forgejo_token::api::*;


    #[test]
    fn observe_uses_path_from_spec() {
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = observe_forgejo_token(&spec);
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Unknown);
        assert_eq!(obs.path, "/tmp/nonexistent-token-xyz");
    }

    #[test]
    fn provision_skips_when_token_already_valid() {
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                let mut obs = observe_forgejo_token(spec);
                obs.api_authenticated = ForgejoTokenAuthState::Proven;
                obs
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("should not be called".into())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("should not be called".into())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: forgejo_token_path().display().to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = provision_forgejo_token(&spec, &StubPort);
        assert_eq!(report.status, "ready");
        assert_eq!(report.action_taken, "none");
        assert!(report.metadata.is_none());
    }

    #[test]
    fn concrete_adapters_fail_closed_without_installed_identity_binding() {
        let spec = ForgejoTokenSpec::default();
        assert!(LocalForgejoTokenApiAdapter.current_token(&spec).is_err());
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.example".to_string(),
            principal: spec.principal.clone(),
            authorization_token: None,
        };
        assert!(http.current_token(&spec).is_err());
    }

    #[test]
    fn http_adapter_absent_when_file_missing() {
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.service.cynic/api/v1".to_string(),
            principal: "cynic-cortex".to_string(),
            authorization_token: None,
        };
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-http-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = http.observe(&spec);
        // File missing → secret_present false → ApiAuthenticated::Absent (no API call).
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Absent);
    }

    #[test]
    fn http_mutations_require_auth_and_normalize_api_base() {
        assert!(required_authorization(None).is_err());
        assert_eq!(
            format_api_url("https://forgejo.example/api/v1/", "/user"),
            "https://forgejo.example/api/v1/user"
        );
    }

    #[test]
    fn secret_writer_rejects_noncanonical_targets_before_io() {
        let error = write_protected_secret("/tmp/arbitrary-token", "secret", 0o640)
            .expect_err("arbitrary token paths must be rejected");
        assert!(error.contains("canonical path"));
    }

    #[test]
    fn noncanonical_paths_blocked_before_port_interaction() {
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("must not create".to_string())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("must not remove".to_string())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: "/tmp/noncanonical-token".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let provision = provision_forgejo_token(&spec, &StubPort);
        let rotation = rotate_forgejo_token(&spec, &StubPort);
        assert_eq!(provision.status, "blocked");
        assert_eq!(provision.action_taken, "none");
        assert_eq!(rotation.status, "blocked");
        assert_eq!(rotation.action_taken, "none");
    }

    #[test]
    fn api_material_requires_metadata_before_secret_write() {
        let response = serde_json::json!({"token": "raw-secret"});
        let error = extract_api_token_material(&response).expect_err("metadata is required first");
        assert!(error.contains("missing exact id"));
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_before_remove() {
        #[derive(Debug)]
        struct InvalidIdentityPort;
        impl ForgejoTokenApiPort for InvalidIdentityPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                panic!("create must not run")
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                panic!("remove must not run for an invalid identity")
            }
            fn current_token(
                &self,
                _spec: &ForgejoTokenSpec,
            ) -> Result<ForgejoTokenMetadata, String> {
                Ok(ForgejoTokenMetadata {
                    token_id: 1,
                    name: "cynic".to_string(),
                    sha1: "<unavailable>".to_string(),
                })
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: forgejo_token_path().display().to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = rotate_forgejo_token(&spec, &InvalidIdentityPort);
        assert_eq!(report.status, "blocked");
        assert_eq!(report.action_taken, "none");
        assert_eq!(
            report.unknowns[0],
            "forgejo_token_current_metadata_unavailable"
        );
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_and_non_hex() {
        assert!(validate_token_identity("").is_err());
        assert!(validate_token_identity("<unavailable>").is_err());
        assert!(validate_token_identity(&"a".repeat(39)).is_err());
        assert!(validate_token_identity(&"g".repeat(40)).is_err());
        assert!(validate_token_identity(&"a".repeat(40)).is_ok());
    }

    #[test]
    fn runtime_default_is_unavailable() {
        let runtime = ForgejoTokenRuntimeSpec::default();
        let observation = observe_forgejo_token_runtime(&runtime);
        assert!(
            observation
                .unknowns
                .iter()
                .any(|unknown| unknown == "forgejo_token_adapter_selection_unproven")
        );
        assert!(runtime.adapter.create(&runtime.token_spec).is_err());
    }

    #[test]
    fn default_spec_uses_canonical_path() {
        let spec = DefaultForgejoTokenSpec::with_token_path("/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.target_path, "/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.principal, "cynic-cortex");
        assert_eq!(spec.mode, 0o640);
    }

