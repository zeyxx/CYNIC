pub mod ast;
pub mod cache;
pub mod chunk;
pub mod commit;
pub mod geometry;
pub mod graph;
pub mod graph_expansion;
pub mod hybrid_rrf;
pub mod scorer;
pub mod synthesis;
pub mod vectorizer;

pub use ast::{
    CORPUS_MANIFEST_SCHEMA_V1, CodeBlock, CorpusManifest, DOCUMENT_TREE_SCHEMA_V1, DocumentTree,
    MarkdownLink, SectionNode,
};
pub use cache::{CORPUS_CACHE_SCHEMA_V1, CachedFileRecord, CorpusCacheRecord};
pub use chunk::{AuthorityTier, DocumentChunk};
pub use commit::{GIT_COMMIT_SCHEMA_V1, GitCommitRecord};
pub use geometry::{CrateGeometryRecord, WORKSPACE_GEOMETRY_SCHEMA_V1, WorkspaceGeometry};
pub use graph::{BrokenReference, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph};
pub use graph_expansion::expand_hits_with_graph;
pub use hybrid_rrf::{DEFAULT_RRF_K, DenseHit, fuse_reciprocal_ranks};
pub use scorer::{LexicalIndex, SearchHit};
pub use synthesis::{
    CitedSource, SYNTHESIS_PROMPT_SCHEMA_V1, SynthesisPrompt, SynthesisPromptBuilder,
};
pub use vectorizer::{
    ChunkEmbedder, ChunkVector, cosine_similarity, top_k_dense, vectorize_chunks,
};
