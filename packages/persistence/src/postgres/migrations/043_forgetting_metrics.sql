-- Migration 043: Catastrophic Forgetting Detection (LV-4)
-- BWT/FWT tracking for continual learning validation

-- Task baselines: Initial performance per task type
CREATE TABLE IF NOT EXISTS forgetting_baselines (
  id SERIAL PRIMARY KEY,
  task_type VARCHAR(100) UNIQUE NOT NULL,
  baseline_accuracy NUMERIC(5,4) NOT NULL,
  sample_count INTEGER NOT NULL DEFAULT 1,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_forgetting_baselines_task ON forgetting_baselines(task_type);

-- Individual judgment records for BWT calculation
CREATE TABLE IF NOT EXISTS forgetting_judgments (
  id SERIAL PRIMARY KEY,
  task_type VARCHAR(100) NOT NULL,
  judgment_id VARCHAR(100) NOT NULL,
  accuracy NUMERIC(5,4) NOT NULL,
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_forgetting_judgments_task ON forgetting_judgments(task_type);
CREATE INDEX idx_forgetting_judgments_created ON forgetting_judgments(created_at DESC);

-- BWT/FWT metrics snapshots
CREATE TABLE IF NOT EXISTS forgetting_metrics (
  id SERIAL PRIMARY KEY,
  average_bwt NUMERIC(6,4) NOT NULL,
  task_count INTEGER NOT NULL,
  catastrophic_count INTEGER NOT NULL DEFAULT 0,
  catastrophic_tasks JSONB,
  confidence NUMERIC(5,4),
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_forgetting_metrics_created ON forgetting_metrics(created_at DESC);
CREATE INDEX idx_forgetting_metrics_catastrophic ON forgetting_metrics(catastrophic_count) WHERE catastrophic_count > 0;

-- Catastrophic forgetting alerts
CREATE TABLE IF NOT EXISTS forgetting_alerts (
  id SERIAL PRIMARY KEY,
  alert_type VARCHAR(50) NOT NULL,
  catastrophic_count INTEGER NOT NULL,
  catastrophic_tasks JSONB NOT NULL,
  average_bwt NUMERIC(6,4) NOT NULL,
  acknowledged BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_forgetting_alerts_type ON forgetting_alerts(alert_type);
CREATE INDEX idx_forgetting_alerts_unack ON forgetting_alerts(acknowledged) WHERE acknowledged = FALSE;
CREATE INDEX idx_forgetting_alerts_created ON forgetting_alerts(created_at DESC);

COMMENT ON TABLE forgetting_baselines IS 'C6.5: Task baselines for BWT calculation';
COMMENT ON TABLE forgetting_judgments IS 'C6.5: Individual judgment records for continual learning validation';
COMMENT ON TABLE forgetting_metrics IS 'C6.5: BWT/FWT metrics snapshots';
COMMENT ON TABLE forgetting_alerts IS 'C6.5: Catastrophic forgetting alerts (BWT < -0.1)';
