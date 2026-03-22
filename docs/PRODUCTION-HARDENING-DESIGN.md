# CYNIC Production Hardening — Design Document

*2026-03-22 — Research-backed priority tiers. Every fix with its gate.*

## Philosophy

Fixes without gates regress. This is the fix-and-shift pattern, observed 3+ times across sessions. This doc organizes 32 production gaps by **priority tier** informed by empirical research of production Rust services (axum, actix-web, tonic) in March 2026.

**Key insight from research:** Industry doesn't ship in phases — it ships in priority tiers. Items from different domains (compiler, logging, signals) are interleaved by criticality, not grouped by category.

```
L7: Compiler         #![deny(...)], type system         → permanent, zero cost
L6: Clippy lints     clippy::unwrap_used                → permanent, zero cost
L5: Tests            cargo test                         → every build
L4: Pre-commit hooks gitleaks, custom grep              → every commit
L3: Pre-push hooks   cargo build+test+clippy            → every push
L2: Runtime checks   health loop, boot validation       → continuous
L1: CLAUDE.md rules  "don't do X"                       → AI compliance (weakest)
```

## Research Findings (March 2026)

Sources: dasroot.net production guides, Determinate Systems axum instrumentation, tokio.rs docs, axum-prometheus/metrics-rs crate docs, Atlassian/CircleCI git workflow analysis, config-rs patterns guide.

| Domain | Industry consensus | CYNIC status | Priority |
|---|---|---|---|
| Structured logging | `tracing` + `tracing-subscriber` universal | 134 unstructured calls, zero tracing | **P0** |
| Graceful shutdown | SIGTERM in every production guide | Only ctrl_c(), data loss on every restart | **P0** |
| Compiler enforcement | clippy deny lints standard | No deny lints | **P0** |
| Prometheus metrics | `axum-prometheus` + `metrics` standard | Zero metrics | **P1** |
| Git workflow | GitHub Flow for small teams | Direct to main | **P0 (process)** |
| Config management | config-rs + env layering | TOML + ad-hoc env, 3 drift incidents | **P1 (env only)** |
| Testing | Unit+integration essential, property/fuzz P2 | Some unit, few integration | **P1** |
| CI/CD | Pre-push hooks sufficient for 2-person teams | Have hooks, untested gate | **P0 (verify)** |
| External observability | VictoriaMetrics + Grafana after kernel instrumented | None | **P3** |

**Crystallized truths from analysis:**
- T1: Git workflow before code changes (zero cost, rollback safety)
- T2: Priority tiers, not linear phases
- T3: SIGTERM is prerequisite for metrics (graceful shutdown before data collection)
- T4: tracing foundation (init) and tracing migration (134 calls) are separate work units
- T5: config-rs is unnecessary at our scale — fix env drift with generation, not a framework
- T6: Pre-push hooks ARE the CI — add deploy-time check as second gate
- T7: Ship the fix before perfecting the ordering

---

## Git Workflow (establish FIRST, before any code)

**Model:** GitHub Flow (simplified for 2 devs, no PR review)

```
main ← always deployable
  └─ feat/description ← short-lived, one logical change
       └─ /build passes → merge --no-ff → push → delete branch
```

**Rules:**
- No direct commits to main (except CLAUDE.md/docs hotfixes)
- Branch naming: `feat/`, `fix/`, `chore/` prefix
- `/build` must pass before merge
- `git merge --no-ff` to preserve branch history
- Delete branch after merge

**Gate:** Pre-push hook verifies not pushing directly to main without merge commit.

---

## P0 — Do Now (this session)

These are industry-unanimous critical gaps. Compiler gates + SIGTERM + tracing foundation. All orthogonal, all independently testable.

### P0.1 — `#![deny(clippy::unwrap_used)]` in non-test code

**Gap:** C2 — 77 `.unwrap()` calls across 18 files. `reasoning_content` unwrap in `openai.rs` will panic in production.
**Fix:** Replace all non-test `.unwrap()` with `.unwrap_or_default()`, `if let`, or `?`.
**Gate:** Add `#![deny(clippy::unwrap_used)]` to `lib.rs`. Compiler rejects all future `.unwrap()`.
**Test:** `cargo clippy --release -- -D warnings` (in pre-push hook).
**Verify:** Add `.unwrap()` in non-test code, confirm clippy fails.

