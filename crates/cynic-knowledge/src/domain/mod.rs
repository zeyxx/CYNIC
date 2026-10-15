pub mod ast;
pub mod authority;
pub mod cache;
pub mod chunk;
pub mod chunk_quality;
pub mod commit;
pub mod continuous_fusion;
pub mod epistemic_grounding;
pub mod geometry;
pub mod graph;
pub mod graph_expansion;
pub mod hybrid_rrf;
pub mod scorer;
pub mod synthesis;
pub mod vector_cache;
pub mod vectorizer;

pub use ast::{
    CORPUS_MANIFEST_SCHEMA_V1, CodeBlock, CorpusManifest, DOCUMENT_TREE_SCHEMA_V1, DocumentTree,
    MarkdownLink, SectionNode,
};
pub use authority::{
    AUTHORITY_TIERS_REGISTRY_PATH, AUTHORITY_TIERS_SCHEMA_V1, AuthorityOverrides, authority_for,
};
pub use cache::{CORPUS_CACHE_SCHEMA_V1, CachedFileRecord, CorpusCacheRecord};
pub use chunk::{AuthorityTier, DocumentChunk};
pub use chunk_quality::{
    CHAR_WINDOW_CHARS, CHAR_WINDOW_OVERLAP_CHARS, CHUNK_QUALITY_REPORT_SCHEMA_V1, ChunkAnomaly,
    ChunkLengthPercentiles, ChunkQualityReport, DEFAULT_MAX_CHUNK_CHARS, DEFAULT_MIN_CHUNK_CHARS,
    MAX_CHUNK_CHARS, split_span_by_char_budget,
};
pub use commit::{GIT_COMMIT_SCHEMA_V1, GitCommitRecord};
pub use continuous_fusion::{
    EmpiricalCdf, NormalizationStrategy, fuse_continuous_convex, min_max_normalize,
};
pub use epistemic_grounding::{
    DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD, DEFAULT_MIN_TERM_COVERAGE_RATIO,
    EPISTEMIC_PROMPT_SCHEMA_V1, EpistemicAssemblyConfig, EpistemicCoverageVerdict, EpistemicSource,
    UNCOVERED_REJECTION_TOKEN, assemble_epistemic_context, compute_term_coverage,
    reorder_lost_in_the_middle, sort_by_topological_authority,
};
pub use geometry::{CrateGeometryRecord, WORKSPACE_GEOMETRY_SCHEMA_V1, WorkspaceGeometry};
pub use graph::{BrokenReference, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph};
pub use graph_expansion::expand_hits_with_graph;
pub use hybrid_rrf::{DEFAULT_RRF_K, DenseHit, fuse_reciprocal_ranks};
pub use scorer::{LexicalIndex, SearchHit, bm25_margin, dense_margin, tokenize};
pub use synthesis::{
    CitedSource, SYNTHESIS_PROMPT_SCHEMA_V1, SynthesisPrompt, SynthesisPromptBuilder,
};
pub use vector_cache::{
    VECTOR_CACHE_SCHEMA_V1, VectorCacheRecord, load_vector_cache, save_vector_cache,
    vectorize_chunks_cached,
};
pub use vectorizer::{
    ChunkEmbedder, ChunkVector, cosine_similarity, top_k_dense, vectorize_chunks,
};
