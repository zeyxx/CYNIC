-- Migration: Add encryption at rest for sensitive columns
-- Version: 1.4 (Encryption at Rest)
-- Date: 2026-03-03
-- Purpose: Add encrypted column support to CYNIC database

-- ============================================================================
-- SENSITIVE DATA COLUMNS REQUIRING ENCRYPTION
-- ============================================================================

-- Governance data: community_token, treasury_address, contract addresses
ALTER TABLE IF EXISTS communities ADD COLUMN community_token_encrypted TEXT;
ALTER TABLE IF EXISTS communities ADD COLUMN treasury_address_encrypted TEXT;
ALTER TABLE IF EXISTS communities ADD COLUMN near_contract_address_encrypted TEXT;

-- Wallet secrets: private keys, signing keys
ALTER TABLE IF EXISTS wallets ADD COLUMN private_key_encrypted TEXT;
ALTER TABLE IF EXISTS wallets ADD COLUMN signing_key_encrypted TEXT;

-- API secrets: API keys, tokens, credentials
ALTER TABLE IF EXISTS api_credentials ADD COLUMN api_key_encrypted TEXT;
ALTER TABLE IF EXISTS api_credentials ADD COLUMN api_secret_encrypted TEXT;
ALTER TABLE IF EXISTS api_credentials ADD COLUMN bearer_token_encrypted TEXT;

-- Event journal sensitive data: error messages, debug info
ALTER TABLE IF EXISTS event_journal ADD COLUMN error_message_encrypted TEXT;

-- User sensitive data: email, phone, addresses
ALTER TABLE IF EXISTS users ADD COLUMN email_encrypted TEXT;
ALTER TABLE IF EXISTS users ADD COLUMN phone_encrypted TEXT;
ALTER TABLE IF EXISTS users ADD COLUMN address_encrypted TEXT;

-- ============================================================================
-- ENCRYPTION METADATA
-- ============================================================================

-- Track which columns are encrypted and with which key ID
CREATE TABLE IF NOT EXISTS encrypted_columns (
    id SERIAL PRIMARY KEY,
    table_name VARCHAR(255) NOT NULL,
    column_name VARCHAR(255) NOT NULL,
    key_id VARCHAR(255) NOT NULL DEFAULT 'default',
    encryption_algorithm VARCHAR(50) NOT NULL DEFAULT 'aes-256-gcm',
    encrypted_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(table_name, column_name)
);

-- Insert metadata for encrypted columns
INSERT INTO encrypted_columns (table_name, column_name, key_id) VALUES
    ('communities', 'community_token_encrypted', 'governance-secrets'),
    ('communities', 'treasury_address_encrypted', 'governance-secrets'),
    ('communities', 'near_contract_address_encrypted', 'governance-secrets'),
    ('wallets', 'private_key_encrypted', 'wallet-keys'),
    ('wallets', 'signing_key_encrypted', 'wallet-keys'),
    ('api_credentials', 'api_key_encrypted', 'api-secrets'),
    ('api_credentials', 'api_secret_encrypted', 'api-secrets'),
    ('api_credentials', 'bearer_token_encrypted', 'api-secrets'),
    ('event_journal', 'error_message_encrypted', 'journal-errors'),
    ('users', 'email_encrypted', 'user-pii'),
    ('users', 'phone_encrypted', 'user-pii'),
    ('users', 'address_encrypted', 'user-pii')
ON CONFLICT (table_name, column_name) DO NOTHING;

-- ============================================================================
-- AUDIT LOGGING FOR ENCRYPTION OPERATIONS
-- ============================================================================

-- Create audit log table if not exists (may exist from task 1.3)
CREATE TABLE IF NOT EXISTS audit_log (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    action VARCHAR(255) NOT NULL,
    principal VARCHAR(255),
    resource VARCHAR(255),
    result VARCHAR(50),
    details JSONB
);

-- Add index for audit queries
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_principal ON audit_log(principal);

-- ============================================================================
-- ENCRYPTION KEY STORAGE (Vault Integration)
-- ============================================================================

-- Table to track encryption key metadata (actual keys stored in Vault)
CREATE TABLE IF NOT EXISTS encryption_keys (
    id SERIAL PRIMARY KEY,
    key_id VARCHAR(255) NOT NULL UNIQUE,
    vault_path VARCHAR(255) NOT NULL,
    algorithm VARCHAR(50) NOT NULL DEFAULT 'aes-256-gcm',
    key_size_bits INT NOT NULL DEFAULT 256,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    rotated_at TIMESTAMP WITH TIME ZONE,
    last_used TIMESTAMP WITH TIME ZONE,
    status VARCHAR(50) NOT NULL DEFAULT 'active'
);

