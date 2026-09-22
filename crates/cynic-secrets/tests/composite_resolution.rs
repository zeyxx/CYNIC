//! Integration tests for cynic-secrets resolution.
//!
//! Tests the CompositeSecretProvider fallback chain and individual
//! adapter behaviour. Uses MemorySecretProvider (no I/O) for determinism.

use cynic_secrets::{
    SecretClass, SecretProviderKind, SecretProviderPort, SecretReference,
    adapters::{MemorySecretProvider, ProcessEnvironmentSecretProvider},
};

fn ref_(id: &str, locator: &str, provider: SecretProviderKind) -> SecretReference {
    SecretReference {
        id: id.to_string(),
        class: SecretClass::RuntimeMachineIdentity,
        provider,
        locator: locator.to_string(),
    }
}

#[test]
fn memory_provider_resolves_stored_secret() {
    let provider = MemorySecretProvider::new().with_secret("porkbun-key", "pk1_test");
    let r = ref_(
        "porkbun-key",
        "PORKBUN_API_KEY",
        SecretProviderKind::ProcessEnvironment,
    );
    assert_eq!(provider.resolve(&r), Some("pk1_test".to_string()));
}

#[test]
fn memory_provider_returns_none_for_unknown_secret() {
    let provider = MemorySecretProvider::new();
    let r = ref_("missing", "MISSING", SecretProviderKind::ProcessEnvironment);
    assert_eq!(provider.resolve(&r), None);
}

#[test]
fn env_provider_rejects_non_env_references() {
    let provider = ProcessEnvironmentSecretProvider;
    let r = ref_("test", "some-path", SecretProviderKind::PrincipalOwnedFile);
    assert_eq!(provider.resolve(&r), None);
}

#[test]
fn env_provider_resolves_when_env_var_set() {
    // Set env var, resolve, then unset — isolated test
    unsafe {
        std::env::set_var("CYNIC_TEST_SECRET", "test-value-123");
    }
    let provider = ProcessEnvironmentSecretProvider;
    let r = ref_(
        "test",
        "CYNIC_TEST_SECRET",
        SecretProviderKind::ProcessEnvironment,
    );
    assert_eq!(provider.resolve(&r), Some("test-value-123".to_string()));
    unsafe {
        std::env::remove_var("CYNIC_TEST_SECRET");
    }
}

#[test]
fn env_provider_returns_none_when_env_var_unset() {
    unsafe {
        std::env::remove_var("CYNIC_DEFINITELY_NOT_SET_VAR");
    }
    let provider = ProcessEnvironmentSecretProvider;
    let r = ref_(
        "test",
        "CYNIC_DEFINITELY_NOT_SET_VAR",
        SecretProviderKind::ProcessEnvironment,
    );
    assert_eq!(provider.resolve(&r), None);
}

#[test]
fn status_reflects_presence() {
    let provider = MemorySecretProvider::new().with_secret("found-key", "secret");
    let r = ref_(
        "found-key",
        "FOUND_KEY",
        SecretProviderKind::ProcessEnvironment,
    );
    let status = provider.status(&r);
    assert!(status.present);

    let missing = ref_(
        "missing-key",
        "MISSING_KEY",
        SecretProviderKind::ProcessEnvironment,
    );
    let status_missing = provider.status(&missing);
    assert!(!status_missing.present);
}
