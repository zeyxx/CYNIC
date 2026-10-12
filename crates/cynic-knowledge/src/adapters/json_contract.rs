use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use serde_json::Value;

/// Parses a normative JSON contract into high-authority structural chunks.
pub fn parse_json_contract(source_path: &str, content: &str) -> Vec<DocumentChunk> {
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
                AuthorityTier::L0Constitution,
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
                AuthorityTier::L0Constitution,
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
