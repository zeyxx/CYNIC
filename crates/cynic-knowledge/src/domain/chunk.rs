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
}
