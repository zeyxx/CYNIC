-- CYNIC SurrealDB Schema — v0.7.5
-- Namespace: cynic | Database: v2
-- Source of truth for table definitions.
-- Apply with: surreal import --endpoint http://localhost:8000 --namespace cynic --database v2

-- ── Agent coordination ──────────────────────────────────────

DEFINE TABLE agent_session TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD active ON agent_session TYPE bool PERMISSIONS FULL;
DEFINE FIELD agent_id ON agent_session TYPE string PERMISSIONS FULL;
DEFINE FIELD agent_type ON agent_session TYPE string PERMISSIONS FULL;
DEFINE FIELD intent ON agent_session TYPE string PERMISSIONS FULL;
DEFINE FIELD last_seen ON agent_session TYPE datetime PERMISSIONS FULL;
DEFINE FIELD registered_at ON agent_session TYPE datetime PERMISSIONS FULL;

DEFINE INDEX agent_session_active_idx ON agent_session FIELDS active;

-- ── Crystals (CCM pipeline) ─────────────────────────────────

DEFINE TABLE crystal TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD confidence ON crystal TYPE float PERMISSIONS FULL;
DEFINE FIELD content ON crystal TYPE string PERMISSIONS FULL;
DEFINE FIELD domain ON crystal TYPE string PERMISSIONS FULL;
DEFINE FIELD observations ON crystal TYPE int PERMISSIONS FULL;
DEFINE FIELD state ON crystal TYPE string PERMISSIONS FULL;

DEFINE INDEX crystal_domain_idx ON crystal FIELDS domain;
DEFINE INDEX crystal_obs_idx ON crystal FIELDS observations;
DEFINE INDEX crystal_vec_idx ON crystal FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32 EFC 150 M 12 M0 24 LM 0.40242960438184466f;

-- ── Dog usage tracking ──────────────────────────────────────

DEFINE TABLE dog_usage TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD completion_tokens ON dog_usage TYPE int PERMISSIONS FULL;
DEFINE FIELD dog_id ON dog_usage TYPE string PERMISSIONS FULL;
DEFINE FIELD failures ON dog_usage TYPE int PERMISSIONS FULL;
DEFINE FIELD prompt_tokens ON dog_usage TYPE int PERMISSIONS FULL;
DEFINE FIELD requests ON dog_usage TYPE int PERMISSIONS FULL;
DEFINE FIELD total_latency_ms ON dog_usage TYPE int PERMISSIONS FULL;
DEFINE FIELD updated_at ON dog_usage TYPE datetime PERMISSIONS FULL;

-- ── MCP audit trail ─────────────────────────────────────────

DEFINE TABLE mcp_audit TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD agent_id ON mcp_audit TYPE string PERMISSIONS FULL;
DEFINE FIELD details ON mcp_audit TYPE string PERMISSIONS FULL;
DEFINE FIELD tool ON mcp_audit TYPE string PERMISSIONS FULL;
DEFINE FIELD ts ON mcp_audit TYPE datetime PERMISSIONS FULL;

DEFINE INDEX mcp_audit_ts_idx ON mcp_audit FIELDS ts;

-- ── Observations (CCM analytics) ────────────────────────────

DEFINE TABLE observation TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD agent_id ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD context ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD created_at ON observation TYPE datetime PERMISSIONS FULL;
DEFINE FIELD domain ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD project ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD session_id ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD status ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD target ON observation TYPE string PERMISSIONS FULL;
DEFINE FIELD tool ON observation TYPE string PERMISSIONS FULL;

DEFINE INDEX obs_agent_idx ON observation FIELDS agent_id;
DEFINE INDEX obs_created_idx ON observation FIELDS created_at;
DEFINE INDEX obs_domain_idx ON observation FIELDS domain;
DEFINE INDEX obs_project_idx ON observation FIELDS project;
DEFINE INDEX obs_target_idx ON observation FIELDS target;

-- ── REST audit trail ────────────────────────────────────────

DEFINE TABLE rest_audit TYPE ANY SCHEMALESS PERMISSIONS NONE;

-- ── Session compliance ──────────────────────────────────────

DEFINE TABLE session_compliance TYPE ANY SCHEMALESS PERMISSIONS NONE;

-- ── Session summaries ───────────────────────────────────────

DEFINE TABLE session_summary TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD agent_id ON session_summary TYPE string PERMISSIONS FULL;
DEFINE FIELD created_at ON session_summary TYPE datetime PERMISSIONS FULL;
DEFINE FIELD observations_count ON session_summary TYPE int PERMISSIONS FULL;
DEFINE FIELD session_id ON session_summary TYPE string PERMISSIONS FULL;
DEFINE FIELD summary ON session_summary TYPE string PERMISSIONS FULL;

DEFINE INDEX session_summary_created_idx ON session_summary FIELDS created_at;
DEFINE INDEX session_summary_session_idx ON session_summary FIELDS session_id UNIQUE;

-- ── Verdicts (judge pipeline) ───────────────────────────────

DEFINE TABLE verdict TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD anomaly_axiom ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD anomaly_detected ON verdict TYPE bool PERMISSIONS FULL;
DEFINE FIELD burn ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD created_at ON verdict TYPE datetime PERMISSIONS FULL;
DEFINE FIELD culture ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD dog_id ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD dog_scores_json ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD fidelity ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD integrity_hash ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD kind ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD max_disagreement ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD phi ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD prev_hash ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_burn ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_culture ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_fidelity ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_phi ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_sovereignty ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD reasoning_verify ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD sovereignty ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD stimulus ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD total ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD verdict_id ON verdict TYPE string PERMISSIONS FULL;
DEFINE FIELD verify ON verdict TYPE float PERMISSIONS FULL;
DEFINE FIELD voter_count ON verdict TYPE int PERMISSIONS FULL;

DEFINE INDEX verdict_created_idx ON verdict FIELDS created_at;
DEFINE INDEX verdict_id_idx ON verdict FIELDS verdict_id UNIQUE;

-- ── Work claims (coordination locks) ────────────────────────

DEFINE TABLE work_claim TYPE ANY SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD active ON work_claim TYPE bool PERMISSIONS FULL;
DEFINE FIELD agent_id ON work_claim TYPE string PERMISSIONS FULL;
DEFINE FIELD claim_type ON work_claim TYPE string PERMISSIONS FULL;
DEFINE FIELD claimed_at ON work_claim TYPE datetime PERMISSIONS FULL;
DEFINE FIELD target ON work_claim TYPE string PERMISSIONS FULL;

DEFINE INDEX work_claim_active_idx ON work_claim FIELDS active;
DEFINE INDEX work_claim_target_idx ON work_claim FIELDS target;

-- ── Infrastructure snapshots (probe system) ──────────────────
DEFINE TABLE IF NOT EXISTS infra_snapshot SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS ts ON infra_snapshot TYPE datetime;
DEFINE FIELD IF NOT EXISTS overall ON infra_snapshot TYPE string;
DEFINE FIELD IF NOT EXISTS probes ON infra_snapshot TYPE array;
DEFINE FIELD IF NOT EXISTS probes.* ON infra_snapshot TYPE object;
DEFINE INDEX IF NOT EXISTS idx_infra_ts ON infra_snapshot FIELDS ts;
