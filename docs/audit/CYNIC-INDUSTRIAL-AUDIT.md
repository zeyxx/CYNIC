# CYNIC Industrial Audit — 2026-03-24

State of the system. Not aspirational — what IS, what's BROKEN, what to DO.

**Scope**: 6 deep audits, 43 source files, 67 findings, 8 root causes.
**Audited by**: 5 parallel agents (SQL injection, MCP auth, concurrency, error propagation, infra) + 1 surface audit.

---

## What's Solid (confirmed under scrutiny)

These hold up. Don't break them.

- **Concurrency model**: 0 FAILs. No lock held across `.await`. No deadlock paths. Mutex discipline correct.
- **SQL injection from user input**: 0 paths. `escape_surreal()` + `sanitize_id()` cover all user-facing inputs.
- **Hexagonal architecture**: `ChatPort`, `StoragePort`, `EmbeddingPort` traits clean. `grep 'reqwest' domain/` = 0.
- **Compiler enforcement**: `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]`.
- **Graceful shutdown**: SIGTERM → CancellationToken → drain REST → flush usage → drain bg tasks.
- **Secret handling**: No secret values in logs. Env-var indirection in TOML. File-based API keys.
- **Transaction safety**: `observe_crystal` wrapped in `BEGIN/COMMIT TRANSACTION`.

---

## 8 Root Causes

Every finding traces to one of these. Fix the root, fix the class.

### RC1 — No shared security layer between REST and MCP

REST has middleware (auth → rate-limit → audit). MCP has NOTHING. Same kernel, asymmetric protection.

| Finding | Severity | Location |
|---|---|---|
| MCP zero authentication | CRITICAL | `api/mcp/mod.rs` — zero references to `api_key` |
| MCP no rate limiting | HIGH | REST: 30/min global, 10/min judge. MCP: unlimited |
| `cynic_infer` MCP-only, unprotected | HIGH | Sovereign LLM reachable only through unprotected surface |
| Error messages leak internal state | MEDIUM | `format!("Storage error: {}", e)` returns raw errors |
| Agent impersonation via self-declared agent_id | MEDIUM | No length check, no identity verification |
| Event injection via unauthenticated register | LOW | Any MCP caller can inject `SessionRegistered` events |

**Gate**: Shared `ServiceLayer` trait that both REST and MCP must implement. Compiler refuses to build a handler without auth+rate_limit+audit.

### RC2 — Health lies about system state

`dog_health.len()` counts ALL Dogs including circuit-tripped ones. `/health` returns "sovereign" (200 OK) with 40% of validators dead.

| Finding | Severity | Location |
|---|---|---|
| Health counts all Dogs, not healthy ones | CRITICAL | `health.rs:35` — `dog_count = dog_health.len()` |
| No liveness/readiness separation | HIGH | Single `/health` mixes both concerns |
| No startup probe | MEDIUM | `ExecStartPre=sleep 3` is not a health gate |
| `cynic-health.timer` orphaned | MEDIUM | Timer file exists, not enabled |
| No remote alerting | MEDIUM | Desktop-only notifications |

**Gate**: `fn health_status(dogs: &[DogHealth]) -> Status` counts `circuit == closed` only. Unit test asserts: 5 Dogs, 3 critical → `degraded`. Separate `/live` and `/ready`.

### RC3 — Config-as-declared ≠ System-as-running

Boot validates structure (TOML parses), not state (model exists, endpoint responds correctly). Config drift invisible.

| Finding | Severity | Location |
|---|---|---|
| No model name verification at boot | HIGH | `main.rs:119` — model logged, not verified |
| No config drift detection at runtime | MEDIUM | No steady-state comparison |
| `CARGO_MANIFEST_DIR` baked into binary | LOW | `main.rs:91` — fragile path for domain prompts |
| `dirs::config_dir()` silent fallback to "." | LOW | `main.rs:55` |

**Gate**: `BackendConfig::validate()` calls `/models` and asserts configured model name is present. Kernel won't bind port until all backends validate. Fail fast, not fail silent.

### RC4 — Hand-rolled SQL instead of parameterized queries

`escape_surreal()` + string interpolation. One missed field = injection. Collision in `sanitize_record_id`.

| Finding | Severity | Location |
|---|---|---|
| `flush_usage` dog_id unescaped | HIGH | `surreal.rs:546` — stored injection vector via DB |
| `sanitize_record_id` collision | MEDIUM | `surreal.rs:634` — `a-b` and `a.b` → same key |
| `sanitize_record_id` no length limit | MEDIUM | MCP path has no size guard |
| `escape_surreal` doesn't escape backticks | LOW | `mod.rs:347` — no active surface but undocumented |

**Gate**: Replace ALL string-interpolated SQL with SurrealDB parameterized queries (`vars`). Delete `escape_surreal()` — if it doesn't exist, it can't be used wrong.