### P0.2 — SIGTERM handler + graceful shutdown

**Gap:** C1 — systemd sends SIGTERM, kernel handles only SIGINT. Every restart = data loss.
**Fix:** Add `tokio::signal::unix::signal(SignalKind::terminate())` in `tokio::select!` alongside `ctrl_c()`.
**Gate:** Integration test sends SIGTERM to spawned process, verifies usage flushed.
**Test:** `tests/integration_shutdown.rs` — `#[ignore]` (needs process spawn).
**Verify:** `systemctl --user restart cynic-kernel`, check usage data persisted.
**Industry pattern:** axum's `with_graceful_shutdown()` + tokio signal handlers are in every production guide.

### P0.3 — Timeout on ALL `tokio::spawn` background awaits

**Gap:** H3 — 10 `tokio::spawn` across 4 files, some with bare `.await` on DB/HTTP calls.
**Fix:** Wrap in `tokio::time::timeout(Duration::from_secs(10), ...)`.
**Gate:** Pre-push hook grep: `grep -rn 'tokio::spawn' src/ | grep -v timeout | grep -v test` → must return zero.
**Test:** Existing tests pass; timeout is a safety net for production stalls.
**Verify:** Run grep, confirm zero hits.

### P0.4 — tracing foundation (crate + subscriber init ONLY)

**Gap:** H1 — 134 unstructured log calls, zero structured logging.
**Fix (foundation only — NOT full migration):**
1. Add `tracing`, `tracing-subscriber` to Cargo.toml
2. Initialize subscriber in `main.rs` with JSON format + EnvFilter
3. Migrate `main.rs` boot messages only (klog! → tracing::info!)
4. Add `tower_http::TraceLayer` to router for HTTP request spans

**NOT in P0:** migrating all 134 calls. That's P1 (mechanical, separate sessions).

**Crates:**
- `tracing` — structured event emission
- `tracing-subscriber` with features `["env-filter", "json"]`

**Init pattern (from industry research):**
```rust
tracing_subscriber::registry()
    .with(EnvFilter::from_default_env().add_directive("cynic_kernel=info".parse()?))
    .with(tracing_subscriber::fmt::layer().json())
    .init();
```

**Gate:** `RUST_LOG=info cargo run` produces JSON on stdout.
**Test:** Boot kernel, verify JSON log line appears.

### P0.5 — Input length validation

**Gap:** M4 — `/coord/register` intent field has no length limit.
**Fix:** Add length checks: `intent` max 512 chars, batch target items max 256 chars.
**Gate:** Unit test per endpoint sending oversized input, expects 400.
**Test:** In `api/rest/coord.rs` tests.
**Verify:** `cargo test` catches regressions.

---

## P1 — Next Sessions (after P0 solid)

### P1.1 — tracing migration (incremental, module by module)

**Scope:** Migrate 134 klog!/eprintln! calls to tracing macros.
**Strategy:** One module per commit. Dual logging (klog! + tracing) is acceptable during migration.

**Severity mapping:**

| Current pattern | tracing level |
|---|---|
| `klog!("[Ring N]...")` (boot) | `info!` |
| `klog!("[config] ✓...")` | `info!` |
| `klog!("[config] ✗...")` | `warn!` |
| `eprintln!("[Judge] Dog failed...")` | `warn!` |
| `eprintln!("[Judge] TIMEOUT...")` | `error!` |
| `eprintln!("[storage] SLOW QUERY...")` | `warn!` |
| `eprintln!("[config] Cannot read...")` | `error!` |

**Order:** judge.rs → pipeline.rs → main.rs → storage/ → infra/ → probe/ → api/

**Gate (after full migration):** `grep -rn 'eprintln!\|klog!' src/ | grep -v test` → zero.
**Final:** Remove `klog!` macro definition.

### P1.2 — Env file consolidation

**Gap:** M6 — two env files drift independently (3 incidents).
**Fix:** Single source `~/.cynic-env`. `make deploy` generates `~/.config/cynic/env`.
**Gate:** Deploy script diff-checks; drift = warning.
**Skip:** config-rs adoption (unnecessary at our scale — we already parse TOML).

### P1.3 — /metrics endpoint (Prometheus format)

**Prerequisite:** P0.2 (SIGTERM) must be done — without graceful shutdown, metrics lose data on restart.

