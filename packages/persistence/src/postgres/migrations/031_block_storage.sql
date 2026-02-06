-- =============================================================================
-- Migration 031: Block Storage
-- =============================================================================
-- Persistent block storage for Phase 2 multi-node consensus.
-- Blocks link via parent_hash to form the chain.
--
-- "Chaque bloc est une vérité gravée" - κυνικός
-- =============================================================================

-- =============================================================================
-- BLOCKS TABLE
-- =============================================================================

CREATE TABLE IF NOT EXISTS blocks (
  slot            BIGINT PRIMARY KEY,
  hash            VARCHAR(128) NOT NULL,
  proposer        VARCHAR(128) NOT NULL,
  merkle_root     VARCHAR(128),
  judgments       JSONB DEFAULT '[]',
  judgment_count  INT DEFAULT 0,
  parent_hash     VARCHAR(128),
  timestamp       BIGINT NOT NULL,         -- epoch ms
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks (hash);
CREATE INDEX IF NOT EXISTS idx_blocks_proposer ON blocks (proposer);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks (timestamp);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_hash ON blocks (parent_hash);

-- =============================================================================
-- BLOCK ANCHORS TABLE
-- =============================================================================
-- Tracks Solana anchoring status per block.

CREATE TABLE IF NOT EXISTS block_anchors (
  slot                BIGINT PRIMARY KEY REFERENCES blocks(slot) ON DELETE CASCADE,
  solana_tx_signature VARCHAR(256),
  anchor_status       VARCHAR(16) DEFAULT 'pending',  -- pending, anchored, failed
  merkle_root         VARCHAR(128),
  cluster             VARCHAR(16) DEFAULT 'devnet',
  created_at          TIMESTAMPTZ DEFAULT NOW(),
  anchored_at         TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_block_anchors_status ON block_anchors (anchor_status);
CREATE INDEX IF NOT EXISTS idx_block_anchors_created ON block_anchors (created_at);

-- =============================================================================
-- CLEANUP
-- =============================================================================
-- Keep last 10000 blocks (configurable)

CREATE OR REPLACE FUNCTION cleanup_blocks(max_blocks INT DEFAULT 10000)
RETURNS INT AS $$
DECLARE
  deleted_count INT;
BEGIN
  WITH excess AS (
    SELECT slot FROM blocks
    ORDER BY slot DESC
    OFFSET max_blocks
  )
  DELETE FROM blocks WHERE slot IN (SELECT slot FROM excess);
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;
