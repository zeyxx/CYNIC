-- CYNIC Total Memory + Full Autonomy Schema
-- φ guides all ratios
--
-- Migration: 015_total_memory
-- Created: 2026-01-27
--
-- Total Memory: CYNIC remembers EVERYTHING
-- Full Autonomy: Goals pursued, tasks scheduled, self-correction automatic

-- =============================================================================
-- EXTENSIONS
-- =============================================================================

-- pgvector for semantic search (vector embeddings)
-- Note: Requires pgvector extension to be installed on PostgreSQL server
-- Install via: CREATE EXTENSION IF NOT EXISTS vector;
-- If not available, the embedding columns will be created but vector operations won't work
DO $$
BEGIN
  CREATE EXTENSION IF NOT EXISTS vector;
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'pgvector extension not available - vector search disabled';
END $$;

-- =============================================================================
-- PHASE 1: TOTAL MEMORY TABLES
-- =============================================================================

-- Conversation memories with vector embeddings
-- Stores summaries, key moments, decisions, and preferences from conversations
CREATE TABLE IF NOT EXISTS conversation_memories (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           VARCHAR(255) NOT NULL,
  session_id        VARCHAR(255),

  -- Memory classification
  memory_type       VARCHAR(50) NOT NULL CHECK (memory_type IN (
    'summary',       -- Session/conversation summaries
    'key_moment',    -- Important moments during conversation
    'decision',      -- Decisions made
    'preference',    -- User preferences learned
    'correction',    -- Corrections made to CYNIC's behavior
    'insight'        -- Insights extracted
  )),

  -- Content
  content           TEXT NOT NULL,

  -- Vector embedding for semantic search (1536 dimensions for OpenAI ada-002)
  -- If pgvector not available, this column will be NULL
  embedding         vector(1536),

  -- Importance scoring (0.0 to 1.0)
  importance        FLOAT DEFAULT 0.5 CHECK (importance >= 0 AND importance <= 1),

  -- Additional context (tags, source, related items)
  context           JSONB DEFAULT '{}',

  -- Access tracking for relevance decay
  created_at        TIMESTAMPTZ DEFAULT NOW(),
  last_accessed     TIMESTAMPTZ DEFAULT NOW(),
  access_count      INTEGER DEFAULT 0
);

-- Architectural decisions (design choices with rationale)
-- Tracks decisions made during software development
CREATE TABLE IF NOT EXISTS architectural_decisions (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           VARCHAR(255) NOT NULL,
  project_path      VARCHAR(500),

  -- Decision classification
  decision_type     VARCHAR(100) NOT NULL CHECK (decision_type IN (
    'pattern',       -- Design pattern choice
    'technology',    -- Technology/library choice
    'structure',     -- Code/file structure decision
    'naming',        -- Naming convention
    'api',           -- API design decision
    'data',          -- Data model decision
    'security',      -- Security-related decision
    'performance',   -- Performance optimization
    'testing',       -- Testing strategy
    'deployment',    -- Deployment/infrastructure
    'other'          -- Other decisions
  )),

  -- Content
  title             VARCHAR(500) NOT NULL,
  description       TEXT NOT NULL,
  rationale         TEXT,                    -- Why this decision was made

  -- Alternatives considered
  alternatives      JSONB DEFAULT '[]',      -- [{option, reason_rejected}]

  -- Consequences and impacts
  consequences      JSONB DEFAULT '{}',      -- {positive: [], negative: [], neutral: []}

  -- Vector embedding for semantic search
  embedding         vector(1536),

  -- Lifecycle status
  status            VARCHAR(50) DEFAULT 'active' CHECK (status IN (
    'active',        -- Currently in effect
    'superseded',    -- Replaced by newer decision
    'deprecated',    -- Should not be followed
    'pending'        -- Under consideration
  )),

  -- References
  superseded_by     UUID REFERENCES architectural_decisions(id),
  related_decisions JSONB DEFAULT '[]',      -- Array of decision IDs

  created_at        TIMESTAMPTZ DEFAULT NOW(),
  updated_at        TIMESTAMPTZ DEFAULT NOW()
);

