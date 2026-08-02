-- Applied only by the separately authorized schema migrator, never by the
-- steady-state runtime identity.
DEFINE TABLE OVERWRITE agent_workload_delivery SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS workload_id ON agent_workload_delivery TYPE string;
DEFINE FIELD IF NOT EXISTS idempotency_key ON agent_workload_delivery TYPE string;
DEFINE FIELD IF NOT EXISTS node_id ON agent_workload_delivery TYPE string;
DEFINE FIELD IF NOT EXISTS submitted_by ON agent_workload_delivery TYPE string;
DEFINE FIELD IF NOT EXISTS status ON agent_workload_delivery TYPE string;
DEFINE FIELD IF NOT EXISTS submitted_at_ms ON agent_workload_delivery TYPE int;
DEFINE FIELD IF NOT EXISTS accepted_at_ms ON agent_workload_delivery TYPE option<int>;
DEFINE FIELD IF NOT EXISTS record_json ON agent_workload_delivery TYPE string;
DEFINE INDEX IF NOT EXISTS agent_workload_delivery_idempotency_idx ON agent_workload_delivery FIELDS idempotency_key UNIQUE;
DEFINE INDEX IF NOT EXISTS agent_workload_delivery_pending_idx ON agent_workload_delivery FIELDS node_id, status, submitted_at_ms;
DEFINE TABLE OVERWRITE agent_workspace_claim SCHEMAFULL;
DEFINE TABLE OVERWRITE runtime_proof_commit SCHEMAFULL;
