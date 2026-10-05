//! Canonical plane vocabulary — the single translation point for network plane names.
//!
//! ## Why this exists
//!
//! The workspace carried two vocabularies for the same planes:
//!
//! | Canonical (`cynic_protocol::Plane`) | Legacy registry keys |
//! |-------------------------------------|----------------------|
//! | `service`                           | `synapse`            |
//! | `admin`                             | `management`         |
//! | `mesh`                              | `overlay`            |
//!
//! Every reader crate used to re-implement its own fallback chain
//! (`.get("service").or_else(|| .get("synapse"))`). Those chains drifted and
//! produced silently-empty lookups: the canonical keys `service`/`admin`/`mesh`
//! written in `infra/registry.json` were never declared in the reader structs,
//! so the fields resolved to `None` and hardcoded fallbacks took over.
//!
//! This module is the ONLY place that knows the mapping. Domain code reads
//! canonical names; registry input may use any legacy key.
//!
//! ## What is *not* a plane
//!
//! `local` is the node's own underlay / L2-adjacency address (spec §5.1). It is
//! an address attribute, not an address domain, and is exposed through
//! [`UNDERLAY_KEY`] rather than through the plane alias map.

pub use cynic_protocol::plane::Plane;

/// Canonical plane keys, in canonical order.
pub const CANONICAL_PLANE_KEYS: [&str; 3] = ["service", "admin", "mesh"];

/// Legacy registry key -> canonical plane key.
pub const LEGACY_PLANE_ALIASES: [(&str, &str); 3] = [
    ("synapse", "service"),
    ("management", "admin"),
    ("overlay", "mesh"),
];

/// Registry key holding the node underlay / L2-adjacency address.
pub const UNDERLAY_KEY: &str = "local";

/// Translate any registry plane key into its canonical name.
///
/// Unknown keys pass through unchanged so non-plane keys (e.g.
/// `proxmox_guest`) remain addressable.
pub fn canonical_plane_key(key: &str) -> &str {
    match key {
        "synapse" => "service",
        "management" => "admin",
        "overlay" => "mesh",
        other => other,
    }
}

/// True when `key` is already a canonical plane name.
pub fn is_canonical_plane_key(key: &str) -> bool {
    CANONICAL_PLANE_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_keys_translate_to_canonical() {
        assert_eq!(canonical_plane_key("synapse"), "service");
        assert_eq!(canonical_plane_key("management"), "admin");
        assert_eq!(canonical_plane_key("overlay"), "mesh");
    }

    #[test]
    fn canonical_keys_are_identity() {
        for key in CANONICAL_PLANE_KEYS {
            assert_eq!(canonical_plane_key(key), key);
            assert!(is_canonical_plane_key(key));
        }
    }

    #[test]
    fn non_plane_keys_pass_through() {
        assert_eq!(canonical_plane_key("proxmox_guest"), "proxmox_guest");
        assert!(!is_canonical_plane_key("proxmox_guest"));
        assert!(!is_canonical_plane_key("synapse"));
    }

    #[test]
    fn plane_enum_matches_canonical_keys() {
        assert_eq!(Plane::Service.name(), "service");
        assert_eq!(Plane::Admin.name(), "admin");
        assert_eq!(Plane::Mesh.name(), "mesh");
    }

    #[test]
    fn every_alias_target_is_canonical() {
        for (_, canonical) in LEGACY_PLANE_ALIASES {
            assert!(is_canonical_plane_key(canonical));
        }
    }
}