**Crates:**
- `axum-prometheus` — HTTP request duration/count/pending (5 lines to add)
- `metrics` + `metrics-exporter-prometheus` — custom domain metrics

**Architecture:**
```
Router
  └─ .layer(prometheus_layer)        ← HTTP metrics auto-collected
  └─ GET /metrics → handle.render()  ← Prometheus scrape endpoint (no auth)
  └─ Custom: counter!/gauge!/histogram! in domain code
```

**Metrics to emit:**

| Metric | Type | Labels |
|---|---|---|
| `cynic_dog_latency_seconds` | Histogram | `dog`, `model` |
| `cynic_dog_requests_total` | Counter | `dog`, `status` |
| `cynic_dog_circuit_state` | Gauge (0/1) | `dog` |
| `cynic_verdict_qscore` | Histogram | `domain`, `kind` |
| `cynic_storage_query_duration_seconds` | Histogram | `operation` |
| `cynic_auth_failures_total` | Counter | — |
| `cynic_rate_limit_rejections_total` | Counter | — |

**Gate:** Integration test: `curl /metrics | grep cynic_dog_requests_total`.

---

## P2 — When P0+P1 Solid

### P2.1 — Model identity verification in health loop

**Gap:** H8 — health loop probes but doesn't verify model identity.
**Fix:** GET `/v1/models`, check `expected_model` matches.
**Gate:** Runtime `tracing::error!` on mismatch + `cynic_dog_model_drift` gauge.

### P2.2 — Dog staleness tracking

**Gap:** H9 — no `last_scored` per Dog.
**Fix:** `last_success: Option<Instant>` in CircuitBreaker.
**Gate:** `cynic_dog_last_scored_seconds` gauge in /metrics.

### P2.3 — /health enrichment

**Gap:** /health missing model and staleness info.
**Fix:** Add `model`, `model_verified`, `last_scored_secs_ago` to DogHealthResponse.
**Gate:** E2E test validates new fields.

### P2.4 — API.md accuracy

**Gap:** C4 — documents 4/15 endpoints, wrong auth model.
**Fix:** Generate API.md from route definitions. Script, not manual doc.
**Gate:** Pre-push hook: `make check-api-docs` → drift = block.

### P2.5 — backends.toml in repo

**Gap:** M10 — backends.toml not versioned.
**Fix:** `infra/backends.toml` with placeholders, `make deploy` resolves via envsubst.
**Gate:** `make check-config` validates TOML syntax.

---

## P3 — External Stack (only after kernel instrumented)

Without tracing + /metrics in the kernel, these tools see nothing.

### P3.1 — VictoriaMetrics
Single binary, 200MB RAM. Scrapes `/metrics` every 15s. 90-day retention.

### P3.2 — Grafana
Dashboards: Dog health, scoring distribution, cost tracking, crystal lifecycle. 8 alert rules.

### P3.3 — CrowdSec
SSH + API endpoint protection. Custom parser for CYNIC structured logs.

### P3.4 — Off-site backups
`rsync` to secondary node daily. Cron + verification script.

### P3.5 — Alerting beyond desktop
Grafana unified alerting → webhook to Discord/Telegram.

### P3.6 — Duplicate health timer cleanup
Keep `cynic-healthcheck.service`, remove `cynic-health.timer/service`.

---

## Success Criteria

After P0+P1:
- `cargo clippy --release -- -D warnings -D clippy::unwrap_used` passes
- SIGTERM triggers graceful shutdown with usage flush
- `RUST_LOG=info` produces JSON structured logs
- `/metrics` returns valid Prometheus text
- Pre-push hooks catch: secrets, unwrap, bare spawn, eprintln in non-test
- Feature branch workflow enforced

After P2+P3:
- `/health` shows per-Dog: model verified, last scored, circuit state
- Grafana dashboard with real-time Dog health
- Off-site backup verifiable
- Alerts reach external channel

## Anti-Goals

- No Kubernetes. No Consul. No Ansible.
- No config-rs (unnecessary at our scale).
- No Langfuse/Phoenix (build epistemic metrics natively).
- No full Wazuh (CrowdSec is right-sized).
- No custom dashboard in cynic-ui for ops (Grafana's job).
- Zero new CLAUDE.md rules — enforce mechanically.
