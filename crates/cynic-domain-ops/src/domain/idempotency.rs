//! Pure idempotency key generation for domain operations.
//!
//! Every write operation (register, NS update, zone migration) carries a
//! deterministic idempotency key derived from (domain, operation, content).
//! This prevents duplicate operations at the provider API level — if the
//! same key was already used, the provider returns the prior result.
//!
//! Canonical hashing is SHA3-512 per foundation.json, with the `sha3-512:`
//! prefix.

use sha3::{Digest, Sha3_512};

use cynic_protocol::domain_ops::DomainOperation;

/// Generate a deterministic idempotency key for a domain operation.
///
/// The key is `sha3-512:<hex>` where the hash input is:
///   domain || \x00 || operation || \x00 || content
///
/// This is pure and deterministic: the same inputs always produce the same
/// key, enabling safe retries without duplicate side effects.
pub fn generate_idempotency_key(domain: &str, operation: DomainOperation, content: &str) -> String {
    let mut hasher = Sha3_512::new();
    hasher.update(domain.as_bytes());
    hasher.update(b"\x00");
    hasher.update(operation_as_str(operation).as_bytes());
    hasher.update(b"\x00");
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha3-512:{}", hex::encode(result))
}

/// Pure string representation of a DomainOperation for key generation.
fn operation_as_str(op: DomainOperation) -> &'static str {
    match op {
        DomainOperation::CheckAvailability => "check_availability",
        DomainOperation::Register => "register",
        DomainOperation::UpdateNameservers => "update_nameservers",
        DomainOperation::MigrateZone => "migrate_zone",
        DomainOperation::VerifyDelegation => "verify_delegation",
        DomainOperation::VerifyEdge => "verify_edge",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::domain_ops::DomainOperation;

    #[test]
    fn idempotency_key_is_deterministic() {
        let key1 = generate_idempotency_key(
            "example.com",
            DomainOperation::Register,
            "years=1:auto_renew=true",
        );
        let key2 = generate_idempotency_key(
            "example.com",
            DomainOperation::Register,
            "years=1:auto_renew=true",
        );
        assert_eq!(key1, key2);
    }

    #[test]
    fn different_domains_produce_different_keys() {
        let key1 = generate_idempotency_key("a.com", DomainOperation::Register, "content");
        let key2 = generate_idempotency_key("b.com", DomainOperation::Register, "content");
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_operations_produce_different_keys() {
        let key1 = generate_idempotency_key("example.com", DomainOperation::Register, "content");
        let key2 =
            generate_idempotency_key("example.com", DomainOperation::UpdateNameservers, "content");
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_content_produces_different_keys() {
        let key1 = generate_idempotency_key("example.com", DomainOperation::Register, "a");
        let key2 = generate_idempotency_key("example.com", DomainOperation::Register, "b");
        assert_ne!(key1, key2);
    }

    #[test]
    fn key_uses_sha3_512_prefix() {
        let key = generate_idempotency_key("example.com", DomainOperation::Register, "content");
        assert!(key.starts_with("sha3-512:"));
        let hex_part = &key["sha3-512:".len()..];
        assert_eq!(hex_part.len(), 128); // SHA3-512 = 512 bits = 64 bytes = 128 hex chars
    }

    #[test]
    fn empty_content_still_produces_valid_key() {
        let key = generate_idempotency_key("example.com", DomainOperation::VerifyEdge, "");
        assert!(key.starts_with("sha3-512:"));
        assert_eq!(key["sha3-512:".len()..].len(), 128);
    }
}
