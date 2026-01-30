/**
 * SQLite Schema - Core Tables for Offline/Edge Deployment
 *
 * Creates SQLite-compatible tables matching the PostgreSQL schema.
 * Supports core CYNIC functionality: judgments, patterns, users, sessions.
 *
 * "Same truth, smaller kennel" - κυνικός
 *
 * @module @cynic/persistence/sqlite/schema
 */

'use strict';

/**
 * Core schema SQL statements
 * φ-aligned: includes essential tables only, minimalist approach
 */
export const SCHEMA_SQL = `
-- =============================================================================
-- CYNIC SQLite Schema (Offline/Edge Fallback)
-- =============================================================================
-- Core tables only - matches PostgreSQL schema structure
-- JSON stored as TEXT, UUIDs as TEXT, timestamps as TEXT (ISO 8601)
-- =============================================================================

-- Users table
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  email TEXT UNIQUE,
  username TEXT UNIQUE,
  display_name TEXT,
  burn_amount REAL DEFAULT 0,
  e_score REAL DEFAULT 0,
  uptime_hours REAL DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  last_active TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  session_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  project_path TEXT,
  project_name TEXT,
  branch TEXT,
  started_at TEXT DEFAULT (datetime('now')),
  ended_at TEXT,
  work_done INTEGER DEFAULT 0,
  heat_generated INTEGER DEFAULT 0,
  patterns_detected INTEGER DEFAULT 0,
  judgments_made INTEGER DEFAULT 0,
  metadata TEXT DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_path);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);

-- Judgments table (core PoJ)
CREATE TABLE IF NOT EXISTS judgments (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  judgment_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  session_id TEXT,
  item_type TEXT NOT NULL,
  item_content TEXT,
  item_hash TEXT,
  q_score REAL,
  global_score REAL,
  confidence REAL,
  verdict TEXT,
  axiom_scores TEXT DEFAULT '{}',
  dimension_scores TEXT,
  weaknesses TEXT DEFAULT '[]',
  context TEXT DEFAULT '{}',
  reasoning_path TEXT DEFAULT '[]',
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_judgments_user ON judgments(user_id);
CREATE INDEX IF NOT EXISTS idx_judgments_session ON judgments(session_id);
CREATE INDEX IF NOT EXISTS idx_judgments_type ON judgments(item_type);
CREATE INDEX IF NOT EXISTS idx_judgments_verdict ON judgments(verdict);
CREATE INDEX IF NOT EXISTS idx_judgments_created ON judgments(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_judgments_hash ON judgments(item_hash);

-- Patterns table
CREATE TABLE IF NOT EXISTS patterns (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  pattern_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  session_id TEXT,
  pattern_type TEXT NOT NULL,
  name TEXT,
  description TEXT,
  frequency INTEGER DEFAULT 1,
  weight REAL DEFAULT 1.0,
  last_seen TEXT DEFAULT (datetime('now')),
  first_seen TEXT DEFAULT (datetime('now')),
  axiom TEXT DEFAULT 'VERIFY',
  context TEXT DEFAULT '{}',
  metadata TEXT DEFAULT '{}',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_patterns_user ON patterns(user_id);
CREATE INDEX IF NOT EXISTS idx_patterns_type ON patterns(pattern_type);
CREATE INDEX IF NOT EXISTS idx_patterns_axiom ON patterns(axiom);
CREATE INDEX IF NOT EXISTS idx_patterns_frequency ON patterns(frequency DESC);
CREATE INDEX IF NOT EXISTS idx_patterns_weight ON patterns(weight DESC);

-- Knowledge table
CREATE TABLE IF NOT EXISTS knowledge (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  knowledge_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  session_id TEXT,
  knowledge_type TEXT NOT NULL,
  title TEXT,
  content TEXT,
  source TEXT,
  confidence REAL DEFAULT 0.618,
  axiom TEXT DEFAULT 'VERIFY',
  tags TEXT DEFAULT '[]',
  metadata TEXT DEFAULT '{}',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_user ON knowledge(user_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_type ON knowledge(knowledge_type);
CREATE INDEX IF NOT EXISTS idx_knowledge_axiom ON knowledge(axiom);

-- Feedback table (for learning)
CREATE TABLE IF NOT EXISTS feedback (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  feedback_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  judgment_id TEXT REFERENCES judgments(judgment_id),
  feedback_type TEXT NOT NULL,
  outcome TEXT,
  correct INTEGER,
  notes TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_feedback_user ON feedback(user_id);
CREATE INDEX IF NOT EXISTS idx_feedback_judgment ON feedback(judgment_id);
CREATE INDEX IF NOT EXISTS idx_feedback_outcome ON feedback(outcome);

-- PoJ Blocks table
CREATE TABLE IF NOT EXISTS poj_blocks (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  block_hash TEXT UNIQUE NOT NULL,
  prev_hash TEXT,
  slot INTEGER NOT NULL,
  proposer TEXT,
  block_type TEXT DEFAULT 'judgment',
  judgments TEXT DEFAULT '[]',
  merkle_root TEXT,
  signature TEXT,
  status TEXT DEFAULT 'pending',
  created_at TEXT DEFAULT (datetime('now')),
  finalized_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_poj_blocks_slot ON poj_blocks(slot DESC);
CREATE INDEX IF NOT EXISTS idx_poj_blocks_status ON poj_blocks(status);
CREATE INDEX IF NOT EXISTS idx_poj_blocks_proposer ON poj_blocks(proposer);

-- Psychology state table
CREATE TABLE IF NOT EXISTS psychology_state (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  user_id TEXT UNIQUE REFERENCES users(id),
  session_id TEXT,
  energy INTEGER DEFAULT 100,
  focus INTEGER DEFAULT 100,
  state TEXT DEFAULT 'READY',
  heat INTEGER DEFAULT 0,
  work INTEGER DEFAULT 0,
  temperature REAL DEFAULT 0,
  entropy REAL DEFAULT 0,
  last_updated TEXT DEFAULT (datetime('now')),
  history TEXT DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_psychology_user ON psychology_state(user_id);

-- Conversation memories table
CREATE TABLE IF NOT EXISTS conversation_memories (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  memory_id TEXT UNIQUE NOT NULL,
  user_id TEXT REFERENCES users(id),
  session_id TEXT,
  memory_type TEXT NOT NULL,
  content TEXT NOT NULL,
  importance REAL DEFAULT 0.5,
  metadata TEXT DEFAULT '{}',
  embedding TEXT,
  created_at TEXT DEFAULT (datetime('now')),
  last_accessed TEXT DEFAULT (datetime('now')),
  access_count INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_memories_user ON conversation_memories(user_id);
CREATE INDEX IF NOT EXISTS idx_memories_type ON conversation_memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON conversation_memories(importance DESC);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at TEXT DEFAULT (datetime('now')),
  description TEXT
);

-- Insert initial version if not exists
INSERT OR IGNORE INTO schema_version (version, description)
VALUES (1, 'Initial SQLite schema for offline/edge deployment');
`;

/**
 * Initialize SQLite schema
 * @param {SQLiteClient} client - SQLite client
 * @returns {Promise<void>}
 */
export async function initializeSchema(client) {
  await client.connect();
  client.exec(SCHEMA_SQL);
}

/**
 * Check if schema is initialized
 * @param {SQLiteClient} client - SQLite client
 * @returns {Promise<boolean>}
 */
export async function isSchemaInitialized(client) {
  try {
    await client.connect();
    const result = await client.query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'"
    );
    return result.rows.length > 0;
  } catch {
    return false;
  }
}

/**
 * Get current schema version
 * @param {SQLiteClient} client - SQLite client
 * @returns {Promise<number>}
 */
export async function getSchemaVersion(client) {
  try {
    const result = await client.query(
      'SELECT MAX(version) as version FROM schema_version'
    );
    return result.rows[0]?.version || 0;
  } catch {
    return 0;
  }
}

export default {
  SCHEMA_SQL,
  initializeSchema,
  isSchemaInitialized,
  getSchemaVersion,
};
