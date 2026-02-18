-- CYNIC Python Kernel — Database Initialization
-- Runs once on first postgres-py container start.
-- DDL is also applied at runtime via create_tables() in postgres.py.
--
-- "φ distrusts φ" — κυνικός

-- Enable pgvector extension (required for vector similarity search)
CREATE EXTENSION IF NOT EXISTS vector;

-- Enable uuid-ossp for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- The full DDL is handled by create_tables() in cynic/core/storage/postgres.py
-- at kernel startup. This file just ensures extensions are available.
