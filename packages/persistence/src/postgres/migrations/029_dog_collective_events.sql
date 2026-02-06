-- =============================================================================
-- Migration 029: Dog Collective Events
-- =============================================================================
-- Closes the remaining data loops for Dog collective behavior.
-- Events were emitted but never persisted - a black hole.
--
-- "Le pack se souvient de chaque décision" - κυνικός
-- =============================================================================

-- =============================================================================
-- DOG EVENTS TABLE
-- =============================================================================
-- Captures individual dog invocations, blocks, warnings, actions.
-- Source: DogStateEmitter → EventType.DOG_EVENT

CREATE TABLE IF NOT EXISTS dog_events (
  id              BIGSERIAL PRIMARY KEY,
  event_id        UUID DEFAULT gen_random_uuid(),
  dog_name        VARCHAR(32) NOT NULL,
  event_type      VARCHAR(32) NOT NULL,  -- invocation, block, warning, action, error
  stats           JSONB DEFAULT '{}',
  health          VARCHAR(16),
  details         JSONB DEFAULT '{}',
  session_id      UUID,
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dog_events_dog_name ON dog_events (dog_name);
CREATE INDEX IF NOT EXISTS idx_dog_events_event_type ON dog_events (event_type);
CREATE INDEX IF NOT EXISTS idx_dog_events_created_at ON dog_events (created_at);
CREATE INDEX IF NOT EXISTS idx_dog_events_session ON dog_events (session_id);

-- =============================================================================
-- CONSENSUS VOTES TABLE
-- =============================================================================
-- Captures consensus results with full vote breakdown.
-- Source: AmbientConsensus → EventType.CONSENSUS_COMPLETED

CREATE TABLE IF NOT EXISTS consensus_votes (
  id              BIGSERIAL PRIMARY KEY,
  consensus_id    VARCHAR(128) NOT NULL,
  topic           VARCHAR(256) NOT NULL,
  approved        BOOLEAN NOT NULL,
  agreement       REAL,               -- 0.0 to 1.0
  guardian_veto   BOOLEAN DEFAULT FALSE,
  votes           JSONB DEFAULT '{}', -- { dogName: { vote, reason } }
  stats           JSONB DEFAULT '{}', -- { approve, reject, abstain, total }
  reason          VARCHAR(256),
  session_id      UUID,
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_consensus_topic ON consensus_votes (topic);
CREATE INDEX IF NOT EXISTS idx_consensus_approved ON consensus_votes (approved);
CREATE INDEX IF NOT EXISTS idx_consensus_created_at ON consensus_votes (created_at);

-- =============================================================================
-- DOG SIGNALS TABLE
-- =============================================================================
-- Inter-dog communication (7 signal types).
-- Source: DogSignal events from AmbientConsensus

CREATE TABLE IF NOT EXISTS dog_signals (
  id              BIGSERIAL PRIMARY KEY,
  signal_type     VARCHAR(64) NOT NULL,  -- e.g. dog:danger_detected
  source_dog      VARCHAR(32),
  payload         JSONB DEFAULT '{}',
  session_id      UUID,
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dog_signals_type ON dog_signals (signal_type);
CREATE INDEX IF NOT EXISTS idx_dog_signals_source ON dog_signals (source_dog);
CREATE INDEX IF NOT EXISTS idx_dog_signals_created_at ON dog_signals (created_at);

-- =============================================================================
-- COLLECTIVE SNAPSHOTS TABLE
-- =============================================================================
-- Periodic collective health snapshots (sampled, not every emission).
-- Source: DogStateEmitter → EventType.CYNIC_STATE

CREATE TABLE IF NOT EXISTS collective_snapshots (
  id              BIGSERIAL PRIMARY KEY,
  active_dogs     INT,
  dog_count       INT,
  average_health  REAL,
  health_rating   VARCHAR(16),
  pattern_count   INT,
  memory_load     REAL,
  memory_freshness REAL,
  snapshot_data   JSONB DEFAULT '{}',
  session_id      UUID,
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_collective_snapshots_created_at ON collective_snapshots (created_at);
CREATE INDEX IF NOT EXISTS idx_collective_snapshots_health ON collective_snapshots (health_rating);

-- =============================================================================
-- CLEANUP FUNCTIONS
-- =============================================================================
-- Keep last 10K events per dog, 30 days of snapshots.

CREATE OR REPLACE FUNCTION cleanup_dog_events(max_per_dog INT DEFAULT 10000)
RETURNS INT AS $$
DECLARE
  deleted_count INT := 0;
  rows_affected INT;
  dog_row RECORD;
BEGIN
  FOR dog_row IN SELECT DISTINCT dog_name FROM dog_events LOOP
    WITH excess AS (
      SELECT id FROM dog_events
      WHERE dog_name = dog_row.dog_name
      ORDER BY created_at DESC
      OFFSET max_per_dog
    )
    DELETE FROM dog_events WHERE id IN (SELECT id FROM excess);
    GET DIAGNOSTICS rows_affected = ROW_COUNT;
    deleted_count := deleted_count + rows_affected;
  END LOOP;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_collective_snapshots(retention_days INT DEFAULT 30)
RETURNS INT AS $$
DECLARE
  deleted_count INT;
BEGIN
  DELETE FROM collective_snapshots
  WHERE created_at < NOW() - (retention_days || ' days')::INTERVAL;
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_dog_signals(retention_days INT DEFAULT 30)
RETURNS INT AS $$
DECLARE
  deleted_count INT;
BEGIN
  DELETE FROM dog_signals
  WHERE created_at < NOW() - (retention_days || ' days')::INTERVAL;
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_consensus_votes(retention_days INT DEFAULT 90)
RETURNS INT AS $$
DECLARE
  deleted_count INT;
BEGIN
  DELETE FROM consensus_votes
  WHERE created_at < NOW() - (retention_days || ' days')::INTERVAL;
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;
