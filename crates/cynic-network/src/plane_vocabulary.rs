//! Canonical plane vocabulary — the single translation point for network plane names.
//!
//! ## Why this exists
//!
//! The workspace carried two vocabularies for the same planes:
//!
//! | Canonical (`cynic_protocol::Plane`) | Legacy registry keys |
//! |-------------------------------------|----------------------|
//! | `service`                           | `service`            |
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

/// Read `plane` from a JSON object, accepting canonical and legacy keys.
///
/// This is the read path every registry consumer must use. Before it existed,
/// each crate re-implemented its own `.get("service").or_else(|| .get("synapse"))`
/// chain — or, worse, read the legacy key alone. Those chains drifted and then
/// broke silently when the registry was migrated: a legacy-only reader against
/// a canonical registry resolves to `None` and the node vanishes with no error.
///
/// Lookup tries the canonical key first, then the legacy aliases, then the
/// underlay key. Returns `None` only when no plane declares an address.
pub fn plane_address<'a>(value: &'a serde_json::Value, plane: &str) -> Option<&'a str> {
    let object = value.as_object()?;
    plane_address_in(object, plane)
}

/// [`plane_address`] over a borrowed JSON object.
pub fn plane_address_in<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    plane: &str,
) -> Option<&'a str> {
    let canonical = canonical_plane_key(plane);
    let mut candidates = Vec::with_capacity(2);
    candidates.push(canonical.to_string());
    for (legacy, target) in LEGACY_PLANE_ALIASES {
        if target == canonical {
            candidates.push(legacy.to_string());
        }
    }
    if plane == UNDERLAY_KEY {
        candidates.push(UNDERLAY_KEY.to_string());
    }
    candidates
        .iter()
        .filter_map(|key| object.get(key))
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .find(|address| !address.is_empty())
}

/// Read the underlay / L2-adjacency address of a node network object.
///
/// `local` is not a plane (spec §5.1), so it is read directly and never
/// through the plane alias map.
pub fn underlay_address(value: &serde_json::Value) -> Option<&str> {
    value
        .as_object()?
        .get(UNDERLAY_KEY)?
        .as_str()
        .map(str::trim)
        .filter(|address| !address.is_empty())
}

/// `serde_json::Value`. This is the same resolution order as [`plane_address`].
pub fn plane_address_in_map<'a, K, V>(
    addresses: &'a std::collections::BTreeMap<K, V>,
    plane: &str,
) -> Option<&'a str>
where
    K: AsRef<str> + Ord,
    V: AsRef<str>,
{
    let canonical = canonical_plane_key(plane);
    addresses
        .iter()
        .find(|(key, _)| canonical_plane_key(key.as_ref()) == canonical)
        .map(|(_, value)| value.as_ref().trim())
        .filter(|address| !address.is_empty())
}

/// Read the underlay / L2-adjacency address from a `BTreeMap`-shaped table.
///
/// `local` is not a plane (spec §5.1), so it is read directly and never
/// through the plane alias map.
pub fn underlay_address_in_map<'a, K, V>(
    addresses: &'a std::collections::BTreeMap<K, V>,
) -> Option<&'a str>
where
    K: AsRef<str> + Ord,
    V: AsRef<str>,
{
    addresses
        .iter()
        .find(|(key, _)| key.as_ref() == UNDERLAY_KEY)
        .map(|(_, value)| value.as_ref().trim())
        .filter(|address| !address.is_empty())
}

/// [`plane_address_in_map`] across the canonical plane order, then underlay.
pub fn best_plane_address_in_map<'a, K, V>(
    addresses: &'a std::collections::BTreeMap<K, V>,
) -> Option<&'a str>
where
    K: AsRef<str> + Ord,
    V: AsRef<str>,
{
    for plane in CANONICAL_PLANE_KEYS {
        if let Some(address) = plane_address_in_map(addresses, plane) {
            return Some(address);
        }
    }
    addresses
        .iter()
        .find(|(key, _)| key.as_ref() == UNDERLAY_KEY)
        .map(|(_, value)| value.as_ref().trim())
        .filter(|address| !address.is_empty())
}

