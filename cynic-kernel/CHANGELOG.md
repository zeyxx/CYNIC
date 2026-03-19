# Changelog — cynic-kernel

All notable changes. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] — 2026-03-19

### Added
- **P2 Semantic verdict cache** — cosine similarity lookup before Dog fan-out (threshold 0.95, FIFO 1000)
- **Storage observability** — atomic counters in query(): queries, errors, slow queries >100ms, avg latency
- **Dog::health()** trait method — unified backend observability, cascades from ChatPort
- **EmbeddingBackend 5-state health** — BackendStatus instead of bool
- **StoragePort::metrics()** — domain-level storage metrics, no concrete type leak
- **Shutdown usage flush** — actually flushes to SurrealDB on SIGINT (was no-op before)
- **45s axum TimeoutLayer** — prevents cascading timeouts from slow SurrealDB
- **SurrealDB 401 retry** — 3 attempts with 50/100ms backoff for intermittent auth failures
- **StorageError + CoordError implement std::error::Error**

### Changed
- **NullStorage::ping()** returns Err — /health no longer lies about storage in degraded mode
- **/health embed check** has 2s timeout — prevents blocking on slow embedding server
- **Rate limiter eviction** moved to background timer (was O(N) on every request)
- **/judge** now checks global rate limit first, then judge-specific (was bypassing global)
- **Usage flush SQL** deduplicated — single build_flush_sql() method
- **dog_health()** runs parallel via join_all (was sequential = 5×2s)
- **probe/** replaced std::env::var("HOME") with dirs::home_dir() (5 files)
- **Domain purity** — blake3 moved from domain/ccm.rs to judge.rs, chrono removed from domain/usage.rs
- **mcp_audit cleanup** — date-based DELETE instead of subquery (was causing 10s timeouts + transaction drops)

### Removed
- **gRPC stack** — 4 stub files, BackendRouter, 6 optional deps (tonic, prost, etc.) — -761 lines
- **ucb1_phi** dead MCTS primitive from temporal.rs
- **InferencePort impl** from openai.rs (Dogs use ChatPort, not InferencePort)
- **BackendCapability** field from OpenAiCompatBackend

### Fixed
- systemd Requires= → Wants= for surrealdb (NullStorage fallback was never exercised)
- MemoryMax=2G on kernel, 4G on SurrealDB (OOM protection)
- LimitNOFILE=65535 on SurrealDB
- SurrealDB block cache capped at 512MB (was 13.5GB auto-sized)
- SurrealDB credentials via EnvironmentFile (was --pass in ps aux)
- Backup service: gzip compression, env file auth

## [0.5.0] — 2026-03-18

### Added
- Git describe in /health — traceable builds
- Compound G — health storage check, error responses, failure tracking
- Data integrity + hook robustness (5 fixes)

## [0.4.0] — 2026-03-17 (not tagged)

### Added
- Hexagonal module refactor (23→43 files, 9 modules)
- CCM workflow aggregator
- Coordination L2 (register, claim, release, who)
- HOWL threshold fix (0.82→φ⁻²+φ⁻⁴)

## [0.3.0] — 2026-03-16 (not tagged)

### Added
- MCP server (rmcp, stdio)
- 6 Dogs live (deterministic + 5 inference)
- SurrealDB HTTP adapter (replaced surrealdb crate)
- Security hardening (auth, CORS, rate limiting, input validation)

## [0.2.0] — 2026-03-14 (not tagged)

### Added
- REST API (axum)
- Judge with parallel Dog evaluation
- Circuit breaker per backend
- DeterministicDog + InferenceDog

## [0.1.0] — 2026-03-10

### Added
- Initial kernel bootstrap
- Domain types (Stimulus, AxiomScores, Verdict, QScore)
- Phi constants
- Probe module