-- Lessons learned (mistakes and corrections)
-- Tracks errors and how to prevent them in the future
CREATE TABLE IF NOT EXISTS lessons_learned (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           VARCHAR(255) NOT NULL,

  -- Category of lesson
  category          VARCHAR(100) NOT NULL CHECK (category IN (
    'bug',           -- Bug introduced and fixed
    'architecture',  -- Architectural mistake
    'process',       -- Process/workflow issue
    'communication', -- Misunderstanding with user
    'performance',   -- Performance issue caused
    'security',      -- Security vulnerability
    'testing',       -- Testing gap
    'design',        -- Design flaw
    'judgment',      -- Judgment error (score too high/low)
    'other'          -- Other lessons
  )),

  -- The mistake and correction
  mistake           TEXT NOT NULL,           -- What went wrong
  correction        TEXT NOT NULL,           -- What was done to fix it
  prevention        TEXT,                    -- How to prevent in future

  -- Severity assessment
  severity          VARCHAR(20) DEFAULT 'medium' CHECK (severity IN (
    'low',           -- Minor issue, easily recoverable
    'medium',        -- Moderate impact
    'high',          -- Significant impact
    'critical'       -- Major impact, should never repeat
  )),

  -- Vector embedding for semantic search
  embedding         vector(1536),

  -- Tracking recurrence
  occurrence_count  INTEGER DEFAULT 1,
  last_occurred     TIMESTAMPTZ DEFAULT NOW(),

  -- Source reference
  source_judgment_id VARCHAR(32),            -- If related to a judgment
  source_session_id  VARCHAR(255),           -- Session where learned

  created_at        TIMESTAMPTZ DEFAULT NOW()
);

-- =============================================================================
-- PHASE 2: AUTONOMY TABLES
-- =============================================================================

-- Persistent goals (objectives CYNIC pursues autonomously)
CREATE TABLE IF NOT EXISTS autonomous_goals (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           VARCHAR(255) NOT NULL,

  -- Goal classification
  goal_type         VARCHAR(100) NOT NULL CHECK (goal_type IN (
    'quality',       -- Code quality improvement
    'learning',      -- Learn from feedback
    'maintenance',   -- Code maintenance tasks
    'monitoring',    -- Monitor for issues
    'security',      -- Security scanning
    'documentation', -- Documentation updates
    'performance',   -- Performance optimization
    'custom'         -- User-defined goal
  )),

  -- Goal definition
  title             VARCHAR(500) NOT NULL,
  description       TEXT,

  -- Success criteria (array of conditions)
  success_criteria  JSONB DEFAULT '[]',      -- [{criterion, weight, met}]

  -- Progress tracking
  progress          FLOAT DEFAULT 0.0 CHECK (progress >= 0 AND progress <= 1),
  progress_notes    JSONB DEFAULT '[]',      -- Array of progress updates

  -- Lifecycle
  status            VARCHAR(50) DEFAULT 'active' CHECK (status IN (
    'active',        -- Being worked on
    'paused',        -- Temporarily paused
    'completed',     -- Successfully completed
    'abandoned',     -- Abandoned (with reason)
    'blocked'        -- Blocked by dependency
  )),

  -- Priority (0-100, higher = more important)
  priority          INTEGER DEFAULT 50 CHECK (priority >= 0 AND priority <= 100),

  -- Context and configuration
  config            JSONB DEFAULT '{}',      -- Goal-specific configuration

  -- Timing
  created_at        TIMESTAMPTZ DEFAULT NOW(),
  updated_at        TIMESTAMPTZ DEFAULT NOW(),
  completed_at      TIMESTAMPTZ,
  due_at            TIMESTAMPTZ              -- Optional deadline
);

-- Durable task queue (survives restarts)
CREATE TABLE IF NOT EXISTS autonomous_tasks (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  goal_id           UUID REFERENCES autonomous_goals(id) ON DELETE SET NULL,
  user_id           VARCHAR(255) NOT NULL,

  -- Task definition
  task_type         VARCHAR(100) NOT NULL,   -- analyze_patterns, run_tests, etc.
  payload           JSONB NOT NULL,          -- Task-specific data

  -- Execution status
  status            VARCHAR(50) DEFAULT 'pending' CHECK (status IN (
    'pending',       -- Waiting to execute
    'running',       -- Currently executing
    'completed',     -- Successfully completed
    'failed',        -- Failed (may retry)
    'retry',         -- Scheduled for retry
    'cancelled'      -- Cancelled
  )),

  -- Priority (0-100, higher = more important)
  priority          INTEGER DEFAULT 50 CHECK (priority >= 0 AND priority <= 100),

  -- Scheduling
  scheduled_for     TIMESTAMPTZ DEFAULT NOW(),
  started_at        TIMESTAMPTZ,
  completed_at      TIMESTAMPTZ,

  -- Retry handling
  retry_count       INTEGER DEFAULT 0,
  max_retries       INTEGER DEFAULT 3,
  error_message     TEXT,

  -- Result storage
  result            JSONB,

  -- Context
  created_by        VARCHAR(100) DEFAULT 'daemon',  -- daemon, user, hook, etc.
  created_at        TIMESTAMPTZ DEFAULT NOW()
);

