-- =============================================================================
-- Migration 013: Add UNIQUE constraint on users.username
-- =============================================================================
-- The users table was missing a UNIQUE constraint on username.
-- This caused "no unique or exclusion constraint" errors when
-- _ensureUserExists used ON CONFLICT (username).
--
-- "Trust nothing. Verify everything." - κυνικός
-- =============================================================================

-- Add UNIQUE constraint on username
-- This will fail if there are duplicate usernames
-- We use CREATE UNIQUE INDEX CONCURRENTLY for non-blocking creation
-- Note: CONCURRENTLY cannot be used in a transaction, so we do it directly

ALTER TABLE users
ADD CONSTRAINT users_username_key UNIQUE (username);

-- Add index for faster lookups by username
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('013_fix_username_unique')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- This fixes the "no unique or exclusion constraint" error
-- when syncing user profiles via brain_profile_sync
