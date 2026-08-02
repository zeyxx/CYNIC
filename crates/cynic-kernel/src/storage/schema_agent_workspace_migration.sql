-- Applied only by the separately authorized schema migrator, never by the
-- steady-state runtime identity.
DEFINE TABLE OVERWRITE agent_workspace_claim SCHEMAFULL;
DEFINE TABLE OVERWRITE runtime_proof_commit SCHEMAFULL;
