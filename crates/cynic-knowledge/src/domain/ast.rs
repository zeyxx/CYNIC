use crate::domain::chunk::AuthorityTier;
use serde::{Deserialize, Serialize};

pub const DOCUMENT_TREE_SCHEMA_V1: &str = "cynic.knowledge.document_tree.v1";
pub const CORPUS_MANIFEST_SCHEMA_V1: &str = "cynic.knowledge.corpus_manifest.v1";

/// Code block extracted from a Markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
    pub line_start: usize,
    pub line_end: usize,
}

/// A hyperlink reference extracted from Markdown text `[label](target)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownLink {
    pub label: String,
    pub target: String,
}

/// A node in the hierarchical section tree of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionNode {
    pub section_id: String,
    pub level: usize,
    pub title: String,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub code_blocks: Vec<CodeBlock>,
    pub links: Vec<MarkdownLink>,
    pub children: Vec<SectionNode>,
}

impl SectionNode {
    /// Recursively counts all sections in this sub-tree (including self).
    pub fn count_sections(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.count_sections())
            .sum::<usize>()
    }

    /// Recursively counts all code blocks in this sub-tree.
    pub fn count_code_blocks(&self) -> usize {
        self.code_blocks.len()
            + self
                .children
                .iter()
                .map(|c| c.count_code_blocks())
                .sum::<usize>()
    }
}

/// Complete structural AST of a single Markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTree {
    pub schema: String,
    pub source_path: String,
    pub document_hash: String,
    pub byte_size: usize,
    pub line_count: usize,
    pub authority: AuthorityTier,
    pub title: String,
    pub root_sections: Vec<SectionNode>,
}

impl DocumentTree {
    /// Total sections in this document.
    pub fn total_sections(&self) -> usize {
        self.root_sections.iter().map(|s| s.count_sections()).sum()
    }

    /// Total code blocks in this document.
    pub fn total_code_blocks(&self) -> usize {
        self.root_sections
            .iter()
            .map(|s| s.count_code_blocks())
            .sum()
    }
}

/// A versioned manifest representing the entire parsed corpus across the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub schema: String,
    pub corpus_hash: String,
    pub total_documents: usize,
    pub total_sections: usize,
    pub total_code_blocks: usize,
    pub total_bytes: usize,
    pub documents: Vec<DocumentTree>,
}

impl CorpusManifest {
    /// Builds a new CorpusManifest from an ordered list of DocumentTrees.
    pub fn from_documents(mut documents: Vec<DocumentTree>) -> Self {
        documents.sort_by(|a, b| a.source_path.cmp(&b.source_path));

        let mut hasher = blake3::Hasher::new();
        let mut total_sections = 0;
        let mut total_code_blocks = 0;
        let mut total_bytes = 0;

        for doc in &documents {
            hasher.update(doc.source_path.as_bytes());
            hasher.update(doc.document_hash.as_bytes());
            total_sections += doc.total_sections();
            total_code_blocks += doc.total_code_blocks();
            total_bytes += doc.byte_size;
        }

        let corpus_hash = hasher.finalize().to_hex().to_string();

        Self {
            schema: CORPUS_MANIFEST_SCHEMA_V1.to_string(),
            corpus_hash,
            total_documents: documents.len(),
            total_sections,
            total_code_blocks,
            total_bytes,
            documents,
        }
    }
}