-- Insert standard encryption key IDs
INSERT INTO encryption_keys (key_id, vault_path, algorithm, key_size_bits) VALUES
    ('default', 'encryption/default', 'aes-256-gcm', 256),
    ('governance-secrets', 'encryption/governance-secrets', 'aes-256-gcm', 256),
    ('wallet-keys', 'encryption/wallet-keys', 'aes-256-gcm', 256),
    ('api-secrets', 'encryption/api-secrets', 'aes-256-gcm', 256),
    ('journal-errors', 'encryption/journal-errors', 'aes-256-gcm', 256),
    ('user-pii', 'encryption/user-pii', 'aes-256-gcm', 256)
ON CONFLICT (key_id) DO NOTHING;

-- Create index for key lookups
CREATE INDEX IF NOT EXISTS idx_encryption_keys_key_id ON encryption_keys(key_id);
CREATE INDEX IF NOT EXISTS idx_encryption_keys_status ON encryption_keys(status);

-- ============================================================================
-- DATA MIGRATION: PLAINTEXT → ENCRYPTED (Application-level)
-- ============================================================================

-- Note: The actual data migration happens at the application level via
-- Python code, not in SQL. This is because:
-- 1. Encryption keys live in Vault (not in database)
-- 2. Nonces must be random per encryption (not deterministic)
-- 3. We can't encrypt/decrypt in SQL without key access

-- Placeholder columns for encrypted data exist above.
-- Application code (see cynic/kernel/security/encryption.py) will:
-- 1. Read plaintext from old columns
-- 2. Encrypt using TransparentEncryption service
-- 3. Write to new _encrypted columns
-- 4. Verify all data migrated
-- 5. Drop old plaintext columns (in future migration)

-- ============================================================================
-- BACKUP & COMPLIANCE
-- ============================================================================

-- Create backup tracking table (for disaster recovery)
CREATE TABLE IF NOT EXISTS encryption_backups (
    id SERIAL PRIMARY KEY,
    backup_id VARCHAR(255) NOT NULL UNIQUE,
    backup_type VARCHAR(50),  -- 'vault-snapshot', 'key-rotation', 'emergency'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(50),  -- 'pending', 'completed', 'verified'
    details JSONB
);

-- Track key rotations for compliance
CREATE TABLE IF NOT EXISTS key_rotations (
    id SERIAL PRIMARY KEY,
    key_id VARCHAR(255) NOT NULL,
    old_key_version VARCHAR(255),
    new_key_version VARCHAR(255),
    rotated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    rotation_reason VARCHAR(255),
    status VARCHAR(50),  -- 'pending', 'in_progress', 'completed'
    details JSONB
);

CREATE INDEX IF NOT EXISTS idx_key_rotations_key_id ON key_rotations(key_id);
CREATE INDEX IF NOT EXISTS idx_key_rotations_rotated_at ON key_rotations(rotated_at);

-- ============================================================================
-- INDEXES FOR ENCRYPTED COLUMNS
-- ============================================================================

-- Note: Encrypted columns cannot be indexed directly (breaks encryption semantics)
-- Instead, we can create hash-based indexes for existence checks:

ALTER TABLE IF EXISTS communities
    ADD COLUMN IF NOT EXISTS community_token_hash VARCHAR(64);

ALTER TABLE IF EXISTS api_credentials
    ADD COLUMN IF NOT EXISTS api_key_hash VARCHAR(64);

ALTER TABLE IF EXISTS users
    ADD COLUMN IF NOT EXISTS email_hash VARCHAR(64);

-- These hash columns allow "does this value exist" queries without revealing plaintext
-- They are computed as: SHA-256(plaintext) and updated when encrypted data changes

-- ============================================================================
-- MIGRATION STATUS
-- ============================================================================

-- Table to track migration progress
CREATE TABLE IF NOT EXISTS migration_status (
    id SERIAL PRIMARY KEY,
    migration_name VARCHAR(255) NOT NULL UNIQUE,
    version INT NOT NULL,
    status VARCHAR(50) NOT NULL,  -- 'pending', 'in_progress', 'completed'
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT
);

INSERT INTO migration_status (migration_name, version, status, started_at) VALUES
    ('002_add_encryption_at_rest', 1, 'completed', CURRENT_TIMESTAMP)
ON CONFLICT (migration_name) DO UPDATE SET status = 'completed', completed_at = CURRENT_TIMESTAMP;
