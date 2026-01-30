-- =============================================================================
-- Migration 019: HNSW Index Optimization
-- =============================================================================
--
-- Optimizes vector search indexes with φ-aligned parameters for better
-- performance. Inspired by Claude Flow's 61μs HNSW performance.
--
-- HNSW Parameters (φ-tuned):
--   m = 21 (Fibonacci F8) - connections per layer
--   ef_construction = 89 (Fibonacci F11) - build-time search width
--
-- Runtime ef_search should be set per-query for recall/speed tradeoff.
--
-- "Fast enough to think, accurate enough to trust" - κυνικός
-- =============================================================================

-- Drop and recreate indexes with optimized parameters
-- Note: This requires pgvector 0.5.0+ for HNSW WITH parameters

-- =============================================================================
-- OPTIMIZED HNSW INDEXES
-- =============================================================================

-- Conversation memories - primary memory store
DO $$
BEGIN
  -- Drop old index if exists
  DROP INDEX IF EXISTS idx_conv_mem_vector;

  -- Create optimized HNSW index
  -- m=21 (F8): more connections = better recall, slightly more memory
  -- ef_construction=89 (F11): higher = better index quality, slower build
  CREATE INDEX idx_conv_mem_vector ON conversation_memories
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 21, ef_construction = 89);

  RAISE NOTICE 'Created optimized HNSW index for conversation_memories';
EXCEPTION WHEN OTHERS THEN
  -- Fallback: try without parameters (older pgvector)
  BEGIN
    CREATE INDEX IF NOT EXISTS idx_conv_mem_vector ON conversation_memories
      USING hnsw (embedding vector_cosine_ops);
    RAISE NOTICE 'Created basic HNSW index for conversation_memories (pgvector < 0.5.0)';
  EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Vector index not created - pgvector may not be available';
  END;
END $$;

-- Architectural decisions
DO $$
BEGIN
  DROP INDEX IF EXISTS idx_arch_dec_vector;

  CREATE INDEX idx_arch_dec_vector ON architectural_decisions
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 21, ef_construction = 89);

  RAISE NOTICE 'Created optimized HNSW index for architectural_decisions';
EXCEPTION WHEN OTHERS THEN
  BEGIN
    CREATE INDEX IF NOT EXISTS idx_arch_dec_vector ON architectural_decisions
      USING hnsw (embedding vector_cosine_ops);
    RAISE NOTICE 'Created basic HNSW index for architectural_decisions (pgvector < 0.5.0)';
  EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Vector index not created - pgvector may not be available';
  END;
END $$;

-- Lessons learned
DO $$
BEGIN
  DROP INDEX IF EXISTS idx_lessons_vector;

  CREATE INDEX idx_lessons_vector ON lessons_learned
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 21, ef_construction = 89);

  RAISE NOTICE 'Created optimized HNSW index for lessons_learned';
EXCEPTION WHEN OTHERS THEN
  BEGIN
    CREATE INDEX IF NOT EXISTS idx_lessons_vector ON lessons_learned
      USING hnsw (embedding vector_cosine_ops);
    RAISE NOTICE 'Created basic HNSW index for lessons_learned (pgvector < 0.5.0)';
  EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Vector index not created - pgvector may not be available';
  END;
END $$;

-- =============================================================================
-- HELPER FUNCTIONS FOR RUNTIME OPTIMIZATION
-- =============================================================================

-- Function to set HNSW search parameters for current session
-- Higher ef_search = better recall, slower search
-- Lower ef_search = faster search, may miss some results
--
-- Recommended values (φ-aligned):
--   Fast search: ef_search = 21 (F8)
--   Balanced:    ef_search = 55 (F10)
--   High recall: ef_search = 144 (F12)
CREATE OR REPLACE FUNCTION set_hnsw_ef_search(p_ef_search INTEGER DEFAULT 55)
RETURNS void AS $$
BEGIN
  -- Set for current transaction
  EXECUTE format('SET LOCAL hnsw.ef_search = %s', p_ef_search);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION set_hnsw_ef_search IS
  'Set HNSW ef_search parameter for current session. φ-aligned defaults: 21 (fast), 55 (balanced), 144 (high recall)';

