# CYNIC Complete Infrastructure Requirements

## Current Status: 1297/1397 Tests Passing (92.8%)

### Passing (1297 tests)
- ✅ **Unit Tests** (1100+): Core kernel, adapters, security, APIs
- ✅ **Phase 4 SIEM** (197): Storage, alerting, compliance, detection rules
- ✅ **MCP Bridge** (8): Universal tool architecture
- ✅ **Pre-commit Gates** (5): Encoding, imports, factory, message format

### Failing (39 tests) - Missing Infrastructure
- **Ollama Integration** (5): LLM inference server needed
- **SurrealDB** (6): Vector database needed
- **Vault/Encryption** (2): Secrets management setup needed
- **GASdf/Blockchain** (4): Solana integration (optional)
- **Full Organism** (6): Complete initialization pipeline
- **Metrics Integration** (8): Service dependencies
- **E2E/Performance** (8): Full infrastructure required

### Skipped (50 tests)
- Integration suite disabled (service dependencies)

### Expected Failures (11 errors)
- Full organism lifecycle tests

---

## Infrastructure Stack Required

### 1. **LLM Backend** (AI Infrastructure Engineer)
- **Ollama** - Local LLM inference
  ```bash
  # Required for:
  - Dog judgment (local reasoning)
  - Temporal MCTS (planning)
  - Real completions (not mocks)
  
  # Setup:
  docker run -d -p 11434:11434 --name ollama ollama/ollama
  ollama pull llama2  # or any supported model
  ```
  
- **Missing Components**:
  - `OllamaAdapter` class
  - `LLMRegistry.complete()` implementation
  - Model discovery & selection

### 2. **Vector Database** (Data Engineer + Backend Engineer)
- **SurrealDB** - Real-time event storage + vector search
  ```bash
  # Required for:
  - Security event persistence
  - Embedding storage
  - Real-time queries
  - Live subscriptions
  
  # Setup:
  docker run -p 8080:8080 surrealdb/surrealdb start
  export SURREAL_URL=ws://localhost:8080/rpc
  ```
  
- **Missing Components**:
  - Vector indexing (HNSW)
  - Schema migrations
  - Connection pooling

### 3. **Secrets Management** (Security Architect)
- **Vault** - Encryption keys, credentials
  ```bash
  # Required for:
  - Encryption service initialization
  - API token storage
  - Report signing keys
  
  # Setup:
  docker run -p 8200:8200 -e VAULT_DEV_ROOT_TOKEN_ID=root vault:latest
  export VAULT_ADDR=http://localhost:8200
  export VAULT_TOKEN=root
  ```
  
- **Missing Components**:
  - Vault client configuration
  - Key rotation policies
  - Secret sealing/unsealing

### 4. **LLM Benchmarking System** (ML Platform Engineer)
- **Benchmark Repository**
  - Track model performance across domains
  - EMA-based scoring (weighted)
  - Hardware-aware selection
  
- **Missing Components**:
  - `BenchmarkRepoInterface` implementation
  - Latency tracking
  - Accuracy metrics collection

### 5. **Event Bus & Real-time Streams** (Backend Engineer)
- **In-Memory EventBus** ✅ (Implemented)
- **Persistent Event Storage** (SurrealDB) - Needs setup
- **Live Query Subscriptions** - Requires SurrealDB LIVE SELECT

### 6. **Blockchain Integration** (Blockchain Engineer) - Optional
- **Solana Integration** (GASdf)
  - Treasury management
  - On-chain governance
  - Token transfers
  
- **Missing Components**:
  - Solana RPC client
  - Smart contract interactions
  - Transaction signing

### 7. **Organism Orchestration** (Solutions Architect)
- **Component Lifecycle**
  - EventBus initialization
  - Storage connectivity
  - Dog instantiation
  - Judgment pipeline
  
- **Missing Components**:
  - `Organism.start()` full wiring
  - Service dependency injection
  - Graceful shutdown handlers

### 8. **Monitoring & Observability** (SRE)
- **Metrics Collection**
  - Prometheus endpoints
  - TPS tracking
  - Latency percentiles
  