/// Address on the first plane that declares one, canonical order, then underlay.
///
/// Mirrors the resolution order of `RegistryReservation::best_address` and
/// `RegistryNodeNetwork::best_ip`, so a `serde_json` consumer and a typed
/// consumer of the same registry agree on which address wins.
pub fn best_plane_address(value: &serde_json::Value) -> Option<&str> {
    for plane in CANONICAL_PLANE_KEYS {
        if let Some(address) = plane_address(value, plane) {
            return Some(address);
        }
    }
    underlay_address(value)
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

    fn canonical_registry_node() -> serde_json::Value {
        serde_json::json!({
            "service": "10.10.10.1",
            "admin": "192.168.100.1",
            "mesh": "100.64.0.1",
            "local": "192.168.0.37"
        })
    }

    fn legacy_registry_node() -> serde_json::Value {
        serde_json::json!({
            "synapse": "10.10.10.1",
            "management": "192.168.100.1",
            "overlay": "100.64.0.1",
            "local": "192.168.0.37"
        })
    }

    /// The regression this API exists for: a canonical registry read by the
    /// legacy vocabulary must resolve identically, and vice versa. Before
    /// `plane_address` there was no shared path, so a legacy-only reader
    /// against a canonical registry silently resolved to `None`.
    #[test]
    fn plane_address_reads_canonical_and_legacy_equally() {
        for node in [canonical_registry_node(), legacy_registry_node()] {
            assert_eq!(plane_address(&node, "service"), Some("10.10.10.1"));
            assert_eq!(plane_address(&node, "admin"), Some("192.168.100.1"));
            assert_eq!(plane_address(&node, "mesh"), Some("100.64.0.1"));
        }
    }

    #[test]
    fn plane_address_prefers_canonical_when_both_are_present() {
        let node = serde_json::json!({
            "service": "10.10.10.1",
            "synapse": "172.16.0.1"
        });
        assert_eq!(plane_address(&node, "service"), Some("10.10.10.1"));
    }

    #[test]
    fn plane_address_is_none_when_the_plane_is_absent() {
        let node = serde_json::json!({ "admin": "192.168.100.1" });
        assert_eq!(plane_address(&node, "service"), None);
    }

    #[test]
    fn plane_address_ignores_blank_and_non_string_values() {
        let node = serde_json::json!({ "service": "   ", "synapse": "10.10.10.9", "mesh": 42 });
        assert_eq!(plane_address(&node, "service"), Some("10.10.10.9"));
        assert_eq!(plane_address(&node, "mesh"), None);
    }

    #[test]
    fn underlay_is_read_under_its_own_key_not_through_the_plane_map() {
        let node = canonical_registry_node();
        assert_eq!(underlay_address(&node), Some("192.168.0.37"));
        assert_eq!(plane_address(&node, UNDERLAY_KEY), Some("192.168.0.37"));
    }





    /// A malformed address or CIDR is not evidence of membership, so it
    /// returns `false` rather than panicking or admitting the node.

    #[test]
    fn best_plane_address_follows_canonical_order() {
        let node = serde_json::json!({ "mesh": "100.64.0.1", "admin": "192.168.100.1", "service": "10.10.10.1" });
        assert_eq!(best_plane_address(&node), Some("10.10.10.1"));
    }

    #[test]
    fn best_plane_address_falls_through_to_underlay() {
        let node = serde_json::json!({ "local": "192.168.0.37" });
        assert_eq!(best_plane_address(&node), Some("192.168.0.37"));
        assert_eq!(best_plane_address(&serde_json::json!({})), None);
    }

    #[test]
    fn best_plane_address_agrees_across_both_vocabularies() {
        assert_eq!(
            best_plane_address(&canonical_registry_node()),
            best_plane_address(&legacy_registry_node())
        );
    }

    fn canonical_address_map() -> std::collections::BTreeMap<&'static str, &'static str> {
        std::collections::BTreeMap::from([
            ("service", "10.10.10.1"),
            ("admin", "192.168.100.1"),
            ("mesh", "100.64.0.1"),
            ("local", "192.168.0.37"),
        ])
    }

    fn legacy_address_map() -> std::collections::BTreeMap<&'static str, &'static str> {
        std::collections::BTreeMap::from([
            ("synapse", "10.10.10.1"),
            ("management", "192.168.100.1"),
            ("overlay", "100.64.0.1"),
            ("local", "192.168.0.37"),
        ])
    }

    #[test]
    fn plane_address_in_map_reads_canonical_and_legacy_equally() {
        for map in [canonical_address_map(), legacy_address_map()] {
            assert_eq!(plane_address_in_map(&map, "service"), Some("10.10.10.1"));
            assert_eq!(plane_address_in_map(&map, "admin"), Some("192.168.100.1"));
            assert_eq!(plane_address_in_map(&map, "mesh"), Some("100.64.0.1"));
        }
    }

    /// A non-plane key must not be mistaken for a plane. The previous map
    /// variant returned `None` on the first non-matching key, which made it
    /// depend on BTreeMap ordering rather than on the plane being asked for.
    #[test]
    fn plane_address_in_map_ignores_keys_of_other_planes() {
        let map = std::collections::BTreeMap::from([
            ("admin", "192.168.100.1"),
            ("guest", "172.30.41.10"),
            ("service", "10.10.10.1"),
        ]);
        assert_eq!(plane_address_in_map(&map, "service"), Some("10.10.10.1"));
        assert_eq!(plane_address_in_map(&map, "guest"), Some("172.30.41.10"));
        assert_eq!(plane_address_in_map(&map, "mesh"), None);
    }

    #[test]
    fn best_plane_address_in_map_agrees_with_the_json_path() {
        assert_eq!(
            best_plane_address_in_map(&canonical_address_map()),
            Some("10.10.10.1")
        );
        assert_eq!(
            best_plane_address_in_map(&canonical_address_map()),
            best_plane_address(&canonical_registry_node())
        );
        assert_eq!(
            best_plane_address_in_map(&legacy_address_map()),
            best_plane_address(&legacy_registry_node())
        );
    }

    #[test]
    fn best_plane_address_in_map_falls_through_to_underlay() {
        let map = std::collections::BTreeMap::from([("local", "192.168.0.37")]);
        assert_eq!(best_plane_address_in_map(&map), Some("192.168.0.37"));
        let empty: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
        assert_eq!(best_plane_address_in_map(&empty), None);
    }
}

/// Re-exported so a caller that already imports the plane vocabulary has
/// the plane-membership question in the same place.
pub use crate::plane_cidr::address_in_cidr;
