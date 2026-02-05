-- =============================================================================
-- Migration 028: DPO Learning Pipeline
-- =============================================================================
-- Implements Direct Preference Optimization for CYNIC's routing decisions.
-- Converts feedback → preference pairs → weight updates.
--
-- "Le chien apprend de ses erreurs." - φ-aligned continuous improvement
-- =============================================================================

-- =============================================================================
-- PREFERENCE PAIRS TABLE
-- =============================================================================
-- Stores preference pairs for DPO training.
-- Each pair represents: "chosen response was better than rejected response"

CREATE TABLE IF NOT EXISTS preference_pairs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Preference pair data
    chosen          JSONB NOT NULL,   -- The preferred response/action
    rejected        JSONB NOT NULL,   -- The rejected response/action

    -- Context when pair was created
    context         JSONB NOT NULL DEFAULT '{}',
    context_type    VARCHAR(50),      -- 'code', 'review', 'explain', etc.
    task_type       VARCHAR(50),      -- 'Write', 'Edit', 'Bash', etc.

    -- Source feedback
    feedback_ids    UUID[] NOT NULL DEFAULT '{}',
    confidence      DECIMAL(5,4) DEFAULT 0.618,  -- φ⁻¹ default

    -- Processing status
    processed       BOOLEAN DEFAULT FALSE,
    processed_at    TIMESTAMPTZ,
    batch_id        VARCHAR(50),

    -- Metadata
    service_id      VARCHAR(50) DEFAULT 'default',
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- =============================================================================
-- ROUTING WEIGHTS TABLE
-- =============================================================================
-- Persists learned routing weights per Dog per context.
-- Updated by DPO optimizer based on preference pairs.

CREATE TABLE IF NOT EXISTS routing_weights (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id      VARCHAR(50) DEFAULT 'default',

    -- Routing target
    dog_name        VARCHAR(50) NOT NULL,     -- 'Analyst', 'Guardian', etc.
    context_type    VARCHAR(50) NOT NULL,     -- 'code', 'security', 'review'

    -- Weight data
    weight          DECIMAL(8,6) NOT NULL DEFAULT 0.5,  -- [0, 1] routing probability
    base_weight     DECIMAL(8,6) DEFAULT 0.5,           -- Initial weight before learning
    confidence      DECIMAL(5,4) DEFAULT 0.236,         -- φ⁻³ initial uncertainty

    -- Learning stats
    episode_count   INTEGER DEFAULT 0,
    positive_count  INTEGER DEFAULT 0,   -- Times chosen over alternatives
    negative_count  INTEGER DEFAULT 0,   -- Times rejected
    last_update     TIMESTAMPTZ DEFAULT NOW(),

    -- EWC++ Fisher score (prevents catastrophic forgetting)
    fisher_score    DECIMAL(10,6) DEFAULT 0.0,
    fisher_updated  TIMESTAMPTZ,

    -- Metadata
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    -- Unique constraint: one weight per dog per context per service
    UNIQUE(service_id, dog_name, context_type)
);

-- =============================================================================
-- DPO OPTIMIZER STATE TABLE
-- =============================================================================
-- Tracks DPO optimization runs and convergence.

