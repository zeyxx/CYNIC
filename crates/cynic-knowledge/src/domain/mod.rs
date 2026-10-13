pub mod ast;
pub mod cache;
pub mod chunk;
pub mod commit;
pub mod geometry;
pub mod graph;
pub mod scorer;
pub mod synthesis;

pub use ast::{
    CORPUS_MANIFEST_SCHEMA_V1, CodeBlock, CorpusManifest, DOCUMENT_TREE_SCHEMA_V1, DocumentTree,
    MarkdownLink, SectionNode,
};
pub use cache::{CORPUS_CACHE_SCHEMA_V1, CachedFileRecord, CorpusCacheRecord};
pub use chunk::{AuthorityTier, DocumentChunk};
pub use commit::{GIT_COMMIT_SCHEMA_V1, GitCommitRecord};
pub use geometry::{CrateGeometryRecord, WORKSPACE_GEOMETRY_SCHEMA_V1, WorkspaceGeometry};
pub use graph::{BrokenReference, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph};
pub use scorer::{LexicalIndex, SearchHit};
pub use synthesis::{
    CitedSource, SYNTHESIS_PROMPT_SCHEMA_V1, SynthesisPrompt, SynthesisPromptBuilder,
};
