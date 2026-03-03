-- PHASE 1B: Add encrypted columns to governance community model
--
-- Migration: Add support for encrypted storage of sensitive governance data
-- - community_token: Community authorization token (API key)
-- - treasury_address: Blockchain address for treasury management
--
-- Strategy:
-- - Add new _*_encrypted columns to store ciphertext
-- - Keep original columns for backward compatibility (deprecated)
-- - Application layer handles encryption/decryption
-- - Encryption keys fetched from Vault (not in database)

-- Add encrypted columns to communities table
ALTER TABLE communities ADD COLUMN IF NOT EXISTS _community_token_encrypted TEXT;
ALTER TABLE communities ADD COLUMN IF NOT EXISTS _treasury_address_encrypted TEXT;

-- Add indexes for encrypted columns (useful for integrity checks)
CREATE INDEX IF NOT EXISTS idx_communities_token_encrypted
ON communities(_community_token_encrypted);

CREATE INDEX IF NOT EXISTS idx_communities_address_encrypted
ON communities(_treasury_address_encrypted);

-- Add comment documenting encryption strategy
COMMENT ON COLUMN communities._community_token_encrypted IS
'Encrypted community token (API key) - encrypted with AES-256-GCM.
Original plaintext community_token column deprecated.
Encryption keys stored in Vault, never in database.';

COMMENT ON COLUMN communities._treasury_address_encrypted IS
'Encrypted treasury blockchain address - encrypted with AES-256-GCM.
Original plaintext treasury_address column deprecated.
Encryption keys stored in Vault, never in database.';

-- Create audit table for encryption key rotations (for compliance)
CREATE TABLE IF NOT EXISTS governance_encryption_rotations (
    rotation_id TEXT PRIMARY KEY,
    community_id TEXT NOT NULL,
    field_name TEXT NOT NULL,  -- 'community_token' or 'treasury_address'
    old_key_id TEXT,
    new_key_id TEXT,
    rotated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,
    FOREIGN KEY (community_id) REFERENCES communities(community_id)
);

CREATE INDEX IF NOT EXISTS idx_encryption_rotations_community
ON governance_encryption_rotations(community_id);

CREATE INDEX IF NOT EXISTS idx_encryption_rotations_timestamp
ON governance_encryption_rotations(rotated_at DESC);

-- Migration guide for existing data:
--
-- For communities with plaintext data that need encryption:
-- 1. Read plaintext community_token from community_token column
-- 2. Encrypt using EncryptionService with key_id='governance-community-token'
-- 3. Store result in _community_token_encrypted column
-- 4. Mark original community_token as deprecated (can set to NULL later)
-- 5. Log rotation to governance_encryption_rotations for audit trail
--
-- This migration supports both plaintext (legacy) and encrypted (new) data
-- coexisting during transition period.
