-- Minimal CYNIC low-layer kernel store.
DEFINE NAMESPACE cynic_os;
DEFINE DATABASE core;

DEFINE TABLE runtime_nodes SCHEMAFULL;
DEFINE FIELD role ON TABLE runtime_nodes TYPE string;
DEFINE FIELD status ON TABLE runtime_nodes TYPE string ASSERT $value IN ['candidate', 'observed', 'locked', 'retired'];
DEFINE FIELD memory_mb ON TABLE runtime_nodes TYPE option<int>;
DEFINE FIELD cores ON TABLE runtime_nodes TYPE option<int>;
DEFINE FIELD updated_at ON TABLE runtime_nodes TYPE datetime VALUE time::now();

DEFINE TABLE mesh_peers SCHEMAFULL;
DEFINE FIELD node_id ON TABLE mesh_peers TYPE record<runtime_nodes>;
DEFINE FIELD public_key ON TABLE mesh_peers TYPE string;
DEFINE FIELD internal_ip ON TABLE mesh_peers TYPE string;
DEFINE FIELD endpoint ON TABLE mesh_peers TYPE option<string>;

DEFINE TABLE workloads SCHEMAFULL;
DEFINE FIELD node_id ON TABLE workloads TYPE record<runtime_nodes>;
DEFINE FIELD mandate ON TABLE workloads TYPE string;
DEFINE FIELD args ON TABLE workloads TYPE array;
DEFINE FIELD status ON TABLE workloads TYPE string ASSERT $value IN ['pending', 'running', 'completed', 'failed'];

DEFINE TABLE depends_on TYPE RELATION IN runtime_nodes OUT runtime_nodes;

CREATE runtime_nodes:kernel CONTENT {
    role: "kernel_primary",
    memory_mb: 2048,
    cores: 1,
    status: "candidate"
};

CREATE runtime_nodes:builder CONTENT {
    role: "build_runner",
    memory_mb: 4096,
    cores: 2,
    status: "candidate"
};

CREATE runtime_nodes:storage CONTENT {
    role: "storage_primary",
    memory_mb: 2048,
    cores: 1,
    status: "candidate"
};

RELATE runtime_nodes:kernel->depends_on->runtime_nodes:storage;
RELATE runtime_nodes:builder->depends_on->runtime_nodes:storage;

DEFINE TABLE agent_tasks SCHEMAFULL;
DEFINE FIELD kind ON TABLE agent_tasks TYPE string;
DEFINE FIELD domain ON TABLE agent_tasks TYPE string;
DEFINE FIELD objective ON TABLE agent_tasks TYPE string;
DEFINE FIELD status ON TABLE agent_tasks TYPE string ASSERT $value IN ['pending', 'claimed', 'completed', 'failed'];
DEFINE FIELD payload ON TABLE agent_tasks TYPE object;
DEFINE FIELD created_at ON TABLE agent_tasks TYPE datetime VALUE time::now();
DEFINE FIELD claimed_by ON TABLE agent_tasks TYPE option<string>;

DEFINE TABLE proof_of_history SCHEMAFULL;
DEFINE FIELD timestamp ON TABLE proof_of_history TYPE datetime VALUE time::now();
DEFINE FIELD source_node ON TABLE proof_of_history TYPE string;
DEFINE FIELD event_type ON TABLE proof_of_history TYPE string;
DEFINE FIELD payload ON TABLE proof_of_history TYPE object;
DEFINE FIELD cryptographic_hash ON TABLE proof_of_history TYPE string;
DEFINE FIELD cryptographic_signature ON TABLE proof_of_history TYPE option<string>;
