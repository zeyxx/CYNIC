use crate::surface_registry::types::CanonicalCliSurface;

include!(concat!(
    env!("OUT_DIR"),
    "/surface_registry_fragment_index.rs"
));

pub(crate) fn canonical_cli_surfaces() -> Vec<CanonicalCliSurface> {
    SURFACE_REGISTRY_FRAGMENT_INDEX
        .iter()
        .map(|(id, fragment_json)| {
            let surface: CanonicalCliSurface = serde_json::from_str(fragment_json)
                .expect("surface fragment must be a valid CanonicalCliSurface object");
            assert_eq!(surface.id, *id);
            surface
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
    }
}