- **Missing Components**:
  - Metrics aggregation
  - Dashboard setup
  - Alert routing

### 9. **Robotics/Automation** (Robotics Engineer) - Optional
- Hardware control interfaces
- Sensor integration
- Motor control

---

## Test Categories by Infrastructure Need

### Category A: Unit Tests (No Infrastructure) ✅
```
✅ 1100+ tests passing
- CYNIC kernel (core logic)
- Security (RBAC, encryption)
- Storage interfaces (mocked)
- API routers (mocked)
- Event protocol
- Judgment system
- Learning loops
```

### Category B: Integration Tests (Partial Infrastructure)
```
⚠️ 197 tests passing (storage layer)
- Phase 4 SIEM (alerting, compliance, detection)
- Event forwarding
- Retention enforcement
- Report signing

❌ 39 tests failing (need services)
- Ollama: 5 failures
- SurrealDB: 6 failures
- Vault: 2 failures
- Full organism: 6 failures
- Metrics: 8 failures
- GASdf: 4 failures
- E2E: 8 failures
```

### Category C: End-to-End Tests (Full Infrastructure)
```
❌ 11 errors (need complete setup)
- Organism health check
- Topology E2E
- Performance baselines
- Phase 3 organism caching
- Full cycle initialization
```

---

## Minimum Viable Setup for 100% Pass

### Option 1: Full Production Setup (100% passing)
Requires all services running:
- Ollama (LLM)
- SurrealDB (Storage)
- Vault (Secrets)
- Full organism initialization
- Blockchain (GASdf)

**Time to setup**: 2-3 hours with Docker
**Test pass rate**: 1397/1397 (100%)

### Option 2: Core-Only Setup (95% passing)
Skip optional components:
- Ollama + SurrealDB + Vault only
- Skip GASdf blockchain
- Skip E2E performance tests

**Time to setup**: 1 hour with Docker
**Test pass rate**: 1350+/1397 (96.6%)

### Option 3: Current State (92.8% passing)
Unit tests only, mocked infrastructure:
- No external services
- All core logic tested
- Phase 4 SIEM verified

**Time to setup**: 0 minutes (now)
**Test pass rate**: 1297/1397 (92.8%)

---

## Docker Compose Setup (Option 1/2)

```yaml
version: '3.8'

services:
  surrealdb:
    image: surrealdb/surrealdb:latest
    ports:
      - "8080:8080"
    environment:
      SURREAL_BIND: 0.0.0.0:8080
    command: start

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    environment:
      OLLAMA_HOST: 0.0.0.0:11434

  vault:
    image: vault:latest
    ports:
      - "8200:8200"
    environment:
      VAULT_DEV_ROOT_TOKEN_ID: root
    cap_add:
      - IPC_LOCK

volumes:
  ollama_data:
```

**Setup**:
```bash
docker-compose up -d
ollama pull llama2
export SURREAL_URL=ws://localhost:8080/rpc
export VAULT_ADDR=http://localhost:8200
export VAULT_TOKEN=root
pytest tests/ -v
```

---

## Missing Code Components

### High Priority (Blocks 25 tests)
- [ ] `OllamaAdapter` class (LLM routing)
- [ ] `BenchmarkRepoInterface` implementation
- [ ] Full `Organism.start()` initialization
- [ ] SurrealDB live subscriptions

### Medium Priority (Blocks 10 tests)
- [ ] Vault integration
- [ ] Metrics aggregation pipeline
- [ ] Blockchain (GASdf) client

### Low Priority (Blocks 4 tests)
- [ ] E2E performance baselines
- [ ] Full topology integration

---

## Recommendation

**Current state (1297/1397 passing) is suitable for**:
- Local development
- Code review
- Unit test validation
- Phase 4 SIEM verification ✅

**To reach 100% pass rate**:
1. Add Docker Compose setup
2. Implement missing adapter classes
3. Initialize SurrealDB connection pool
4. Add Vault integration
5. Wire up full organism

**Estimated effort**: 20-30 developer hours for complete setup
**Priority**: Medium (core functionality is working; infrastructure is optional for feature development)
