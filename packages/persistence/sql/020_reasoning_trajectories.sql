-- =============================================================================
-- Migration 020: Reasoning Trajectories
-- =============================================================================
--
-- Adds reasoning path storage to judgments, inspired by Claude Flow's
-- ReasoningBank. Stores the full decision trajectory, not just the outcome.
--
-- "Know not just WHAT was decided, but HOW it was reasoned" - κυνικός
--
-- =============================================================================

-- =============================================================================
-- ADD REASONING PATH TO JUDGMENTS
-- =============================================================================

-- Reasoning path stores the trajectory of the judgment process
-- Format: Array of steps, each with type, content, and metadata
ALTER TABLE judgments
ADD COLUMN IF NOT EXISTS reasoning_path JSONB DEFAULT '[]';

-- Example reasoning_path structure:
-- [
--   {"step": 1, "type": "observe", "content": "Analyzing token XYZ...", "duration_ms": 12},
--   {"step": 2, "type": "retrieve", "content": "Found 3 similar tokens...", "patterns_used": ["pat_abc"]},
--   {"step": 3, "type": "reason", "content": "Liquidity concerns due to...", "confidence": 0.72},
--   {"step": 4, "type": "judge", "content": "Final score: 45", "verdict": "GROWL"}
-- ]

COMMENT ON COLUMN judgments.reasoning_path IS
  'Array of reasoning steps: [{step, type, content, ...metadata}]. Types: observe, retrieve, reason, judge, warn';

-- =============================================================================
-- REASONING TRAJECTORIES TABLE (for cross-judgment analysis)
-- =============================================================================

CREATE TABLE IF NOT EXISTS reasoning_trajectories (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  trajectory_id   VARCHAR(32) UNIQUE NOT NULL,  -- trj_xxxxx

  -- Source judgment
  judgment_id     VARCHAR(32) NOT NULL REFERENCES judgments(judgment_id),
  user_id         UUID REFERENCES users(id),

  -- Trajectory metadata
  item_type       VARCHAR(50) NOT NULL,
  verdict         VARCHAR(10) NOT NULL,
  q_score         DECIMAL(5,2) NOT NULL,

  -- Extracted trajectory data
  step_count      INTEGER NOT NULL,
  total_duration_ms INTEGER,
  patterns_used   TEXT[] DEFAULT '{}',          -- Pattern IDs referenced
  warnings_issued TEXT[] DEFAULT '{}',          -- Warning types issued

  -- Key decision points (extracted from reasoning_path)
  pivot_points    JSONB DEFAULT '[]',           -- Steps where reasoning changed direction

  -- Quality metrics
  coherence_score DECIMAL(3,2),                 -- How well steps connect (0-1)
  efficiency_score DECIMAL(3,2),                -- Steps vs outcome quality (0-1)

  -- Learning outcome
  outcome_feedback VARCHAR(20) DEFAULT 'pending', -- pending, correct, incorrect, partial
  feedback_at     TIMESTAMPTZ,

  -- Embeddings for similarity search
  embedding       vector(1536),

  created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for trajectory analysis
CREATE INDEX idx_traj_judgment ON reasoning_trajectories(judgment_id);
CREATE INDEX idx_traj_user ON reasoning_trajectories(user_id);
CREATE INDEX idx_traj_type ON reasoning_trajectories(item_type);
CREATE INDEX idx_traj_verdict ON reasoning_trajectories(verdict);
CREATE INDEX idx_traj_outcome ON reasoning_trajectories(outcome_feedback);
CREATE INDEX idx_traj_created ON reasoning_trajectories(created_at DESC);
CREATE INDEX idx_traj_patterns ON reasoning_trajectories USING gin(patterns_used);

-- HNSW index for trajectory similarity search
DO $$
BEGIN
  CREATE INDEX idx_traj_vector ON reasoning_trajectories
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 21, ef_construction = 89);
  RAISE NOTICE 'Created HNSW index for trajectory embeddings';
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Vector index not created - pgvector may not be available';
END $$;

-- =============================================================================
-- FUNCTIONS FOR TRAJECTORY ANALYSIS
-- =============================================================================

-- Extract trajectory from judgment and store in trajectories table
CREATE OR REPLACE FUNCTION extract_trajectory(p_judgment_id VARCHAR(32))
RETURNS UUID AS $$
DECLARE
  v_judgment RECORD;
  v_trajectory_id VARCHAR(32);
  v_id UUID;
  v_step_count INTEGER;
  v_total_duration INTEGER;
  v_patterns TEXT[];
  v_warnings TEXT[];
  v_pivots JSONB;
