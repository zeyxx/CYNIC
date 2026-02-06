-- =============================================================================
-- Migration 030: Distributed Tracing
-- =============================================================================
-- φ-aligned distributed tracing for the CYNIC ecosystem.
-- Answers: "Which Dog is slow?", "Where is latency?", "What's the critical path?"
--
-- "φ mesure la latence" - κυνικός
-- =============================================================================

-- =============================================================================
-- TRACES TABLE
-- =============================================================================
-- One row per trace (a group of related spans).

CREATE TABLE IF NOT EXISTS traces (
  trace_id        UUID PRIMARY KEY,
  root_span_id    UUID,
  name            VARCHAR(256) NOT NULL,
  start_time      BIGINT NOT NULL,         -- epoch ms
  end_time        BIGINT,
  duration        INT,                     -- ms
  status          VARCHAR(16) DEFAULT 'unset',  -- unset, ok, error
  service         VARCHAR(64) DEFAULT 'cynic',
  attributes      JSONB DEFAULT '{}',
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_traces_start_time ON traces (start_time);
CREATE INDEX IF NOT EXISTS idx_traces_status ON traces (status);
CREATE INDEX IF NOT EXISTS idx_traces_name ON traces (name);
CREATE INDEX IF NOT EXISTS idx_traces_duration ON traces (duration);

-- =============================================================================
-- SPANS TABLE
-- =============================================================================
-- Individual operations within a trace. Parent-child hierarchy via parent_span_id.

CREATE TABLE IF NOT EXISTS spans (
  span_id         UUID PRIMARY KEY,
  trace_id        UUID NOT NULL REFERENCES traces(trace_id) ON DELETE CASCADE,
  parent_span_id  UUID,
  name            VARCHAR(256) NOT NULL,
  start_time      BIGINT NOT NULL,         -- epoch ms
  end_time        BIGINT,
  duration        INT,                     -- ms
  status          VARCHAR(16) DEFAULT 'unset',
  error           TEXT,
  attributes      JSONB DEFAULT '{}',
  events          JSONB DEFAULT '[]',
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_spans_trace_id ON spans (trace_id);
CREATE INDEX IF NOT EXISTS idx_spans_parent ON spans (parent_span_id);
CREATE INDEX IF NOT EXISTS idx_spans_name ON spans (name);
CREATE INDEX IF NOT EXISTS idx_spans_start_time ON spans (start_time);
CREATE INDEX IF NOT EXISTS idx_spans_duration ON spans (duration);
CREATE INDEX IF NOT EXISTS idx_spans_status ON spans (status);

-- =============================================================================
-- VIEWS
-- =============================================================================

-- Trace summary: count of spans, total duration, error count
CREATE OR REPLACE VIEW trace_summary AS
SELECT
  t.trace_id,
  t.name,
  t.start_time,
  t.duration,
  t.status,
  t.service,
  COUNT(s.span_id) AS span_count,
  COUNT(CASE WHEN s.status = 'error' THEN 1 END) AS error_count,
  MAX(s.duration) AS max_span_duration
FROM traces t
LEFT JOIN spans s ON s.trace_id = t.trace_id
GROUP BY t.trace_id, t.name, t.start_time, t.duration, t.status, t.service;

-- Dog latency percentiles (P50, P95, P99 per dog)
CREATE OR REPLACE VIEW dog_latencies AS
SELECT
  attributes->>'dog.name' AS dog_name,
  attributes->>'dog.sefirah' AS sefirah,
  COUNT(*) AS total_spans,
  ROUND(AVG(duration)) AS avg_ms,
  PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration) AS p50_ms,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration) AS p95_ms,
  PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration) AS p99_ms,
  MIN(duration) AS min_ms,
  MAX(duration) AS max_ms,
  COUNT(CASE WHEN status = 'error' THEN 1 END) AS error_count
FROM spans
WHERE name LIKE 'dog:%'
  AND duration IS NOT NULL
GROUP BY attributes->>'dog.name', attributes->>'dog.sefirah';

-- =============================================================================
-- CLEANUP
-- =============================================================================
-- 13-day retention (Fibonacci F7)

CREATE OR REPLACE FUNCTION cleanup_traces(retention_days INT DEFAULT 13)
RETURNS INT AS $$
DECLARE
  deleted_count INT;
  cutoff_ms BIGINT;
BEGIN
  cutoff_ms := (EXTRACT(EPOCH FROM NOW() - (retention_days || ' days')::INTERVAL) * 1000)::BIGINT;
  -- Cascade deletes spans via FK
  DELETE FROM traces WHERE start_time < cutoff_ms;
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;
