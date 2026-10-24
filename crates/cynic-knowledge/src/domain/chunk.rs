use serde::{Deserialize, Serialize};

/// Constitutional and operational authority tiers in CYNIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuthorityTier {
    /// L0: Foundational Constitution, immutable invariants (AGENTS.md, CONSTITUTION.md, foundation.json).
    L0Constitution,
    /// L1: Formal specifications and verified schemas (specs/, contracts/).
    L1Specification,
    /// L2: Design documents, architectural RFCs, exploratory roadmaps (docs/design/).
    L2Design,
    /// L3: Operational notes, ephemeral captures, and diagnostics.
    L3Operation,
}

impl AuthorityTier {
    /// Resolves authority tier from source file path.
    pub fn from_path(path: &str) -> Self {
        if path.starts_with("contracts/")
            || path.ends_with("AGENTS.md")
            || path.ends_with("CONSTITUTION.md")
            || path.contains("docs/foundation/")
        {
            AuthorityTier::L0Constitution
        } else if path.contains("docs/specs/") {
            AuthorityTier::L1Specification
        } else if path.contains("docs/design/") {
            AuthorityTier::L2Design
        } else {
            AuthorityTier::L3Operation
        }
    }
}

/// A deterministic chunk extracted from the structural hierarchy of a document (PageIndex model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Deterministic BLAKE3 hash of content + source path.
    pub chunk_id: String,
    /// Canonical repository-relative path (e.g. "docs/foundation/CONSTITUTION.md").
    pub source_path: String,
    /// 1-indexed starting line in the source document.
    pub line_start: usize,
    /// 1-indexed ending line in the source document.
    pub line_end: usize,
    /// Hierarchical breadcrumbs (e.g. ["# 1. The Triad", "## 1.1 Pillar 1"]).
    pub section_hierarchy: Vec<String>,
    /// Exact section content (code blocks and tables preserved).
    pub content: String,
    /// Authority tier for constitutional ranking.
    pub authority: AuthorityTier,
}

impl DocumentChunk {
    /// Creates a new DocumentChunk, calculating its deterministic BLAKE3 identifier.
    pub fn new(
        source_path: String,
        line_start: usize,
        line_end: usize,
        section_hierarchy: Vec<String>,
        content: String,
        authority: AuthorityTier,
    ) -> Self {
        let hash_input = format!("{}:{}:{}", source_path, line_start, content);
        let chunk_id = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

        Self {
            chunk_id,
            source_path,
            line_start,
            line_end,
            section_hierarchy,
            content,
            authority,
        }
    }

    /// Full breadcrumb title.
    pub fn title(&self) -> String {
        if self.section_hierarchy.is_empty() {
            self.source_path.clone()
        } else {
            self.section_hierarchy.join(" > ")
        }
    }

    /// Converts this chunk into a canonical tabular record map matching schema `cynic.document_chunk.v1`.
    pub fn to_record_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert("chunk_id".to_string(), serde_json::Value::String(self.chunk_id.clone()));
        map.insert("source_path".to_string(), serde_json::Value::String(self.source_path.clone()));
        map.insert("line_start".to_string(), serde_json::json!(self.line_start as i64));
        map.insert("line_end".to_string(), serde_json::json!(self.line_end as i64));
        map.insert("char_length".to_string(), serde_json::json!(self.content.chars().count() as i64));
        map.insert("authority".to_string(), serde_json::Value::String(format!("{:?}", self.authority)));
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_chunk_to_record_map() {
        let chunk = DocumentChunk::new(
            "docs/foundation/AGENTS.md".to_string(),
            1,
            25,
            vec!["# Minimal Agent Protocol".to_string()],
            "CYNIC is a low-layer sovereign Rust runtime for multi-agent systems.".to_string(),
            AuthorityTier::L0Constitution,
        );

        let map = chunk.to_record_map();
        assert_eq!(map.get("source_path").unwrap(), "docs/foundation/AGENTS.md");
        assert_eq!(map.get("line_start").unwrap(), 1);
        assert_eq!(map.get("line_end").unwrap(), 25);
        assert_eq!(map.get("authority").unwrap(), "L0Constitution");
        assert_eq!(
            map.get("char_length").unwrap(),
            chunk.content.chars().count() as i64
        );
        assert_eq!(map.get("chunk_id").unwrap(), chunk.chunk_id.as_str());
    }
}
