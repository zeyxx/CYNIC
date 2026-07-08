-- 1. Create Namespace and Database
DEFINE NAMESPACE cynic_os;
DEFINE DATABASE core;

-- 2. Define Tables
DEFINE TABLE organs SCHEMAFULL;
DEFINE FIELD role ON TABLE organs TYPE string;
DEFINE FIELD memory_mb ON TABLE organs TYPE int;
DEFINE FIELD cores ON TABLE organs TYPE int;
DEFINE FIELD status ON TABLE organs TYPE string ASSERT $value IN ['active', 'dead', 'hibernating'];

DEFINE TABLE mesh_peers SCHEMAFULL;
DEFINE FIELD organ_id ON TABLE mesh_peers TYPE record<organs>;
DEFINE FIELD public_key ON TABLE mesh_peers TYPE string;
DEFINE FIELD internal_ip ON TABLE mesh_peers TYPE string;
DEFINE FIELD endpoint ON TABLE mesh_peers TYPE string;

DEFINE TABLE workloads SCHEMAFULL;
DEFINE FIELD organ_id ON TABLE workloads TYPE record<organs>;
DEFINE FIELD mandate ON TABLE workloads TYPE string;
DEFINE FIELD args ON TABLE workloads TYPE array;

-- 3. Define Graph Relations (Edges)
DEFINE TABLE depends_on TYPE RELATION IN organs OUT organs;

-- 4. Seed the Genesis Organs (The Holy Trinity)
CREATE organs:101 CONTENT {
    role: "dna_forgejo",
    memory_mb: 1024,
    cores: 1,
    status: "active"
};

CREATE organs:103 CONTENT {
    role: "memory_surrealdb",
    memory_mb: 2048,
    cores: 1,
    status: "active"
};

CREATE organs:102 CONTENT {
    role: "brain_cortex",
    memory_mb: 8192,
    cores: 2,
    status: "hibernating"
};

-- 5. Link dependencies (Graph)
RELATE organs:101->depends_on->organs:103;
RELATE organs:102->depends_on->organs:103;

-- 6. CYNIC Mempool (Distribution Chain)
DEFINE TABLE agent_tasks SCHEMAFULL;
DEFINE FIELD kind ON TABLE agent_tasks TYPE string;
DEFINE FIELD domain ON TABLE agent_tasks TYPE string;
DEFINE FIELD objective ON TABLE agent_tasks TYPE string;
DEFINE FIELD status ON TABLE agent_tasks TYPE string ASSERT $value IN ['pending', 'claimed', 'completed', 'failed'];
DEFINE FIELD payload ON TABLE agent_tasks TYPE object;
DEFINE FIELD created_at ON TABLE agent_tasks TYPE datetime VALUE time::now();
DEFINE FIELD claimed_by ON TABLE agent_tasks TYPE option<string>;

-- 7. Proof of History (Append-Only Sink)
DEFINE TABLE proof_of_history SCHEMAFULL;
DEFINE FIELD timestamp ON TABLE proof_of_history TYPE datetime VALUE time::now();
DEFINE FIELD source_organ ON TABLE proof_of_history TYPE string;
DEFINE FIELD event_type ON TABLE proof_of_history TYPE string;
DEFINE FIELD payload ON TABLE proof_of_history TYPE object;
DEFINE FIELD cryptographic_hash ON TABLE proof_of_history TYPE string; -- SHA3-512
DEFINE FIELD cryptographic_signature ON TABLE proof_of_history TYPE option<string>;

