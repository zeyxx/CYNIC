use crate::domain::authority::{AUTHORITY_TIERS_REGISTRY_PATH, AuthorityOverrides};
use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use serde_json::Value;
use std::path::Path;

/// Loads the authority-tier registry for a workspace root.
/// Missing or malformed registry fails open to empty overrides (directory
/// defaults apply); tiers are ranking hints, never a security gate.
pub fn load_authority_overrides<P: AsRef<Path>>(root: P) -> AuthorityOverrides {
    let path = root.as_ref().join(AUTHORITY_TIERS_REGISTRY_PATH);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| AuthorityOverrides::from_json(&text).ok())
        .unwrap_or_default()
}

/// Parses a JSON contract into structural chunks with directory-default authority.
pub fn parse_json_contract(source_path: &str, content: &str) -> Vec<DocumentChunk> {
    parse_json_contract_with_authority(source_path, content, AuthorityTier::from_path(source_path))
}

/// Parses a JSON contract into structural chunks with caller-resolved authority.
pub fn parse_json_contract_with_authority(
    source_path: &str,
    content: &str,
    authority: AuthorityTier,
) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
    let parsed: Result<Value, _> = serde_json::from_str(content);

    let val = match parsed {
        Ok(v) => v,
        Err(_) => {
            // Fallback: whole raw JSON as single chunk
            return vec![DocumentChunk::new(
                source_path.to_string(),
                1,
                content.lines().count(),
                vec![format!("Contract: {}", source_path)],
                content.to_string(),
                authority,
            )];
        }
    };

    if let Value::Object(map) = val {
        // If the contract has a schema or description, extract it
        let title = map
            .get("name")
            .or_else(|| map.get("contract"))
            .or_else(|| map.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or(source_path)
            .to_string();

        // Group into concise chunks
        for (key, section_val) in map {
            let section_str = serde_json::to_string_pretty(&section_val).unwrap_or_default();
            chunks.push(DocumentChunk::new(
                source_path.to_string(),
                1,
                content.lines().count(),
                vec![format!("Contract: {}", title), format!("Field: {}", key)],
                format!("{}:\n{}", key, section_str),
                authority,
            ));
        }
    } else {
        chunks.push(DocumentChunk::new(
            source_path.to_string(),
            1,
            content.lines().count(),
            vec![format!("Contract: {}", source_path)],
            content.to_string(),
            AuthorityTier::L0Constitution,
        ));
    }

    chunks
}