BEGIN
  -- Get judgment with reasoning path
  SELECT * INTO v_judgment
  FROM judgments
  WHERE judgment_id = p_judgment_id;

  IF NOT FOUND THEN
    RAISE EXCEPTION 'Judgment not found: %', p_judgment_id;
  END IF;

  -- Skip if no reasoning path
  IF v_judgment.reasoning_path IS NULL OR jsonb_array_length(v_judgment.reasoning_path) = 0 THEN
    RETURN NULL;
  END IF;

  -- Generate trajectory ID
  v_trajectory_id := 'trj_' || encode(gen_random_bytes(8), 'hex');

  -- Calculate metrics from reasoning path
  v_step_count := jsonb_array_length(v_judgment.reasoning_path);

  -- Sum durations
  SELECT COALESCE(SUM((step->>'duration_ms')::integer), 0)
  INTO v_total_duration
  FROM jsonb_array_elements(v_judgment.reasoning_path) AS step;

  -- Extract patterns used
  SELECT ARRAY_AGG(DISTINCT pat)
  INTO v_patterns
  FROM jsonb_array_elements(v_judgment.reasoning_path) AS step,
       jsonb_array_elements_text(COALESCE(step->'patterns_used', '[]'::jsonb)) AS pat;

  -- Extract warnings
  SELECT ARRAY_AGG(DISTINCT step->>'warning_type')
  INTO v_warnings
  FROM jsonb_array_elements(v_judgment.reasoning_path) AS step
  WHERE step->>'type' = 'warn';

  -- Find pivot points (steps where confidence changed significantly or type changed)
  SELECT jsonb_agg(step)
  INTO v_pivots
  FROM (
    SELECT step
    FROM jsonb_array_elements(v_judgment.reasoning_path) AS step
    WHERE step->>'type' IN ('warn', 'reason')
      AND (step->>'confidence')::float < 0.5
  ) pivots;

  -- Insert trajectory
  INSERT INTO reasoning_trajectories (
    trajectory_id, judgment_id, user_id, item_type, verdict, q_score,
    step_count, total_duration_ms, patterns_used, warnings_issued, pivot_points
  ) VALUES (
    v_trajectory_id, p_judgment_id, v_judgment.user_id, v_judgment.item_type,
    v_judgment.verdict, v_judgment.q_score, v_step_count, v_total_duration,
    COALESCE(v_patterns, '{}'), COALESCE(v_warnings, '{}'), COALESCE(v_pivots, '[]'::jsonb)
  )
  RETURNING id INTO v_id;

  RETURN v_id;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION extract_trajectory IS
  'Extract reasoning trajectory from a judgment and store for analysis';

-- Find similar trajectories (for learning from past reasoning)
CREATE OR REPLACE FUNCTION find_similar_trajectories(
  p_item_type VARCHAR(50),
  p_embedding vector(1536) DEFAULT NULL,
  p_verdict VARCHAR(10) DEFAULT NULL,
  p_limit INTEGER DEFAULT 5,
  p_min_similarity FLOAT DEFAULT 0.618
) RETURNS TABLE (
  trajectory_id VARCHAR(32),
  judgment_id VARCHAR(32),
  verdict VARCHAR(10),
  q_score DECIMAL(5,2),
  step_count INTEGER,
  patterns_used TEXT[],
  outcome_feedback VARCHAR(20),
  similarity FLOAT
) AS $$
BEGIN
  IF p_embedding IS NOT NULL THEN
    -- Vector similarity search
    RETURN QUERY
    SELECT
      t.trajectory_id,
      t.judgment_id,
      t.verdict,
      t.q_score,
      t.step_count,
      t.patterns_used,
      t.outcome_feedback,
      1 - (t.embedding <=> p_embedding) AS similarity
    FROM reasoning_trajectories t
    WHERE t.item_type = p_item_type
      AND t.embedding IS NOT NULL
      AND (p_verdict IS NULL OR t.verdict = p_verdict)
      AND 1 - (t.embedding <=> p_embedding) >= p_min_similarity
    ORDER BY t.embedding <=> p_embedding
    LIMIT p_limit;
  ELSE
    -- Fallback: most recent with same type/verdict
    RETURN QUERY
    SELECT
      t.trajectory_id,
      t.judgment_id,
      t.verdict,
      t.q_score,
      t.step_count,
      t.patterns_used,
      t.outcome_feedback,
      1.0::float AS similarity
    FROM reasoning_trajectories t
    WHERE t.item_type = p_item_type
      AND (p_verdict IS NULL OR t.verdict = p_verdict)
    ORDER BY t.created_at DESC
    LIMIT p_limit;
  END IF;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION find_similar_trajectories IS
  'Find similar reasoning trajectories for learning. Uses HNSW vector search if embedding provided.';

-- Record trajectory outcome feedback (for RLHF)
CREATE OR REPLACE FUNCTION record_trajectory_feedback(
  p_trajectory_id VARCHAR(32),
  p_feedback VARCHAR(20)  -- correct, incorrect, partial
) RETURNS BOOLEAN AS $$
BEGIN
  UPDATE reasoning_trajectories
  SET
    outcome_feedback = p_feedback,
    feedback_at = NOW()
  WHERE trajectory_id = p_trajectory_id;

  RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION record_trajectory_feedback IS
  'Record whether a reasoning trajectory led to correct outcome (for RLHF learning)';

