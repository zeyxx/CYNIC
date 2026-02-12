-- =============================================================================
-- Migration 039: Q-Learning EWC (Elastic Weight Consolidation)
-- =============================================================================
--
-- LV-5: Prevent catastrophic forgetting in Q-learning via EWC.
--
-- Core Concept:
--   Important Q-values (high Fisher Information) are protected from
--   being overwritten during continual learning.
--
-- Formula:
--   Q(s,a) <- Q(s,a) + alpha[r + gamma max Q(s',a') - Q(s,a)] - lambda * F(s,a) * [Q(s,a) - Q_old(s,a)]
--                     Standard Q-update                         EWC penalty
--
-- Where:
--   F(s,a) = Fisher Information (importance of this Q-value)
--   Q_old(s,a) = Consolidated Q-value from previous task
--   lambda = EWC strength (default: 0.1)
--
-- "Le chien se souvient des chemins importants." - CYNIC
--
-- =============================================================================

-- =============================================================================
-- ADD EWC FIELDS TO Q-LEARNING STATE
-- =============================================================================

-- Store consolidated Q-tables (snapshots after task completion)
ALTER TABLE qlearning_state
ADD COLUMN IF NOT EXISTS consolidated_q_table JSONB DEFAULT NULL;

COMMENT ON COLUMN qlearning_state.consolidated_q_table IS
  'Snapshot of Q-table after task consolidation. Used as anchor for EWC penalty.';

-- Store Fisher Information Matrix
ALTER TABLE qlearning_state
ADD COLUMN IF NOT EXISTS fisher_matrix JSONB DEFAULT '{}';

COMMENT ON COLUMN qlearning_state.fisher_matrix IS
  'Fisher Information Matrix: {stateKey: {action: importance}}. Tracks weight importance.';

-- EWC configuration
ALTER TABLE qlearning_state
ADD COLUMN IF NOT EXISTS ewc_lambda DECIMAL(5,4) DEFAULT 0.1;

COMMENT ON COLUMN qlearning_state.ewc_lambda IS
  'EWC strength parameter (lambda). Higher = stronger protection of old knowledge.';

-- Track consolidation events
ALTER TABLE qlearning_state
ADD COLUMN IF NOT EXISTS last_consolidation_at TIMESTAMPTZ DEFAULT NULL;

ALTER TABLE qlearning_state
ADD COLUMN IF NOT EXISTS consolidation_count INTEGER DEFAULT 0;

-- =============================================================================
-- Q-LEARNING TASK TABLE
-- =============================================================================
-- Tracks distinct learning tasks for multi-task learning.

CREATE TABLE IF NOT EXISTS qlearning_tasks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id         VARCHAR(50) UNIQUE NOT NULL,
    service_id      VARCHAR(50) DEFAULT 'default',

    -- Task metadata
    task_type       VARCHAR(50) NOT NULL,
    description     TEXT,

    -- Learning progress
    episodes_count  INTEGER DEFAULT 0,
    avg_reward      DECIMAL(5,4),
    convergence_status VARCHAR(20) DEFAULT 'learning',

    -- EWC consolidation
    consolidated    BOOLEAN DEFAULT FALSE,
    consolidated_at TIMESTAMPTZ,
    fisher_computed BOOLEAN DEFAULT FALSE,

    -- Timing
    first_seen_at   TIMESTAMPTZ DEFAULT NOW(),
    last_seen_at    TIMESTAMPTZ DEFAULT NOW(),
    
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_qlearning_tasks_service
ON qlearning_tasks(service_id);

CREATE INDEX IF NOT EXISTS idx_qlearning_tasks_type
ON qlearning_tasks(task_type);

CREATE INDEX IF NOT EXISTS idx_qlearning_tasks_consolidated
ON qlearning_tasks(consolidated, task_type);

COMMENT ON TABLE qlearning_tasks IS
  'Tracks distinct Q-learning tasks for multi-task continual learning with EWC.';

-- =============================================================================
-- EWC CONSOLIDATION HISTORY
-- =============================================================================

CREATE TABLE IF NOT EXISTS qlearning_ewc_history (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    consolidation_id    VARCHAR(50) UNIQUE NOT NULL,
    service_id          VARCHAR(50) DEFAULT 'default',

    -- Task being consolidated
    task_id             VARCHAR(50),
    task_type           VARCHAR(50),

    -- Fisher computation
    fisher_entries      INTEGER DEFAULT 0,
    avg_fisher          DECIMAL(8,6),
    max_fisher          DECIMAL(8,6),

    -- Q-table snapshot stats
    q_states            INTEGER DEFAULT 0,
    q_values_total      INTEGER DEFAULT 0,

    -- Performance metrics
    pre_consolidation_accuracy  DECIMAL(5,4),
    post_consolidation_accuracy DECIMAL(5,4),
    
    -- EWC parameters used
    ewc_lambda          DECIMAL(5,4) DEFAULT 0.1,

    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_qlearning_ewc_service
ON qlearning_ewc_history(service_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_qlearning_ewc_task
ON qlearning_ewc_history(task_id);

-- =============================================================================
-- FISHER INFORMATION TRACKING
-- =============================================================================

CREATE TABLE IF NOT EXISTS qlearning_fisher_gradients (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id      VARCHAR(50) DEFAULT 'default',

    -- State-Action pair
    state_key       TEXT NOT NULL,
    action          VARCHAR(50) NOT NULL,

    -- Gradient statistics
    gradient_sum    DECIMAL(10,6) DEFAULT 0.0,
    gradient_sq_sum DECIMAL(12,6) DEFAULT 0.0,
    update_count    INTEGER DEFAULT 0,

    -- Fisher Information
    fisher_value    DECIMAL(8,6) DEFAULT 0.0,
    last_fisher_update TIMESTAMPTZ,

    -- Metadata
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(service_id, state_key, action)
);

CREATE INDEX IF NOT EXISTS idx_qlearning_fisher_service
ON qlearning_fisher_gradients(service_id, state_key);

CREATE INDEX IF NOT EXISTS idx_qlearning_fisher_value
ON qlearning_fisher_gradients(fisher_value DESC)
WHERE fisher_value > 0;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('039_qlearning_ewc')
ON CONFLICT (name) DO NOTHING;