-- Proactive notifications (delivered at session start)
CREATE TABLE IF NOT EXISTS proactive_notifications (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           VARCHAR(255) NOT NULL,

  -- Notification type
  notification_type VARCHAR(100) NOT NULL CHECK (notification_type IN (
    'insight',       -- New insight discovered
    'warning',       -- Potential issue detected
    'reminder',      -- Reminder about pending task
    'achievement',   -- Goal completed or milestone
    'pattern',       -- Pattern detected
    'suggestion',    -- Proactive suggestion
    'learning',      -- Something learned
    'question'       -- Question for user
  )),

  -- Content
  title             VARCHAR(500) NOT NULL,
  message           TEXT NOT NULL,

  -- Priority (0-100, higher = show first)
  priority          INTEGER DEFAULT 50 CHECK (priority >= 0 AND priority <= 100),

  -- Additional context
  context           JSONB DEFAULT '{}',      -- Source, related items, actions

  -- Delivery tracking
  delivered         BOOLEAN DEFAULT FALSE,
  delivered_at      TIMESTAMPTZ,

  -- Expiration
  expires_at        TIMESTAMPTZ,             -- NULL = never expires

  -- User interaction
  dismissed         BOOLEAN DEFAULT FALSE,
  dismissed_at      TIMESTAMPTZ,
  action_taken      VARCHAR(100),            -- What user did with notification

  created_at        TIMESTAMPTZ DEFAULT NOW()
);

-- =============================================================================
-- INDEXES FOR MEMORY TABLES
-- =============================================================================

-- Conversation memories indexes
CREATE INDEX IF NOT EXISTS idx_conv_mem_user ON conversation_memories(user_id);
CREATE INDEX IF NOT EXISTS idx_conv_mem_type ON conversation_memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_conv_mem_session ON conversation_memories(session_id);
CREATE INDEX IF NOT EXISTS idx_conv_mem_importance ON conversation_memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_conv_mem_created ON conversation_memories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_conv_mem_accessed ON conversation_memories(last_accessed DESC);

-- Full-text search on content
CREATE INDEX IF NOT EXISTS idx_conv_mem_fts ON conversation_memories
  USING gin(to_tsvector('english', content));

-- Vector search (if pgvector available) - using HNSW for better performance
DO $$
BEGIN
  CREATE INDEX IF NOT EXISTS idx_conv_mem_vector ON conversation_memories
    USING hnsw (embedding vector_cosine_ops);
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Vector index not created - pgvector may not be available';
END $$;

-- Architectural decisions indexes
CREATE INDEX IF NOT EXISTS idx_arch_dec_user ON architectural_decisions(user_id);
CREATE INDEX IF NOT EXISTS idx_arch_dec_project ON architectural_decisions(project_path);
CREATE INDEX IF NOT EXISTS idx_arch_dec_type ON architectural_decisions(decision_type);
CREATE INDEX IF NOT EXISTS idx_arch_dec_status ON architectural_decisions(status);
CREATE INDEX IF NOT EXISTS idx_arch_dec_created ON architectural_decisions(created_at DESC);

-- Full-text search on title and description
CREATE INDEX IF NOT EXISTS idx_arch_dec_fts ON architectural_decisions
  USING gin(to_tsvector('english', title || ' ' || COALESCE(description, '')));

-- Vector search
DO $$
BEGIN
  CREATE INDEX IF NOT EXISTS idx_arch_dec_vector ON architectural_decisions
    USING hnsw (embedding vector_cosine_ops);
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Vector index not created - pgvector may not be available';
END $$;

-- Lessons learned indexes
CREATE INDEX IF NOT EXISTS idx_lessons_user ON lessons_learned(user_id);
CREATE INDEX IF NOT EXISTS idx_lessons_cat ON lessons_learned(category);
CREATE INDEX IF NOT EXISTS idx_lessons_severity ON lessons_learned(severity);
CREATE INDEX IF NOT EXISTS idx_lessons_occurred ON lessons_learned(last_occurred DESC);

-- Full-text search on mistake and correction
CREATE INDEX IF NOT EXISTS idx_lessons_fts ON lessons_learned
  USING gin(to_tsvector('english', mistake || ' ' || correction || ' ' || COALESCE(prevention, '')));

-- Vector search
DO $$
BEGIN
  CREATE INDEX IF NOT EXISTS idx_lessons_vector ON lessons_learned
    USING hnsw (embedding vector_cosine_ops);
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'Vector index not created - pgvector may not be available';
END $$;

-- =============================================================================
-- INDEXES FOR AUTONOMY TABLES
-- =============================================================================