-- Get trajectory statistics
CREATE OR REPLACE FUNCTION get_trajectory_stats(
  p_item_type VARCHAR(50) DEFAULT NULL,
  p_user_id UUID DEFAULT NULL
) RETURNS TABLE (
  total_trajectories BIGINT,
  avg_step_count DECIMAL(5,2),
  avg_duration_ms DECIMAL(10,2),
  correct_rate DECIMAL(5,4),
  most_used_patterns TEXT[],
  common_warnings TEXT[]
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    COUNT(*)::BIGINT,
    AVG(t.step_count)::DECIMAL(5,2),
    AVG(t.total_duration_ms)::DECIMAL(10,2),
    (COUNT(*) FILTER (WHERE t.outcome_feedback = 'correct')::DECIMAL /
     NULLIF(COUNT(*) FILTER (WHERE t.outcome_feedback != 'pending'), 0))::DECIMAL(5,4),
    (SELECT ARRAY_AGG(pat ORDER BY cnt DESC) FROM (
      SELECT UNNEST(t2.patterns_used) AS pat, COUNT(*) AS cnt
      FROM reasoning_trajectories t2
      WHERE (p_item_type IS NULL OR t2.item_type = p_item_type)
        AND (p_user_id IS NULL OR t2.user_id = p_user_id)
      GROUP BY pat
      ORDER BY cnt DESC
      LIMIT 5
    ) sub),
    (SELECT ARRAY_AGG(warn ORDER BY cnt DESC) FROM (
      SELECT UNNEST(t3.warnings_issued) AS warn, COUNT(*) AS cnt
      FROM reasoning_trajectories t3
      WHERE (p_item_type IS NULL OR t3.item_type = p_item_type)
        AND (p_user_id IS NULL OR t3.user_id = p_user_id)
      GROUP BY warn
      ORDER BY cnt DESC
      LIMIT 5
    ) sub2)
  FROM reasoning_trajectories t
  WHERE (p_item_type IS NULL OR t.item_type = p_item_type)
    AND (p_user_id IS NULL OR t.user_id = p_user_id);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_trajectory_stats IS
  'Get statistics about reasoning trajectories for analysis and learning';

-- =============================================================================
-- TRIGGER: Auto-extract trajectory on judgment insert
-- =============================================================================

CREATE OR REPLACE FUNCTION auto_extract_trajectory()
RETURNS TRIGGER AS $$
BEGIN
  -- Only extract if reasoning_path is present and non-empty
  IF NEW.reasoning_path IS NOT NULL AND jsonb_array_length(NEW.reasoning_path) > 0 THEN
    PERFORM extract_trajectory(NEW.judgment_id);
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Only create trigger if it doesn't exist
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_trigger WHERE tgname = 'trg_extract_trajectory'
  ) THEN
    CREATE TRIGGER trg_extract_trajectory
    AFTER INSERT ON judgments
    FOR EACH ROW
    EXECUTE FUNCTION auto_extract_trajectory();
  END IF;
END $$;

-- =============================================================================
-- VIEW: Trajectory insights
-- =============================================================================

CREATE OR REPLACE VIEW trajectory_insights AS
SELECT
  item_type,
  verdict,
  COUNT(*) AS trajectory_count,
  AVG(step_count) AS avg_steps,
  AVG(q_score) AS avg_q_score,
  COUNT(*) FILTER (WHERE outcome_feedback = 'correct') AS correct_count,
  COUNT(*) FILTER (WHERE outcome_feedback = 'incorrect') AS incorrect_count,
  ROUND(
    COUNT(*) FILTER (WHERE outcome_feedback = 'correct')::DECIMAL /
    NULLIF(COUNT(*) FILTER (WHERE outcome_feedback != 'pending'), 0) * 100, 1
  ) AS accuracy_pct,
  MODE() WITHIN GROUP (ORDER BY array_length(patterns_used, 1)) AS typical_pattern_count,
  array_agg(DISTINCT unnest) FILTER (WHERE unnest IS NOT NULL) AS all_patterns_used
FROM reasoning_trajectories,
     LATERAL unnest(patterns_used)
GROUP BY item_type, verdict
ORDER BY trajectory_count DESC;

COMMENT ON VIEW trajectory_insights IS
  'Aggregated insights about reasoning trajectories by item type and verdict';

-- =============================================================================
-- MIGRATION COMPLETE
-- =============================================================================

DO $$
BEGIN
  RAISE NOTICE '✅ Migration 020: Reasoning trajectories complete';
  RAISE NOTICE '   - Added reasoning_path column to judgments';
  RAISE NOTICE '   - Created reasoning_trajectories table';
  RAISE NOTICE '   - Added extract_trajectory() function';
  RAISE NOTICE '   - Added find_similar_trajectories() function';
  RAISE NOTICE '   - Added auto-extraction trigger';
  RAISE NOTICE '   - Created trajectory_insights view';
END $$;