### RC5 — Silent failure paths

Errors that return `Ok(())` or produce wrong results without logging. Every one is a bomb.

| Finding | Severity | Location |
|---|---|---|
| `NullStorage.store_verdict` returns `Ok(())` | HIGH | `domain/storage.rs:241` — Rule #8 violation |
| Claim verification DB error → "conflict (race)" | HIGH | `surreal.rs:695` — indistinguishable from real race |
| SSE serialization failure → empty event | MEDIUM | `events.rs:33` — no log |
| Probe reqwest client build → silent default | MEDIUM | `probe/llm.rs:295` — TLS failure invisible |
| Integrity chain seeding failure not logged | MEDIUM | `main.rs:175` |
| Metrics hydration failure silent (counters = 0) | MEDIUM | `main.rs:239-246` |
| RwLock poison silently swallowed | MEDIUM | `health.rs:83`, `tasks.rs:322` |
| Observe handler returns 200 on dropped observation | MEDIUM | `observe.rs` — false positive to caller |
| Corrupt `dog_scores_json` → empty Vec | LOW | `surreal.rs:106` |
| llama API key file permission error silent | LOW | `summarizer.rs:40-44` |

**Gate**: Types that force error handling. `NullStorage` returns `Err(Degraded)`, not `Ok(())`. `DegradedResult<T>` newtype that forces caller to acknowledge loss. No silent `Ok` on failure.

### RC6 — No sovereign inference process management

llama-servers started manually. No systemd supervision. Duplicate processes accumulate. Hardcoded IPs.

| Finding | Severity | Location |
|---|---|---|
| Real IP in llama-server.service | HIGH | `~/.config/systemd/user/llama-server.service:9` |
| No llama-server unit in repo | MEDIUM | Only lives on deployed machine |
| No duplicate process detection | MEDIUM | Two llama-servers on :8080 observed |
| Zero security hardening in all units | MEDIUM | No `NoNewPrivileges`, `PrivateTmp` |
| `ExecStartPre=sleep 3` fragile | LOW | Should be readiness check |

**Gate**: `infra/systemd/` contains ALL units with `%h/` and env vars (no hardcoded IPs). `make deploy` installs them. `BindsTo=` for dependency ordering.

### RC7 — No request tracing

Zero `#[instrument]`, zero trace IDs. A failed judge request produces logs in 5 subsystems with no correlation.

| Finding | Severity | Location |
|---|---|---|
| No trace IDs on any request | HIGH | Zero `#[instrument]` in codebase |
| Multi-hop request correlation impossible | HIGH | pipeline → judge → dog → storage — no thread |

**Gate**: `#[instrument(skip_all, fields(request_id))]` on pipeline entry points. `request_id = Uuid::new_v4()` propagated through `PipelineDeps`. Lint: `make check` verifies instrument count.

### RC8 — Security hygiene gaps

Committed secrets, default bindings, missing audits.

| Finding | Severity | Location |
|---|---|---|
| Real Tailscale IPs in tracked repo | HIGH | `scripts/exp-*.py`, `benchmarks/SOVEREIGN-INFERENCE.md` |
| `setup-ubuntu.sh` defaults to `0.0.0.0:3030` | MEDIUM | `scripts/setup-ubuntu.sh:157` |
| `make rollback/hotfix` parse JSON | MEDIUM | `Makefile:136,156` — Rule #16 violation |
| No `cargo audit` in pipeline | MEDIUM | Known CVEs not detected |
| CORS `allow_methods(Any)` too broad | LOW | `api/rest/mod.rs:37-52` |
| `Restart=always` on KAIROS collectors | LOW | Infinite loop on clean exit |

**Gate**: gitleaks + IP scan + `cargo audit` unified in `make check`. `setup-ubuntu.sh` defaults to `127.0.0.1`. IPs in scripts replaced with placeholders.

---

## Compound Order

Rule #27: each task should name what it feeds downstream.

```
RC2 (honest health) ──────────────→ RC1 (MCP shares health contract)
     │                                    │
     └─→ RC3 (boot validation uses       │
          honest health to fail fast)     │
                                          │
RC5 (no silent failures) ────────────→ RC1 (MCP error handling)
     │
     └─→ RC4 (parameterized queries eliminate
          a class of silent failure)

RC8 (security hygiene) → independent, do first (quick wins)
RC6 (process management) → independent, do in parallel
RC7 (tracing) → do after RC1+RC2 (needs request context to be useful)
```

**Recommended execution order:**

