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
pub mod query_routing;
pub mod scorer;
pub mod synthesis;
pub mod vector_cache;
pub mod vectorizer;

pub use ast::{
    CodeBlock, CorpusManifest, DocumentTree, MarkdownLink, SectionNode, CORPUS_MANIFEST_SCHEMA_V1,
    DOCUMENT_TREE_SCHEMA_V1,
};
pub use authority::{
    authority_for, AuthorityOverrides, AUTHORITY_TIERS_REGISTRY_PATH, AUTHORITY_TIERS_SCHEMA_V1,
};
pub use cache::{CORPUS_CACHE_SCHEMA_V2, CachedFileRecord, CorpusCacheRecord};
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
pub use query_routing::{QueryRoute, RoutingTable, route_query};
pub use scorer::{LexicalIndex, SearchHit, bm25_margin, dense_margin, tokenize};
pub use cache::{CachedFileRecord, CorpusCacheRecord, CORPUS_CACHE_SCHEMA_V2};
pub use chunk::{AuthorityTier, DocumentChunk};
pub use chunk_quality::{
    split_span_by_char_budget, ChunkAnomaly, ChunkLengthPercentiles, ChunkQualityReport,
    CHAR_WINDOW_CHARS, CHAR_WINDOW_OVERLAP_CHARS, CHUNK_QUALITY_REPORT_SCHEMA_V1,
    DEFAULT_MAX_CHUNK_CHARS, DEFAULT_MIN_CHUNK_CHARS, MAX_CHUNK_CHARS,
};
pub use commit::{GitCommitRecord, GIT_COMMIT_SCHEMA_V1};
pub use continuous_fusion::{
    fuse_continuous_convex, min_max_normalize, EmpiricalCdf, NormalizationStrategy,
};
pub use epistemic_grounding::{
    assemble_epistemic_context, compute_term_coverage, reorder_lost_in_the_middle,
    sort_by_topological_authority, EpistemicAssemblyConfig, EpistemicCoverageVerdict,
    EpistemicSource, DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD, DEFAULT_MIN_TERM_COVERAGE_RATIO,
    EPISTEMIC_PROMPT_SCHEMA_V1, UNCOVERED_REJECTION_TOKEN,
};
pub use geometry::{CrateGeometryRecord, WorkspaceGeometry, WORKSPACE_GEOMETRY_SCHEMA_V1};
pub use graph::{BrokenReference, KnowledgeEdge, KnowledgeEdgeKind, KnowledgeGraph};
pub use graph_expansion::expand_hits_with_graph;
pub use hybrid_rrf::{fuse_reciprocal_ranks, DenseHit, DEFAULT_RRF_K};
pub use query_routing::{route_query, QueryRoute, RoutingTable};
pub use scorer::{bm25_margin, dense_margin, tokenize, LexicalIndex, SearchHit};
pub use synthesis::{
    CitedSource, SynthesisPrompt, SynthesisPromptBuilder, SYNTHESIS_PROMPT_SCHEMA_V1,
};
pub use vector_cache::{
    load_vector_cache, normalize_engine, save_vector_cache, vectorize_chunks_cached, VectorCacheRecord,
    VECTOR_CACHE_SCHEMA_V1,
};
pub use vectorizer::{
    cosine_similarity, top_k_dense, vectorize_chunks, ChunkEmbedder, ChunkVector,
};
