-- Migration 042: TD Error Tracker (LV-1: Learning Validation)
--
-- Tracks Temporal Difference errors to detect:
-- 1. Convergence (learning complete, Q-values stable)
-- 2. Drift (distribution shift, need to re-learn)
--
-- TD-Error = |Q(s,a) - (r + γ * max Q(s',a'))|

CREATE TABLE IF NOT EXISTS td_error_tracker (
  id BIGSERIAL PRIMARY KEY,
  service_id TEXT NOT NULL DEFAULT 'default',
  
  -- Q-Learning context
  state TEXT NOT NULL,                    -- State features (serialized)
  action TEXT NOT NULL,                   -- Action (dog) taken
  
  -- TD-Error metrics
  td_error DOUBLE PRECISION NOT NULL,     -- |target - currentQ|
  rolling_avg_td_error DOUBLE PRECISION,  -- Rolling avg TD-Error
  
  -- Convergence/drift flags
  is_converged BOOLEAN DEFAULT FALSE,     -- Below convergence threshold
  is_drift BOOLEAN DEFAULT FALSE,         -- Drift detected
  
  -- Q-Learning values
  current_q DOUBLE PRECISION,             -- Q(s,a) before update
  target DOUBLE PRECISION,                -- r + γ * max Q(s',a')
  new_q DOUBLE PRECISION,                 -- Q(s,a) after update
  reward DOUBLE PRECISION,                -- Immediate reward
  
  -- Metadata
  created_at TIMESTAMPTZ DEFAULT NOW(),
  
  -- Indexes for analysis
  INDEX idx_td_error_service (service_id),
  INDEX idx_td_error_convergence (is_converged, rolling_avg_td_error),
  INDEX idx_td_error_drift (is_drift, created_at),
  INDEX idx_td_error_state_action (state, action),
  INDEX idx_td_error_created (created_at DESC)
);

COMMENT ON TABLE td_error_tracker IS 'LV-1: Temporal Difference error tracking for Q-Learning convergence/drift detection';
COMMENT ON COLUMN td_error_tracker.td_error IS 'Absolute difference between target and current Q-value';
COMMENT ON COLUMN td_error_tracker.rolling_avg_td_error IS 'Rolling average TD-Error over last N updates';
COMMENT ON COLUMN td_error_tracker.is_converged IS 'TRUE when rolling avg TD-Error < convergence threshold (0.1)';
COMMENT ON COLUMN td_error_tracker.is_drift IS 'TRUE when TD-Error spikes above drift threshold (0.3) after convergence';
