-- CYNIC Clean-Room Knowledge & Epistemic Schema v1
-- Namespace: cynic | Database: runtime
-- Source of truth for clean-room knowledge graph, RAG chunks, and epistemic facts.

-- 1. Documents (contrats normatifs, specs, crates, articles)
DEFINE TABLE doc TYPE ANY SCHEMALESS PERMISSIONS FULL;
DEFINE FIELD path ON doc TYPE string;
DEFINE FIELD kind ON doc TYPE string; -- 'contract' | 'spec' | 'crate' | 'doc'
DEFINE FIELD title ON doc TYPE option<string>;
DEFINE FIELD content ON doc TYPE option<string>;
DEFINE FIELD updated_at ON doc TYPE datetime DEFAULT time::now();
DEFINE INDEX doc_path_idx ON doc FIELDS path UNIQUE;

-- 2. Chunks (fragments de texte vectorisés pour le RAG)
DEFINE TABLE chunk TYPE ANY SCHEMALESS PERMISSIONS FULL;
DEFINE FIELD chunk_id ON chunk TYPE string;
DEFINE FIELD source_path ON chunk TYPE string;
DEFINE FIELD content ON chunk TYPE string;
DEFINE FIELD line_start ON chunk TYPE int;
DEFINE FIELD line_end ON chunk TYPE int;
DEFINE FIELD section ON chunk TYPE option<string>;
DEFINE FIELD authority ON chunk TYPE option<string>;
-- Index vectoriel HNSW pour embeddings 1024-dim (modèle embeddinggemma / bge)
DEFINE INDEX chunk_vec_idx ON chunk FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32;

-- 3. Relations de Graphe natives (Arêtes typées)
DEFINE TABLE cites TYPE RELATION;          -- in: doc/chunk, out: doc
DEFINE TABLE enforces TYPE RELATION;       -- in: doc (contrat), out: doc (crate/code)
DEFINE TABLE depends_on TYPE RELATION;     -- in: doc (crate), out: doc (crate)
DEFINE TABLE contains_chunk TYPE RELATION; -- in: doc, out: chunk

-- 4. Mémoire Épistémique (Faits observables prouvés par reçu machine)
DEFINE TABLE fact TYPE ANY SCHEMALESS PERMISSIONS FULL;
DEFINE FIELD fact_id ON fact TYPE string;
DEFINE FIELD subject ON fact TYPE string;
DEFINE FIELD predicate ON fact TYPE string;
DEFINE FIELD object_value ON fact TYPE string;
DEFINE FIELD status ON fact TYPE string;    -- 'observed' | 'proven' | 'hypothesized'
DEFINE FIELD proof_ref ON fact TYPE option<string>;
DEFINE FIELD recorded_at ON fact TYPE datetime DEFAULT time::now();
DEFINE INDEX fact_subject_idx ON fact FIELDS subject, predicate;
