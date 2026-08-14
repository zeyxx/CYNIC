DEFINE FIELD OVERWRITE created_at ON agent_tasks TYPE int | string;
DEFINE FIELD OVERWRITE result ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE completed_at ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE agent_id ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE claimed_by ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE error ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE due_at ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE priority ON agent_tasks TYPE option<int | null>;
DEFINE FIELD OVERWRITE consumer ON agent_tasks TYPE option<string | null>;
DEFINE FIELD OVERWRITE escalation_policy ON agent_tasks TYPE option<string | null>;