-- Goals indexes
CREATE INDEX IF NOT EXISTS idx_goals_user ON autonomous_goals(user_id);
CREATE INDEX IF NOT EXISTS idx_goals_status ON autonomous_goals(status);
CREATE INDEX IF NOT EXISTS idx_goals_type ON autonomous_goals(goal_type);
CREATE INDEX IF NOT EXISTS idx_goals_user_status ON autonomous_goals(user_id, status);
CREATE INDEX IF NOT EXISTS idx_goals_priority ON autonomous_goals(priority DESC);

-- Tasks indexes
CREATE INDEX IF NOT EXISTS idx_tasks_user ON autonomous_tasks(user_id);
CREATE INDEX IF NOT EXISTS idx_tasks_goal ON autonomous_tasks(goal_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON autonomous_tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_scheduled ON autonomous_tasks(scheduled_for);
CREATE INDEX IF NOT EXISTS idx_tasks_status_scheduled ON autonomous_tasks(status, scheduled_for);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON autonomous_tasks(priority DESC);

-- Notifications indexes
CREATE INDEX IF NOT EXISTS idx_notif_user ON proactive_notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notif_delivered ON proactive_notifications(delivered);
CREATE INDEX IF NOT EXISTS idx_notif_user_delivered ON proactive_notifications(user_id, delivered);
CREATE INDEX IF NOT EXISTS idx_notif_type ON proactive_notifications(notification_type);
CREATE INDEX IF NOT EXISTS idx_notif_priority ON proactive_notifications(priority DESC);
CREATE INDEX IF NOT EXISTS idx_notif_expires ON proactive_notifications(expires_at);

-- =============================================================================
-- TRIGGERS
-- =============================================================================

-- Update updated_at on architectural_decisions changes
CREATE TRIGGER arch_dec_updated_at BEFORE UPDATE ON architectural_decisions
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Update updated_at on autonomous_goals changes
CREATE TRIGGER goals_updated_at BEFORE UPDATE ON autonomous_goals
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Function to search memories with hybrid FTS + vector scoring
-- φ-weighted: 0.382 FTS + 0.618 vector (golden ratio)
CREATE OR REPLACE FUNCTION search_memories_hybrid(
  p_user_id VARCHAR(255),
  p_query TEXT,
  p_query_embedding vector(1536) DEFAULT NULL,
  p_memory_types VARCHAR[] DEFAULT ARRAY['summary', 'key_moment', 'decision', 'preference'],
  p_min_relevance FLOAT DEFAULT 0.236,  -- φ⁻³
  p_limit INTEGER DEFAULT 10
)
RETURNS TABLE (
  id UUID,
  memory_type VARCHAR(50),
  content TEXT,
  importance FLOAT,
  context JSONB,
  created_at TIMESTAMPTZ,
  combined_score FLOAT
) AS $$
DECLARE
  PHI_FTS CONSTANT FLOAT := 0.382;
  PHI_VECTOR CONSTANT FLOAT := 0.618;
BEGIN
  RETURN QUERY
  WITH fts_results AS (
    SELECT
      cm.id,
      cm.content,
      cm.memory_type,
      ts_rank(to_tsvector('english', cm.content), plainto_tsquery('english', p_query)) as fts_score
    FROM conversation_memories cm
    WHERE cm.user_id = p_user_id
      AND cm.memory_type = ANY(p_memory_types)
  ),
  vector_results AS (
    SELECT
      cm.id,
      CASE
        WHEN p_query_embedding IS NOT NULL AND cm.embedding IS NOT NULL
        THEN 1 - (cm.embedding <=> p_query_embedding)
        ELSE 0
      END as vector_score
    FROM conversation_memories cm
    WHERE cm.user_id = p_user_id
      AND cm.memory_type = ANY(p_memory_types)
  )
  SELECT
    cm.id,
    cm.memory_type,
    cm.content,
    cm.importance,
    cm.context,
    cm.created_at,
    (COALESCE(f.fts_score, 0) * PHI_FTS + COALESCE(v.vector_score, 0) * PHI_VECTOR) as combined_score
  FROM conversation_memories cm
  LEFT JOIN fts_results f ON cm.id = f.id
  LEFT JOIN vector_results v ON cm.id = v.id
  WHERE cm.user_id = p_user_id
    AND cm.memory_type = ANY(p_memory_types)
    AND (COALESCE(f.fts_score, 0) * PHI_FTS + COALESCE(v.vector_score, 0) * PHI_VECTOR) >= p_min_relevance
  ORDER BY combined_score DESC
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to get pending tasks for execution
CREATE OR REPLACE FUNCTION get_pending_tasks(
  p_limit INTEGER DEFAULT 10
)
RETURNS TABLE (
  id UUID,
  goal_id UUID,
  user_id VARCHAR(255),
  task_type VARCHAR(100),
  payload JSONB,
  priority INTEGER,
  scheduled_for TIMESTAMPTZ,
  retry_count INTEGER
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    t.id,
    t.goal_id,
    t.user_id,
    t.task_type,
    t.payload,
    t.priority,
    t.scheduled_for,
    t.retry_count
  FROM autonomous_tasks t
  WHERE t.status = 'pending'
    AND t.scheduled_for <= NOW()
  ORDER BY t.priority DESC, t.scheduled_for ASC
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to get undelivered notifications for a user
CREATE OR REPLACE FUNCTION get_pending_notifications(
  p_user_id VARCHAR(255),
  p_limit INTEGER DEFAULT 5
)
RETURNS TABLE (
  id UUID,
  notification_type VARCHAR(100),
  title VARCHAR(500),
  message TEXT,
  priority INTEGER,
  context JSONB,
  created_at TIMESTAMPTZ
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    n.id,
    n.notification_type,
    n.title,
    n.message,
    n.priority,
    n.context,
    n.created_at
  FROM proactive_notifications n
  WHERE n.user_id = p_user_id
    AND n.delivered = FALSE
    AND n.dismissed = FALSE
    AND (n.expires_at IS NULL OR n.expires_at > NOW())
  ORDER BY n.priority DESC, n.created_at DESC
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to record memory access (updates last_accessed and access_count)
CREATE OR REPLACE FUNCTION record_memory_access(p_memory_ids UUID[])
RETURNS INTEGER AS $$
DECLARE
  updated_count INTEGER;
BEGIN
  UPDATE conversation_memories
  SET
    last_accessed = NOW(),
    access_count = access_count + 1
  WHERE id = ANY(p_memory_ids);

  GET DIAGNOSTICS updated_count = ROW_COUNT;
  RETURN updated_count;
END;
$$ LANGUAGE plpgsql;

-- Function to increment lesson occurrence
CREATE OR REPLACE FUNCTION record_lesson_occurrence(p_lesson_id UUID)
RETURNS VOID AS $$
BEGIN
  UPDATE lessons_learned
  SET
    occurrence_count = occurrence_count + 1,
    last_occurred = NOW()
  WHERE id = p_lesson_id;
END;
$$ LANGUAGE plpgsql;

-- Function to update goal progress
CREATE OR REPLACE FUNCTION update_goal_progress(
  p_goal_id UUID,
  p_progress FLOAT,
  p_note TEXT DEFAULT NULL
)
RETURNS VOID AS $$
BEGIN
  UPDATE autonomous_goals
  SET
    progress = LEAST(p_progress, 1.0),
    progress_notes = CASE
      WHEN p_note IS NOT NULL
      THEN progress_notes || jsonb_build_object(
        'timestamp', NOW(),
        'progress', p_progress,
        'note', p_note
      )
      ELSE progress_notes
    END,
    status = CASE
      WHEN p_progress >= 1.0 THEN 'completed'
      ELSE status
    END,
    completed_at = CASE
      WHEN p_progress >= 1.0 AND completed_at IS NULL THEN NOW()
      ELSE completed_at
    END,
    updated_at = NOW()
  WHERE id = p_goal_id;
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old/expired data
CREATE OR REPLACE FUNCTION cleanup_total_memory()
RETURNS TABLE (
  notifications_deleted INTEGER,
  tasks_deleted INTEGER
) AS $$
DECLARE
  notif_count INTEGER;
  task_count INTEGER;
BEGIN
  -- Delete expired notifications that were never delivered
  DELETE FROM proactive_notifications
  WHERE expires_at < NOW() AND delivered = FALSE;
  GET DIAGNOSTICS notif_count = ROW_COUNT;

  -- Delete old completed/cancelled tasks (keep 30 days)
  DELETE FROM autonomous_tasks
  WHERE status IN ('completed', 'cancelled')
    AND completed_at < NOW() - INTERVAL '30 days';
  GET DIAGNOSTICS task_count = ROW_COUNT;

  notifications_deleted := notif_count;
  tasks_deleted := task_count;
  RETURN NEXT;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('015_total_memory')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ constants used:
-- PHI = 1.618033988749895
-- PHI_INV = 0.618033988749895 (vector weight)
-- PHI_INV_2 = 0.381966011250105 (FTS weight)
-- PHI_INV_3 = 0.236067977499790 (min relevance threshold)

-- CYNIC remembers. CYNIC acts. Loyal to truth, not to comfort.