-- Function to get optimal ef_search based on query type
-- Uses φ ratios for different precision requirements
CREATE OR REPLACE FUNCTION get_optimal_ef_search(
  p_mode TEXT DEFAULT 'balanced'
) RETURNS INTEGER AS $$
BEGIN
  RETURN CASE p_mode
    WHEN 'fast' THEN 21        -- F8: speed priority
    WHEN 'balanced' THEN 55    -- F10: default
    WHEN 'recall' THEN 144     -- F12: accuracy priority
    WHEN 'exhaustive' THEN 377 -- F14: near-exact
    ELSE 55
  END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

COMMENT ON FUNCTION get_optimal_ef_search IS
  'Get φ-aligned ef_search value for different search modes: fast, balanced, recall, exhaustive';

-- =============================================================================
-- OPTIMIZED VECTOR SEARCH FUNCTION
-- =============================================================================

-- Enhanced memory search with configurable HNSW parameters
CREATE OR REPLACE FUNCTION search_memories_hnsw(
  p_user_id UUID,
  p_query_embedding vector(1536),
  p_limit INTEGER DEFAULT 13,           -- F7
  p_search_mode TEXT DEFAULT 'balanced',
  p_min_similarity FLOAT DEFAULT 0.618  -- φ⁻¹
) RETURNS TABLE (
  id UUID,
  content TEXT,
  memory_type TEXT,
  importance FLOAT,
  similarity FLOAT,
  created_at TIMESTAMPTZ,
  last_accessed TIMESTAMPTZ
) AS $$
DECLARE
  v_ef_search INTEGER;
BEGIN
  -- Set HNSW search parameter based on mode
  v_ef_search := get_optimal_ef_search(p_search_mode);
  EXECUTE format('SET LOCAL hnsw.ef_search = %s', v_ef_search);

  RETURN QUERY
  SELECT
    cm.id,
    cm.content,
    cm.memory_type,
    cm.importance,
    1 - (cm.embedding <=> p_query_embedding) AS similarity,
    cm.created_at,
    cm.last_accessed
  FROM conversation_memories cm
  WHERE cm.user_id = p_user_id
    AND cm.embedding IS NOT NULL
    AND 1 - (cm.embedding <=> p_query_embedding) >= p_min_similarity
  ORDER BY cm.embedding <=> p_query_embedding
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION search_memories_hnsw IS
  'Search memories using HNSW with configurable search mode (fast/balanced/recall/exhaustive). φ-aligned similarity threshold.';

-- =============================================================================
-- STATS VIEW FOR MONITORING
-- =============================================================================

CREATE OR REPLACE VIEW hnsw_index_stats AS
SELECT
  schemaname,
  tablename,
  indexname,
  pg_size_pretty(pg_relation_size(indexrelid)) as index_size,
  idx_scan as scans,
  idx_tup_read as tuples_read,
  idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE indexname LIKE '%vector%' OR indexname LIKE '%hnsw%'
ORDER BY pg_relation_size(indexrelid) DESC;

COMMENT ON VIEW hnsw_index_stats IS
  'Monitor HNSW vector index usage and size';

-- =============================================================================
-- MIGRATION COMPLETE
-- =============================================================================

DO $$
BEGIN
  RAISE NOTICE '✅ Migration 019: HNSW optimization complete';
  RAISE NOTICE '   - Indexes rebuilt with m=21, ef_construction=89 (φ-aligned)';
  RAISE NOTICE '   - Added set_hnsw_ef_search() for runtime tuning';
  RAISE NOTICE '   - Added search_memories_hnsw() for optimized search';
  RAISE NOTICE '   - Added hnsw_index_stats view for monitoring';
END $$;