1. **RC8** — Security hygiene quick wins (IPs, setup script, cargo audit). 30 min. Unblocks clean commits.
2. **RC2** — Honest health. 1h. Foundation for all monitoring.
3. **RC5** — Silent failure paths. 2h. Foundation for trust.
4. **RC3** — Boot validation. 1h. Requires honest health (RC2).
5. **RC1** — MCP security layer. 3h. Requires RC2+RC5 design patterns.
6. **RC4** — Parameterized SQL. 4h. Touch every query. Biggest refactor.
7. **RC6** — Process management. 2h. Independent, can parallelize.
8. **RC7** — Request tracing. 2h. Most valuable after RC1+RC2.

---

## CYNIC Judges CYNIC — Honest Score

| Axiom | Score | Evidence |
|---|---|---|
| FIDELITY | 0.15 | /health lies about system state |
| PHI | 0.25 | REST and MCP have asymmetric protection |
| VERIFY | 0.30 | 253 unit tests but 0 integration tests in CI, observe_crystal tx untested concurrently |
| CULTURE | 0.20 | Below industrial SRE standard (no liveness/readiness, no OTel, no remote alerting) |
| BURN | 0.25 | 49 deferred bombs, hand-rolled SQL escaping |
| SOVEREIGNTY | 0.20 | External deps change silently, no config drift detection |
| **Q-Score** | **0.22** | **BARK** |

CYNIC scores Bark on its own infrastructure. This is the honest starting point.

---

## Appendix A — Concurrency Audit (0 FAILs, 7 WARNs)

- HalfOpen allows 2 concurrent probes instead of 1 (`circuit_breaker.rs:64`)
- Verdict cache benign duplicate entries under concurrent store/lookup
- Usage flush snapshot/absorb gap (safe by design — SET semantics)
- Prometheus ratio from non-atomic pair of loads (`metrics.rs:60-64`)
- Blake3 hash chain serializes under burst load (`judge.rs:297`)
- Circuit breaker filter may be stale by execution time (`judge.rs:119`)
- Double crystal observation for concurrent identical stimuli (`pipeline.rs:296-314`)

## Appendix B — Error Propagation Audit (3 FAILs, 7 WARNs)

**FAILs:**
1. `surreal.rs:695` — DB error during claim verification → "Conflict (race)" — wrong result, no log
2. `events.rs:33` — serialization failure → empty SSE event — no log
3. `probe/llm.rs:295` — reqwest client build failure → silent default — no log

**WARNs:**
4. No test for `store_verdict()` failure path
5. Integrity chain seeding error silent (`main.rs:175`)
6. Metrics hydration error silent (`main.rs:239-246`)
7. `dirs::config_dir()` → "." silent fallback (`main.rs:55`)
8. llama API key file permission error silent (`summarizer.rs:40-44`)
9. Corrupt `dog_scores_json` → empty Vec (`surreal.rs:106`)
10. RwLock poison silently swallowed (`health.rs:83`, `tasks.rs:322`)

## Appendix C — SQL/Storage Audit (0 injection FAILs, 6 WARNs)

- `flush_usage` dog_id unescaped — stored injection via DB → SQL (`surreal.rs:546`)
- `sanitize_record_id` collides distinct inputs (`surreal.rs:634`)
- `sanitize_record_id` no length limit — DoS via MCP path
- `escape_surreal` doesn't escape backticks — no active surface
- `observe_crystal` transaction not tested concurrently
- `NullStorage.store_verdict` returns `Ok(())` — Rule #8 violation

## Appendix D — MCP Auth Audit (3 FAILs, 6 WARNs)

**FAILs:**
1. MCP zero authentication — any stdio caller = full kernel access
2. MCP no rate limiting — unlimited judge/infer calls
3. `cynic_infer` MCP-only, unprotected — sovereign LLM fully exposed

**WARNs:**
4. `cynic_infer` prompt injection surface — no content filtering
5. Agent impersonation — self-declared agent_id, no length check in MCP
6. `cynic_audit_query` exposes full audit trail without auth
7. Error messages leak internal URLs, model names
8. Event injection via unauthenticated register
9. Hand-rolled SQL escaping vs parameterized queries

## Appendix E — Systemd/Network/Process Audit (5 FAILs, ~15 WARNs)

**FAILs:**
1. Real IPs in tracked repo (`scripts/exp-*.py`, `benchmarks/SOVEREIGN-INFERENCE.md`)
2. Real IP in `llama-server.service` hardcoded
3. `cynic-health.timer` orphaned (richer monitoring dead)
4. `health-check.sh` JSON parsing (Rule #16 violation)
5. `make rollback/hotfix` JSON parsing (Rule #16 violation)

**Key WARNs:**
- Zero security hardening in all systemd units
- `setup-ubuntu.sh` defaults to `0.0.0.0:3030`
- `ExecStartPre=sleep 3` fragile dependency
- No `StartLimitBurst` — infinite restart loop possible
- No remote alerting path
- `CARGO_MANIFEST_DIR` baked into binary
- CORS `allow_methods(Any)` too broad
- `make check` integration tests conditional on SurrealDB
- No `cargo audit` in pipeline
