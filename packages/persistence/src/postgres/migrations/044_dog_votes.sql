-- Migration 044: Dog Votes Table
-- 
-- Tracks individual Dog votes during parallel judgment cycles.
-- Enables learning from Dog performance and consensus analysis.
--
-- R1: DogPipeline voting infrastructure

CREATE TABLE IF NOT EXISTS dog_votes (
  id SERIAL PRIMARY KEY,
  dog_name VARCHAR(50) NOT NULL,
  item_type VARCHAR(100) NOT NULL,
  item_id VARCHAR(255),
  vote_score NUMERIC(5,4) NOT NULL CHECK (vote_score >= 0 AND vote_score <= 1),
  confidence NUMERIC(5,4) NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
  reasoning TEXT,
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for Dog performance queries
CREATE INDEX IF NOT EXISTS idx_dog_votes_dog_name ON dog_votes(dog_name);

-- Index for item type analysis
CREATE INDEX IF NOT EXISTS idx_dog_votes_item_type ON dog_votes(item_type);

-- Index for temporal queries
CREATE INDEX IF NOT EXISTS idx_dog_votes_created_at ON dog_votes(created_at DESC);

-- Composite index for Dog+ItemType queries
CREATE INDEX IF NOT EXISTS idx_dog_votes_dog_item ON dog_votes(dog_name, item_type);

COMMENT ON TABLE dog_votes IS 'Individual Dog votes during parallel judgment cycles (R1)';
COMMENT ON COLUMN dog_votes.dog_name IS 'Name of Dog that cast the vote';
COMMENT ON COLUMN dog_votes.item_type IS 'Type of item being judged';
COMMENT ON COLUMN dog_votes.item_id IS 'ID of item being judged (if available)';
COMMENT ON COLUMN dog_votes.vote_score IS 'Vote score (0=reject, 1=approve, 0.5=abstain)';
COMMENT ON COLUMN dog_votes.confidence IS 'Dog confidence in vote (φ-bounded)';
COMMENT ON COLUMN dog_votes.reasoning IS 'Dog reasoning for vote';
COMMENT ON COLUMN dog_votes.metadata IS 'Additional metadata (consensus verdict, etc.)';
