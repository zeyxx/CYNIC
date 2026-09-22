/// Architecture boundary tests for cynic-agent-bus.
///
/// Enforces:
/// - domain/ types are 100% pure (no std, no tokio, no reqwest)
/// - Ports are traits only (no `impl Foo for Bar`)
/// - Concrete impls live outside the crate (or in tests/)
/// - No forbidden tokens in domain modules

use std::fs;
use std::path::Path;

fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn assert_no_tokens(path: &Path, forbidden: &[&str], context: &str) {
    let content = read_file(path);
    for token in forbidden {
        if content.contains(token) {
            panic!(
                "ARCH VIOLATION [{}]: {} contains forbidden token `{}`",
                context,
                path.display(),
                token
            );
        }
    }
}

#[test]
fn types_are_pure_domain_no_std() {
    let types = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs");
    assert_no_tokens(
        &types,
        &[
            "use std::",
            "tokio::",
            "reqwest::",
            "crate::infra::",
            "crate::adapters::",
        ],
        "domain purity",
    );
}

#[test]
fn claim_store_allowed_std_collections() {
    let claim = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/claim.rs");
    // claim.rs is the store impl, it may use std::collections.
    // What matters: it doesn't reach outside the crate boundary.
    assert_no_tokens(
        &claim,
        &[
            "reqwest::",
            "crate::infra::",
            "crate::adapters::",
        ],
        "domain purity",
    );
}

#[test]
fn ledger_allowed_std_collections() {
    let ledger = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ledger.rs");
    assert_no_tokens(
        &ledger,
        &[
            "reqwest::",
            "crate::infra::",
            "crate::adapters::",
        ],
        "domain purity",
    );
}

#[test]
fn ports_are_traits_only() {
    let ports = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ports.rs");
    let content = read_file(&ports);
    // Ports may impl std traits (Display, Error) but must not impl port traits
    let port_trait_names = ["AgentExecutor", "IdentityProvider", "CoordinatorReasoner"];
    for line in content.lines() {
        if line.contains("impl") {
            let is_port_impl = port_trait_names.iter().any(|t| line.contains(t));
            assert!(
                !is_port_impl,
                "ports.rs must not contain impl for port trait: {}",
                line.trim()
            );
        }
    }
}

#[test]
fn domain_types_are_pure_structs_and_enums() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    // types.rs must NOT contain async_trait, tokio, or external IO
    assert!(
        !types.contains("#[async_trait"),
        "types.rs must not contain async traits"
    );
    assert!(
        !types.contains("tokio::"),
        "types.rs must not import tokio"
    );
    // impl blocks for inherent methods are fine (e.g. impl AgentId { fn from_ed25519 })
}

#[test]
fn taskkind_has_three_variants() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    assert!(types.contains("Solo"), "TaskKind must have Solo variant");
    assert!(types.contains("Chain"), "TaskKind must have Chain variant");
    assert!(types.contains("Fanout"), "TaskKind must have Fanout variant");
    assert!(
        !types.contains("SoloFile"),
        "SoloFile should have been renamed to Solo"
    );
    assert!(
        !types.contains("SoloCrate"),
        "SoloCrate should have been cut"
    );
}

#[test]
fn proof_has_no_signature_field() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    assert!(
        !types.contains("signature:"),
        "Proof must not carry a Signature field (security theater)"
    );
    assert!(
        !types.contains("struct Signature"),
        "Signature struct must be removed"
    );
}

#[test]
fn budget_has_both_axes() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    assert!(
        types.contains("actions: u32"),
        "Budget must have actions field"
    );
    assert!(
        types.contains("wall_seconds: u32"),
        "Budget must have wall_seconds field"
    );
}

#[test]
fn scopekind_has_three_variants() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    assert!(types.contains("File"), "ScopeKind must have File");
    assert!(types.contains("Module"), "ScopeKind must have Module");
    assert!(types.contains("Crate"), "ScopeKind must have Crate");
}

#[test]
fn claim_error_includes_invalid_proof() {
    let types = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types.rs"));
    assert!(
        types.contains("InvalidProof"),
        "ClaimError must have InvalidProof variant"
    );
}

#[test]
fn claimstore_trait_has_expiry_methods() {
    let claim = read_file(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/claim.rs"));
    assert!(
        claim.contains("async fn expire_claims"),
        "ClaimStore must have expire_claims method"
    );
    assert!(
        claim.contains("async fn is_expired"),
        "ClaimStore must have is_expired method"
    );
}
