//! Canonical file-backed 32-byte hex key loader (load path only).
//!
//! Single home since 2026-09-28 for the loading half of key material (M1).
//! Fuses three implementations that each parsed "hex file -> [u8; 32]" with
//! different permission gates and message wordings:
//! `FileEnrollmentSigner::load` (owner-only gate), `read_hex_32` (no gate,
//! two labels), and the read branch of `bin/enroll_grant.rs`
//! `read_or_create_hex_signing_key` (no gate, path-embedded messages).
//!
//! Message template is byte-identical to the former signer strings with
//! `label = "enrollment signing key"`, so the signer's pinning tests pass
//! unchanged. Reader/bin wordings shift slightly (no tests pinned them);
//! the authoritative behaviors (gate bit `0o027`, hex, 32 bytes) are fused
//! with tests below.
//!
//! Key GENESIS (creation via /dev/urandom) stays in `bin/enroll_grant.rs`:
//! creation is a different causal link, already single-homed. Known gap,
//! not changed here: created key files get no permission hardening —
//! flagged to the bin owner as follow-up.

use std::path::Path;

/// Load 32 bytes of hex key material from `path`.
///
/// * `label` names the key in errors (e.g. `"enrollment signing key"`).
/// * `enforce_owner_only` applies the unix permission gate (no group/other
///   access); disabled on non-unix targets.
pub fn read_hex_key_bytes(
    path: &Path,
    label: &str,
    enforce_owner_only: bool,
) -> Result<[u8; 32], String> {
    if enforce_owner_only {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mode = std::fs::metadata(path)
                .map_err(|error| format!("inspect {label}: {error}"))?
                .mode()
                & 0o777;
            if mode & 0o027 != 0 {
                return Err(format!(
                    "{label} must not be group-writable or accessible by others"
                ));
            }
        }
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    hex::decode(raw.trim())
        .map_err(|_| format!("{label} must be hex"))?
        .try_into()
        .map_err(|_| format!("{label} must contain 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn restrict_to_owner(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    #[cfg(not(unix))]
    fn restrict_to_owner(_path: &Path) {}

    #[test]
    fn rejects_non_hex_and_short_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("test.key");
        std::fs::write(&key_path, "not-hex!!").expect("write");
        restrict_to_owner(&key_path);
        assert_eq!(
            read_hex_key_bytes(&key_path, "test key", false).unwrap_err(),
            "test key must be hex".to_string()
        );
        std::fs::write(&key_path, "ab".repeat(8)).expect("write");
        assert_eq!(
            read_hex_key_bytes(&key_path, "test key", false).unwrap_err(),
            "test key must contain 32 bytes".to_string()
        );
        std::fs::write(&key_path, "07".repeat(32)).expect("write");
        assert_eq!(
            read_hex_key_bytes(&key_path, "test key", false).expect("load"),
            [0x07u8; 32]
        );
    }

    #[cfg(unix)]
    #[test]
    fn gate_rejects_group_writable_keys() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("test.key");
        std::fs::write(&key_path, "07".repeat(32)).expect("write");
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        assert_eq!(
            read_hex_key_bytes(&key_path, "test key", true).unwrap_err(),
            "test key must not be group-writable or accessible by others".to_string()
        );
        assert!(
            read_hex_key_bytes(&key_path, "test key", false).is_ok(),
            "disabled gate must not inspect permissions"
        );
    }
}
