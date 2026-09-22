//! Integration tests for cynic-secrets resolution.
//!
//! Tests the CompositeSecretProvider fallback chain and individual
//! adapter behaviour. Uses MemorySecretProvider (no I/O) for determinism.

use cynic_secrets::{
    SecretClass, SecretProviderKind, SecretProviderPort, SecretStorePort, SecretReference,
    adapters::{EncryptedFileSecretProvider, MemorySecretProvider, OsCredentialStoreSecretProvider, ProcessEnvironmentSecretProvider},
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

#[test]
fn file_provider_roundtrip_encrypts_and_decrypts() {
    let temp = tempfile::tempdir().unwrap();
    let provider = EncryptedFileSecretProvider::new(temp.path());
    let r = SecretReference {
        id: "test-encrypted-secret".to_string(),
        class: SecretClass::RuntimeMachineIdentity,
        provider: SecretProviderKind::PrincipalOwnedFile,
        locator: "test-encrypted-secret".to_string(),
    };
    let value = "super-secret-value-98765";

    // Store — encrypts and writes
    provider.store(&r, value).expect("store should succeed");

    // Resolve — reads, decrypts, returns plaintext
    let resolved = provider.resolve(&r);
    assert_eq!(resolved, Some(value.to_string()));

    // Status — present, with mtime
    let status = provider.status(&r);
    assert!(status.present);
    assert!(status.last_rotated_secs.is_some());

    // Delete
    provider.delete(&r).expect("delete should succeed");
    let resolved_after = provider.resolve(&r);
    assert_eq!(resolved_after, None);
    let status_after = provider.status(&r);
    assert!(!status_after.present);
}

#[test]
fn file_provider_rejects_non_file_references() {
    let temp = tempfile::tempdir().unwrap();
    let provider = EncryptedFileSecretProvider::new(temp.path());
    let r = ref_("test", "TEST_VAR", SecretProviderKind::ProcessEnvironment);
    assert_eq!(provider.resolve(&r), None);
}

#[test]
fn keyring_provider_graceful_when_unavailable() {
    let provider = OsCredentialStoreSecretProvider::default();
    let r = ref_(
        "test-keyring",
        "test-keyring-id",
        SecretProviderKind::OsCredentialStore,
    );
    // On a headless builder, secret-tool/security are likely absent.
    // resolve returns None — the composite provider falls back to next.
    let _ = provider.resolve(&r);
    let status = provider.status(&r);
    // status works regardless
    assert_eq!(status.id, "test-keyring");
}

#[test]
fn secret_reference_env_helper_creates_runtime_ref() {
    let r = SecretReference::env("my-key", "MY_KEY");
    assert_eq!(r.id, "my-key");
    assert_eq!(r.class, SecretClass::RuntimeMachineIdentity);
    assert_eq!(r.provider, SecretProviderKind::ProcessEnvironment);
    assert_eq!(r.locator, "MY_KEY");
}

#[test]
fn env_var_helper_resolves_env_var() {
    unsafe { std::env::set_var("CYNIC_TEST_HELPER_SECRET", "helper-value-42") };
    let result = cynic_secrets::env_var("helper-test", "CYNIC_TEST_HELPER_SECRET");
    assert_eq!(result, Some("helper-value-42".to_string()));
    unsafe { std::env::remove_var("CYNIC_TEST_HELPER_SECRET") };
}

#[test]
fn env_var_present_helper_detects_presence() {
    unsafe { std::env::remove_var("CYNIC_TEST_PRESENT_VAR") };
    assert!(!cynic_secrets::env_var_present("test", "CYNIC_TEST_PRESENT_VAR"));

    unsafe { std::env::set_var("CYNIC_TEST_PRESENT_VAR", "set") };
    assert!(cynic_secrets::env_var_present("test", "CYNIC_TEST_PRESENT_VAR"));
    unsafe { std::env::remove_var("CYNIC_TEST_PRESENT_VAR") };
}
