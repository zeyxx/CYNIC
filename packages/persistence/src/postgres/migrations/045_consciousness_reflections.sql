-- Consciousness Reflections Migration
-- Migration: 045_consciousness_reflections
-- Purpose: Store self-reflection cycles for meta-cognition

CREATE TABLE IF NOT EXISTS consciousness_reflections (
    id              SERIAL PRIMARY KEY,
    user_id         UUID REFERENCES users(id) ON DELETE CASCADE,
    window_hours    INTEGER NOT NULL DEFAULT 24,
    state_snapshot  JSONB NOT NULL,
    prompts         JSONB NOT NULL,
    overall_confidence DECIMAL(5,4),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_consciousness_reflections_user ON consciousness_reflections(user_id);
CREATE INDEX idx_consciousness_reflections_created ON consciousness_reflections(created_at DESC);

-- Migration tracking
INSERT INTO _migrations (name) VALUES ('045_consciousness_reflections')
ON CONFLICT (name) DO NOTHING;
