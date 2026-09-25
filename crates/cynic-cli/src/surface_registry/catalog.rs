use crate::surface_registry::types::CanonicalCliSurface;
<<<<<<< HEAD
=======
use serde_json::Value;
use std::collections::BTreeSet;

const SURFACE_REGISTRY_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../infra/surface-registry/manifest.json"
));
>>>>>>> github/main

include!(concat!(
    env!("OUT_DIR"),
    "/surface_registry_fragment_index.rs"
));

pub(crate) fn canonical_cli_surfaces() -> Vec<CanonicalCliSurface> {
<<<<<<< HEAD
    SURFACE_REGISTRY_FRAGMENT_INDEX
        .iter()
        .map(|(id, fragment_json)| {
            let surface: CanonicalCliSurface = serde_json::from_str(fragment_json)
                .expect("surface fragment must be a valid CanonicalCliSurface object");
            assert_eq!(surface.id, *id);
            surface
        })
        .collect()
=======
    let manifest = load_surface_registry_manifest();
    validate_surface_registry_manifest(&manifest);
    let mut seen_ids = BTreeSet::new();
    let mut surfaces = Vec::new();

    let entries = manifest
        .get("surfaces")
        .and_then(Value::as_array)
        .expect("manifest.surfaces must be an array");

    for surface_entry in entries {
        let id = surface_entry
            .get("id")
            .and_then(Value::as_str)
            .expect("manifest.surfaces[id] must be a string");
        let fragment_json = fragment_for_surface(id)
            .unwrap_or_else(|| panic!("surface-registry fragment missing for manifest id '{id}'"));
        let surface: CanonicalCliSurface = serde_json::from_str(fragment_json)
            .expect("surface fragment must be a valid CanonicalCliSurface object");
        assert_eq!(surface.id, id);
        assert!(
            seen_ids.insert(id.to_string()),
            "duplicate surface id in manifest"
        );
        surfaces.push(surface);
    }
    surfaces
}

fn load_surface_registry_manifest() -> Value {
    serde_json::from_str(SURFACE_REGISTRY_MANIFEST)
        .expect("infra/surface-registry/manifest.json must be valid JSON")
}

fn validate_surface_registry_manifest(manifest: &Value) {
    assert_eq!(
        manifest
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        cynic_protocol::SURFACE_REGISTRY_SCHEMA,
        "infra/surface-registry/manifest.json must declare the canonical surface registry schema"
    );
    assert!(
        manifest.get("surfaces").is_some_and(Value::is_array),
        "infra/surface-registry/manifest.json must contain an array of surface fragments"
    );
}

fn fragment_for_surface(surface_id: &str) -> Option<&'static str> {
    SURFACE_REGISTRY_FRAGMENT_INDEX
        .iter()
        .find_map(|(id, json)| (*id == surface_id).then_some(*json))
>>>>>>> github/main
}

#[cfg(test)]
mod tests {
<<<<<<< HEAD
    use super::SURFACE_REGISTRY_FRAGMENT_INDEX;

    #[test]
    fn generated_fragment_index_contains_embedded_json() {
        assert!(!SURFACE_REGISTRY_FRAGMENT_INDEX.is_empty());
        for (id, payload) in SURFACE_REGISTRY_FRAGMENT_INDEX {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(payload).unwrap()["id"],
                id
            );
        }
=======
    use super::{SURFACE_REGISTRY_FRAGMENT_INDEX, load_surface_registry_manifest};
    use serde_json::Value;
    use std::collections::BTreeSet;

    #[test]
    fn fragment_index_matches_manifest() {
        let manifest = load_surface_registry_manifest();
        let manifest_surfaces = manifest
            .get("surfaces")
            .and_then(Value::as_array)
            .expect("manifest.surfaces must be an array");

        let mut manifest_ids = BTreeSet::new();
        for entry in manifest_surfaces {
            manifest_ids.insert(
                entry
                    .get("id")
                    .and_then(Value::as_str)
                    .expect("manifest surface id must be a string")
                    .to_string(),
            );
        }

        let mut index_ids = BTreeSet::new();
        for (id, _) in SURFACE_REGISTRY_FRAGMENT_INDEX.iter() {
            index_ids.insert((*id).to_string());
        }

        assert_eq!(manifest_ids, index_ids);
>>>>>>> github/main
    }
}
