//! Pure idempotency key generation for domain operations.
//!
//! Every write operation (register, NS update, zone migration) carries a
//! deterministic idempotency key derived from (domain, operation, content).
//! This prevents duplicate operations at the provider API level — if the
//! same key was already used, the provider returns the prior result.
//!
//! Canonical hashing is SHA-256 per foundation.json (T13 flip), with the
//! `sha256:` prefix.

use cynic_protocol::domain_ops::DomainOperation;

/// Generate a deterministic idempotency key for a domain operation.
///
/// The key is `sha256:<hex>` where the hash input is:
///   domain || \x00 || operation || \x00 || content
///
/// This is pure and deterministic: the same inputs always produce the same
/// key, enabling safe retries without duplicate side effects.
pub fn generate_idempotency_key(domain: &str, operation: DomainOperation, content: &str) -> String {
    let mut input = Vec::new();
    input.extend_from_slice(domain.as_bytes());
    input.push(0x00);
    input.extend_from_slice(operation_as_str(operation).as_bytes());
    input.push(0x00);
    input.extend_from_slice(content.as_bytes());
    cynic_protocol::digest::digest_bytes(&input)
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
    fn key_uses_sha256_prefix() {
        let key = generate_idempotency_key("example.com", DomainOperation::Register, "content");
        assert!(key.starts_with("sha256:"));
        let hex_part = &key["sha256:".len()..];
        assert_eq!(hex_part.len(), 64); // SHA-256 = 256 bits = 32 bytes = 64 hex chars
    }

    #[test]
    fn empty_content_still_produces_valid_key() {
        let key = generate_idempotency_key("example.com", DomainOperation::VerifyEdge, "");
        assert!(key.starts_with("sha256:"));
        assert_eq!(key["sha256:".len()..].len(), 64);
    }
}