CREATE TABLE IF NOT EXISTS dpo_optimizer_state (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id      VARCHAR(50) UNIQUE NOT NULL DEFAULT 'default',

    -- Learning parameters (φ-aligned)
    learning_rate   DECIMAL(8,6) DEFAULT 0.236,   -- φ⁻³
    beta            DECIMAL(5,4) DEFAULT 0.1,     -- KL divergence penalty
    regularization  DECIMAL(5,4) DEFAULT 0.618,   -- φ⁻¹ EWC strength

    -- Training state
    epoch           INTEGER DEFAULT 0,
    total_pairs_processed INTEGER DEFAULT 0,
    last_loss       DECIMAL(10,6),
    best_loss       DECIMAL(10,6),
    convergence_count INTEGER DEFAULT 0,

    -- Scheduling
    last_run        TIMESTAMPTZ,
    next_scheduled  TIMESTAMPTZ,
    run_interval_hours INTEGER DEFAULT 24,        -- Daily by default

    -- Stats
    stats           JSONB DEFAULT '{"runs": 0, "convergence_epochs": [], "loss_history": []}',

    -- Metadata
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Create default entry
INSERT INTO dpo_optimizer_state (service_id)
VALUES ('default')
ON CONFLICT (service_id) DO NOTHING;

-- =============================================================================
-- CALIBRATION TRACKING TABLE
-- =============================================================================
-- Monitors prediction accuracy and calibration drift.

CREATE TABLE IF NOT EXISTS calibration_tracking (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id      VARCHAR(50) DEFAULT 'default',

    -- Prediction data
    predicted_outcome VARCHAR(50),    -- What CYNIC predicted
    actual_outcome    VARCHAR(50),    -- What actually happened
    predicted_confidence DECIMAL(5,4),
    context_type     VARCHAR(50),

    -- Bucket for calibration curve
    confidence_bucket INTEGER,        -- 0-9 representing 0-10%, 10-20%, etc.

    -- Metadata
    prediction_id   VARCHAR(50),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- =============================================================================
-- INDEXES
-- =============================================================================

-- Preference Pairs
CREATE INDEX IF NOT EXISTS idx_preference_pairs_unprocessed
ON preference_pairs(processed, created_at)
WHERE processed = FALSE;

CREATE INDEX IF NOT EXISTS idx_preference_pairs_context
ON preference_pairs(context_type);

CREATE INDEX IF NOT EXISTS idx_preference_pairs_batch
ON preference_pairs(batch_id)
WHERE batch_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_preference_pairs_created
ON preference_pairs(created_at DESC);

-- Routing Weights
CREATE INDEX IF NOT EXISTS idx_routing_weights_dog
ON routing_weights(dog_name);

CREATE INDEX IF NOT EXISTS idx_routing_weights_context
ON routing_weights(context_type);

CREATE INDEX IF NOT EXISTS idx_routing_weights_lookup
ON routing_weights(service_id, context_type);

-- Calibration
CREATE INDEX IF NOT EXISTS idx_calibration_bucket
ON calibration_tracking(confidence_bucket);

CREATE INDEX IF NOT EXISTS idx_calibration_service_recent
ON calibration_tracking(service_id, created_at DESC);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Function to create a preference pair from feedback
CREATE OR REPLACE FUNCTION create_preference_pair(
    p_chosen JSONB,
    p_rejected JSONB,
    p_context JSONB,
    p_context_type VARCHAR,
    p_task_type VARCHAR,
    p_feedback_ids UUID[],
    p_confidence DECIMAL DEFAULT 0.618
)
RETURNS UUID AS $$
DECLARE
    v_pair_id UUID;
BEGIN
    INSERT INTO preference_pairs (chosen, rejected, context, context_type, task_type, feedback_ids, confidence)
    VALUES (p_chosen, p_rejected, p_context, p_context_type, p_task_type, p_feedback_ids, p_confidence)
    RETURNING id INTO v_pair_id;

    RETURN v_pair_id;
END;
$$ LANGUAGE plpgsql;

-- Function to update routing weight with DPO gradient
CREATE OR REPLACE FUNCTION update_routing_weight(
    p_service_id VARCHAR,
    p_dog_name VARCHAR,
    p_context_type VARCHAR,
    p_delta DECIMAL,
    p_is_positive BOOLEAN
)
RETURNS routing_weights AS $$
DECLARE
    v_result routing_weights;
    v_learning_rate DECIMAL;
    v_fisher DECIMAL;
    v_effective_delta DECIMAL;
BEGIN
    -- Get learning rate from optimizer state
    SELECT learning_rate INTO v_learning_rate
    FROM dpo_optimizer_state
    WHERE service_id = p_service_id;

    v_learning_rate := COALESCE(v_learning_rate, 0.236);

    -- Upsert the weight
    INSERT INTO routing_weights (service_id, dog_name, context_type, weight, episode_count, positive_count, negative_count)
    VALUES (p_service_id, p_dog_name, p_context_type, 0.5, 0, 0, 0)
    ON CONFLICT (service_id, dog_name, context_type) DO NOTHING;

    -- Get current Fisher score for EWC regularization
    SELECT fisher_score INTO v_fisher
    FROM routing_weights
    WHERE service_id = p_service_id AND dog_name = p_dog_name AND context_type = p_context_type;

    v_fisher := COALESCE(v_fisher, 0.0);

    -- Calculate effective delta with EWC penalty: delta / (1 + fisher)
    v_effective_delta := p_delta * v_learning_rate / (1.0 + v_fisher);

    -- Update weight with clipping to [0, 1]
    UPDATE routing_weights
    SET weight = GREATEST(0.0, LEAST(1.0, weight + v_effective_delta)),
        episode_count = episode_count + 1,
        positive_count = positive_count + CASE WHEN p_is_positive THEN 1 ELSE 0 END,
        negative_count = negative_count + CASE WHEN NOT p_is_positive THEN 1 ELSE 0 END,
        last_update = NOW(),
        updated_at = NOW()
    WHERE service_id = p_service_id AND dog_name = p_dog_name AND context_type = p_context_type
    RETURNING * INTO v_result;

    RETURN v_result;
END;
$$ LANGUAGE plpgsql;

-- Function to get calibration curve data
CREATE OR REPLACE FUNCTION get_calibration_curve(
    p_service_id VARCHAR DEFAULT 'default',
    p_days INTEGER DEFAULT 7
)
RETURNS TABLE (
    bucket INTEGER,
    predicted_rate DECIMAL,
    actual_rate DECIMAL,
    sample_count BIGINT,
    calibration_error DECIMAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        ct.confidence_bucket as bucket,
        (ct.confidence_bucket * 10 + 5)::DECIMAL / 100 as predicted_rate,
        SUM(CASE WHEN ct.predicted_outcome = ct.actual_outcome THEN 1 ELSE 0 END)::DECIMAL /
            NULLIF(COUNT(*), 0) as actual_rate,
        COUNT(*) as sample_count,
        ABS((ct.confidence_bucket * 10 + 5)::DECIMAL / 100 -
            SUM(CASE WHEN ct.predicted_outcome = ct.actual_outcome THEN 1 ELSE 0 END)::DECIMAL /
            NULLIF(COUNT(*), 0)) as calibration_error
    FROM calibration_tracking ct
    WHERE ct.service_id = p_service_id
      AND ct.created_at > NOW() - (p_days || ' days')::INTERVAL
    GROUP BY ct.confidence_bucket
    ORDER BY ct.confidence_bucket;
END;
$$ LANGUAGE plpgsql;

-- Function to get DPO training stats
CREATE OR REPLACE FUNCTION get_dpo_stats(
    p_service_id VARCHAR DEFAULT 'default'
)
RETURNS TABLE (
    total_pairs BIGINT,
    unprocessed_pairs BIGINT,
    total_weights BIGINT,
    avg_weight DECIMAL,
    avg_fisher DECIMAL,
    epochs INTEGER,
    last_loss DECIMAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        (SELECT COUNT(*) FROM preference_pairs WHERE service_id = p_service_id),
        (SELECT COUNT(*) FROM preference_pairs WHERE service_id = p_service_id AND processed = FALSE),
        (SELECT COUNT(*) FROM routing_weights WHERE service_id = p_service_id),
        (SELECT AVG(weight) FROM routing_weights WHERE service_id = p_service_id),
        (SELECT AVG(fisher_score) FROM routing_weights WHERE service_id = p_service_id),
        (SELECT s.epoch FROM dpo_optimizer_state s WHERE s.service_id = p_service_id),
        (SELECT s.last_loss FROM dpo_optimizer_state s WHERE s.service_id = p_service_id);
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old calibration data (keep last 30 days)
CREATE OR REPLACE FUNCTION cleanup_calibration_data(
    p_service_id VARCHAR DEFAULT 'default',
    p_days INTEGER DEFAULT 30
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM calibration_tracking
    WHERE service_id = p_service_id
      AND created_at < NOW() - (p_days || ' days')::INTERVAL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- UPDATED_AT TRIGGERS
-- =============================================================================

DROP TRIGGER IF EXISTS preference_pairs_updated_at ON preference_pairs;
CREATE TRIGGER preference_pairs_updated_at
    BEFORE UPDATE ON preference_pairs
    FOR EACH ROW
    EXECUTE FUNCTION update_qlearning_updated_at();

DROP TRIGGER IF EXISTS routing_weights_updated_at ON routing_weights;
CREATE TRIGGER routing_weights_updated_at
    BEFORE UPDATE ON routing_weights
    FOR EACH ROW
    EXECUTE FUNCTION update_qlearning_updated_at();

DROP TRIGGER IF EXISTS dpo_optimizer_state_updated_at ON dpo_optimizer_state;
CREATE TRIGGER dpo_optimizer_state_updated_at
    BEFORE UPDATE ON dpo_optimizer_state
    FOR EACH ROW
    EXECUTE FUNCTION update_qlearning_updated_at();

-- =============================================================================
-- INITIALIZE DEFAULT ROUTING WEIGHTS (11 Dogs × common contexts)
-- =============================================================================

DO $$
DECLARE
    dogs TEXT[] := ARRAY['CYNIC', 'Guardian', 'Analyst', 'Scholar', 'Sage', 'Architect', 'Oracle', 'Scout', 'Deployer', 'Janitor', 'Cartographer'];
    contexts TEXT[] := ARRAY['code', 'security', 'review', 'explain', 'debug', 'refactor', 'document', 'test', 'deploy', 'general'];
    dog TEXT;
    ctx TEXT;
BEGIN
    FOREACH dog IN ARRAY dogs LOOP
        FOREACH ctx IN ARRAY contexts LOOP
            INSERT INTO routing_weights (service_id, dog_name, context_type, weight, base_weight)
            VALUES ('default', dog, ctx, 0.5, 0.5)
            ON CONFLICT (service_id, dog_name, context_type) DO NOTHING;
        END LOOP;
    END LOOP;
END $$;

-- =============================================================================
-- COMMENTS
-- =============================================================================

COMMENT ON TABLE preference_pairs IS
'Stores preference pairs for DPO training - chosen > rejected pairs from feedback';

COMMENT ON TABLE routing_weights IS
'Learned routing weights per Dog per context - updated by DPO optimizer';

COMMENT ON TABLE dpo_optimizer_state IS
'DPO optimizer state and scheduling - tracks training progress';

COMMENT ON TABLE calibration_tracking IS
'Calibration monitoring - tracks prediction vs actual for drift detection';

COMMENT ON FUNCTION create_preference_pair IS
'Creates a preference pair from feedback data';

COMMENT ON FUNCTION update_routing_weight IS
'Updates a routing weight with DPO gradient and EWC regularization';

COMMENT ON FUNCTION get_calibration_curve IS
'Returns calibration curve data for specified time window';

COMMENT ON FUNCTION get_dpo_stats IS
'Returns DPO training statistics';

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('028_dpo_learning')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- "φ guides learning. Le chien s'améliore continuellement."
-- =============================================================================
