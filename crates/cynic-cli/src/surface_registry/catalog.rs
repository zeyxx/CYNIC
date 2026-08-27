use crate::surface_registry::types::CanonicalCliSurface;
use serde_json::Value;
use std::collections::BTreeSet;

const SURFACE_REGISTRY_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../infra/surface-registry/manifest.json"
));

include!(concat!(
    env!("OUT_DIR"),
    "/surface_registry_fragment_index.rs"
));

pub(crate) fn canonical_cli_surfaces() -> Vec<CanonicalCliSurface> {
    // The runtime assembler is the canonical policy validator. This generated
    // include index only supplies compile-time payloads for the CLI catalog.
    let manifest = load_surface_registry_manifest();
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

fn fragment_for_surface(surface_id: &str) -> Option<&'static str> {
    SURFACE_REGISTRY_FRAGMENT_INDEX
        .iter()
        .find_map(|(id, json)| (*id == surface_id).then_some(*json))
}

#[cfg(test)]
mod tests {
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
    }
}
