-- Migration: 025_telemetry
-- Telemetry storage for usage stats, frictions, and benchmarking
-- "φ mesure tout, φ apprend de tout"

-- Telemetry snapshots (periodic exports)
CREATE TABLE IF NOT EXISTS telemetry_snapshots (
  id SERIAL PRIMARY KEY,
  session_id VARCHAR(64) NOT NULL,
  data JSONB NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_telemetry_snapshots_session
  ON telemetry_snapshots(session_id);
CREATE INDEX IF NOT EXISTS idx_telemetry_snapshots_created
  ON telemetry_snapshots(created_at DESC);

-- Frictions (errors, failures, timeouts)
CREATE TABLE IF NOT EXISTS frictions (
  id SERIAL PRIMARY KEY,
  session_id VARCHAR(64) NOT NULL,
  name VARCHAR(128) NOT NULL,
  severity VARCHAR(16) NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
  category VARCHAR(32),
  details JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_frictions_session
  ON frictions(session_id);
CREATE INDEX IF NOT EXISTS idx_frictions_severity
  ON frictions(severity);
CREATE INDEX IF NOT EXISTS idx_frictions_category
  ON frictions(category);
CREATE INDEX IF NOT EXISTS idx_frictions_created
  ON frictions(created_at DESC);

-- Aggregated metrics (hourly rollups)
CREATE TABLE IF NOT EXISTS telemetry_hourly (
  id SERIAL PRIMARY KEY,
  hour TIMESTAMPTZ NOT NULL,
  metric_name VARCHAR(128) NOT NULL,
  labels JSONB DEFAULT '{}',
  count BIGINT DEFAULT 0,
  sum DOUBLE PRECISION DEFAULT 0,
  min DOUBLE PRECISION,
  max DOUBLE PRECISION,
  avg DOUBLE PRECISION,
  p50 DOUBLE PRECISION,
  p95 DOUBLE PRECISION,
  p99 DOUBLE PRECISION,
  UNIQUE (hour, metric_name, labels)
);

CREATE INDEX IF NOT EXISTS idx_telemetry_hourly_hour
  ON telemetry_hourly(hour DESC);
CREATE INDEX IF NOT EXISTS idx_telemetry_hourly_metric
  ON telemetry_hourly(metric_name);

-- LLM usage tracking (for cost analysis and optimization)
CREATE TABLE IF NOT EXISTS llm_usage (
  id SERIAL PRIMARY KEY,
  session_id VARCHAR(64) NOT NULL,
  model VARCHAR(64) NOT NULL,
  input_tokens INTEGER DEFAULT 0,
  output_tokens INTEGER DEFAULT 0,
  latency_ms INTEGER,
  cached BOOLEAN DEFAULT FALSE,
  success BOOLEAN DEFAULT TRUE,
  error TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_usage_session
  ON llm_usage(session_id);
CREATE INDEX IF NOT EXISTS idx_llm_usage_model
  ON llm_usage(model);
CREATE INDEX IF NOT EXISTS idx_llm_usage_created
  ON llm_usage(created_at DESC);

-- Judgment tracking (for quality analysis)
CREATE TABLE IF NOT EXISTS judgment_metrics (
  id SERIAL PRIMARY KEY,
  session_id VARCHAR(64) NOT NULL,
  verdict VARCHAR(16) NOT NULL,
  q_score SMALLINT,
  confidence REAL,
  dimensions JSONB DEFAULT '{}',
  latency_ms INTEGER,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_judgment_metrics_session
  ON judgment_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_judgment_metrics_verdict
  ON judgment_metrics(verdict);
CREATE INDEX IF NOT EXISTS idx_judgment_metrics_created
  ON judgment_metrics(created_at DESC);

-- Tool usage tracking
CREATE TABLE IF NOT EXISTS tool_usage (
  id SERIAL PRIMARY KEY,
  session_id VARCHAR(64) NOT NULL,
  tool_name VARCHAR(64) NOT NULL,
  success BOOLEAN DEFAULT TRUE,
  latency_ms INTEGER,
  error TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tool_usage_session
  ON tool_usage(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_usage_tool
  ON tool_usage(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_usage_created
  ON tool_usage(created_at DESC);

-- Session summary (one row per session)
CREATE TABLE IF NOT EXISTS session_summary (
  session_id VARCHAR(64) PRIMARY KEY,
  start_time TIMESTAMPTZ NOT NULL,
  end_time TIMESTAMPTZ,
  duration_ms BIGINT,
  action_count INTEGER DEFAULT 0,
  event_count INTEGER DEFAULT 0,
  error_count INTEGER DEFAULT 0,
  llm_calls INTEGER DEFAULT 0,
  llm_tokens_in INTEGER DEFAULT 0,
  llm_tokens_out INTEGER DEFAULT 0,
  judgments INTEGER DEFAULT 0,
  avg_q_score REAL,
  frictions INTEGER DEFAULT 0,
  patterns_detected INTEGER DEFAULT 0,
  metadata JSONB DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_session_summary_start
  ON session_summary(start_time DESC);

-- Helper view: Recent friction summary
CREATE OR REPLACE VIEW recent_frictions AS
SELECT
  date_trunc('hour', created_at) as hour,
  severity,
  category,
  COUNT(*) as count
FROM frictions
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY 1, 2, 3
ORDER BY 1 DESC, 4 DESC;

-- Helper view: Hourly LLM costs
CREATE OR REPLACE VIEW hourly_llm_usage AS
SELECT
  date_trunc('hour', created_at) as hour,
  model,
  COUNT(*) as calls,
  SUM(input_tokens) as input_tokens,
  SUM(output_tokens) as output_tokens,
  AVG(latency_ms)::INTEGER as avg_latency_ms,
  COUNT(*) FILTER (WHERE cached) as cache_hits,
  COUNT(*) FILTER (WHERE NOT success) as errors
FROM llm_usage
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY 1, 2
ORDER BY 1 DESC, 3 DESC;

-- Helper view: Judgment quality over time
CREATE OR REPLACE VIEW judgment_quality AS
SELECT
  date_trunc('hour', created_at) as hour,
  verdict,
  COUNT(*) as count,
  AVG(q_score)::INTEGER as avg_q_score,
  AVG(confidence)::REAL as avg_confidence,
  AVG(latency_ms)::INTEGER as avg_latency_ms
FROM judgment_metrics
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY 1, 2
ORDER BY 1 DESC, 3 DESC;

-- Function: Aggregate hourly metrics
CREATE OR REPLACE FUNCTION aggregate_telemetry_hourly()
RETURNS void AS $$
BEGIN
  -- Aggregate LLM usage
  INSERT INTO telemetry_hourly (hour, metric_name, labels, count, sum, avg)
  SELECT
    date_trunc('hour', created_at),
    'llm_tokens',
    jsonb_build_object('model', model),
    COUNT(*),
    SUM(input_tokens + output_tokens),
    AVG(input_tokens + output_tokens)
  FROM llm_usage
  WHERE created_at > NOW() - INTERVAL '2 hours'
  GROUP BY 1, 2, 3
  ON CONFLICT (hour, metric_name, labels)
  DO UPDATE SET
    count = EXCLUDED.count,
    sum = EXCLUDED.sum,
    avg = EXCLUDED.avg;

  -- Aggregate latencies
  INSERT INTO telemetry_hourly (hour, metric_name, labels, count, sum, min, max, avg)
  SELECT
    date_trunc('hour', created_at),
    'llm_latency',
    jsonb_build_object('model', model),
    COUNT(*),
    SUM(latency_ms),
    MIN(latency_ms),
    MAX(latency_ms),
    AVG(latency_ms)
  FROM llm_usage
  WHERE created_at > NOW() - INTERVAL '2 hours'
    AND latency_ms IS NOT NULL
  GROUP BY 1, 2, 3
  ON CONFLICT (hour, metric_name, labels)
  DO UPDATE SET
    count = EXCLUDED.count,
    sum = EXCLUDED.sum,
    min = EXCLUDED.min,
    max = EXCLUDED.max,
    avg = EXCLUDED.avg;
END;
$$ LANGUAGE plpgsql;

COMMENT ON TABLE telemetry_snapshots IS 'Periodic telemetry exports for analysis';
COMMENT ON TABLE frictions IS 'Friction points: errors, failures, timeouts';
COMMENT ON TABLE llm_usage IS 'LLM call tracking for cost and performance analysis';
COMMENT ON TABLE judgment_metrics IS 'Judgment quality tracking';
COMMENT ON TABLE session_summary IS 'Per-session aggregated statistics';
