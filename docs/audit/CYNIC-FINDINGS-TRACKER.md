# CYNIC Findings Tracker — v0.7.3

*Updated 2026-03-26. Honest inventory of all known findings.*

**Sources:** Industrial Audit (67 findings, 2026-03-24) + Stress Test (23 findings, 2026-03-25)
**Total: 90 findings. 30 fixed. 9 partial. 51 open.**

---

## CRITICAL (3 open)

| # | Source | Finding | Status |
|---|---|---|---|
| F15 | Stress | Crystal API bypasses epistemic gate — 25 calls = Crystallized | OPEN |
| F20 | Stress | Single-dog mode indistinguishable from consensus | OPEN |
| RC1-1 | Audit | MCP zero authentication (stdio = process trust, but no shared ServiceLayer) | PARTIAL — rate limit added, no auth |

## HIGH (10 open, 6 fixed)

| # | Source | Finding | Status |
|---|---|---|---|
| F2 | Stress | X-Forwarded-For spoofing bypasses rate limiter | OPEN |
| F14 | Stress | Prompt injection scored Wag by LLM Dogs | OPEN |
| F16 | Stress | Crystal observe overwrites content | OPEN |
| F23 | Stress | /events unauthenticated, no connection limit, FD exhaustion | OPEN |
| RC1-2 | Audit | MCP no rate limiting | **FIXED** — McpRateLimit 10/min judge, 30/min other |
| RC1-3 | Audit | cynic_infer MCP-only, unprotected | PARTIAL — rate-limited, no auth |
| RC2-1 | Audit | Health counts all Dogs, not healthy ones | **FIXED** — system_health_status() |
| RC2-2 | Audit | No liveness/readiness separation | **FIXED** — /live + /ready |
| RC3-1 | Audit | No model name verification at boot | **FIXED** — verify_model_loaded() |
| RC4-1 | Audit | flush_usage dog_id unescaped | **FIXED** — sanitize + escape |
| RC5-1 | Audit | NullStorage.store_verdict returns Ok(()) | **FIXED** — returns Err |
| RC5-2 | Audit | Claim verification DB error → "race" | **FIXED** — propagates CoordError |
| RC7-1 | Audit | No trace IDs on any request | PARTIAL — request_id in pipeline span, no #[instrument] |
| RC7-2 | Audit | Multi-hop correlation impossible | OPEN — request_id not in PipelineDeps |
| RC8-1 | Audit | Real Tailscale IPs in tracked repo | **FIXED** |
| RC8-5 | Audit | CORS allow_methods(Any) too broad | **FIXED** — explicit methods |

## MEDIUM (22 open, 12 fixed)

| # | Source | Finding | Status |
|---|---|---|---|
| F5 | Stress | Sovereign Dogs can't handle concurrency (serial) | OPEN |
| F7 | Stress | SurrealDB intermittent 401 | OPEN (has retry) |
| F9 | Stress | Fake algebraic notation (f64, Rc4 match chess regex) | OPEN |
| F10 | Stress | "100%" undetected as absolute claim | OPEN |
| F11 | Stress | Context inflates unique_ratio | OPEN |
| F13 | Stress | CJK byte/char mismatch in validation | OPEN |
| F17 | Stress | VerdictCache key: no domain, no dogs filter | OPEN |
| F19 | Stress | Dogs filter ignored on cache hits | OPEN |
| RC1-4 | Audit | Error messages leak internal state | **FIXED** — sanitize_error() |
| RC1-5 | Audit | Agent impersonation (no length check) | **FIXED** — validate_agent_id() |
| RC2-3 | Audit | No startup probe (sleep 3) | OPEN — systemd |
| RC2-4 | Audit | cynic-health.timer orphaned | PARTIAL — renamed, not enabled |
| RC2-5 | Audit | No remote alerting | OPEN — ops/infra |
| RC3-2 | Audit | No config drift detection at runtime | OPEN |
| RC3-3 | Audit | CARGO_MANIFEST_DIR baked into binary | **FIXED** — runtime discovery |
| RC4-2 | Audit | sanitize_record_id collision | **FIXED** — percent-encoding |
| RC4-3 | Audit | sanitize_record_id no length limit | **FIXED** — chars().take(256) |
| RC5-3 | Audit | SSE serialization → empty event | **FIXED** — log + skip |
| RC5-4 | Audit | Probe reqwest → silent default | PARTIAL — logged, still fallback |
| RC5-5 | Audit | Integrity chain seed not logged | **FIXED** |
| RC5-6 | Audit | Metrics hydration silent | **FIXED** |
| RC5-7 | Audit | RwLock poison swallowed | **FIXED** — logged |
| RC5-8 | Audit | Observe handler 200 on dropped | **FIXED** — 503 |
| RC5-9 | Audit | dog_scores_json corrupt → empty | PARTIAL — logged, still empty |
| RC5-10 | Audit | llama API key permission silent | **FIXED** — logged |
| RC6-1 | Audit | Real IP in llama-server.service | OPEN — not in repo |
| RC6-2 | Audit | No llama-server unit in repo | OPEN |
| RC6-3 | Audit | No duplicate process detection | OPEN |
| RC6-4 | Audit | Zero security hardening in units | OPEN |
| RC8-2 | Audit | setup-ubuntu.sh 0.0.0.0 default | **FIXED** — 127.0.0.1 |
| RC8-3 | Audit | Makefile rollback/hotfix parse JSON | **FIXED** — status codes |
| RC8-4 | Audit | No cargo audit in pipeline | PARTIAL — conditional |

## LOW (16 open, 12 fixed/accepted)

| # | Source | Finding | Status |
|---|---|---|---|
| F1 | Stress | Rate limiter works for same-IP | PASS (not a bug) |
| F3 | Stress | Sovereign Dogs bottleneck 16-92s | Accepted — serial inference |
| F4 | Stress | DB slow queries under load | Accepted — expected |
| F6 | Stress | gemma parse failure (prompt format) | OPEN |
| F8 | Stress | SurrealDB tx conflict coord expire | Accepted — idempotent |
| F12 | Stress | DeterministicDog fully predictable | Accepted — by design |
| F18 | Stress | Direct-API crystals lack embeddings | OPEN (related to F15) |
| F21 | Stress | Dogs filter works (no cache hit) | PASS (not a bug) |
| F22 | Stress | /ready pings DB every call | OPEN |
| RC1-6 | Audit | Event injection via register | OPEN |
| RC3-4 | Audit | dirs::config_dir() silent fallback | PARTIAL — logged |
| RC4-4 | Audit | escape_surreal no backtick escape | **FIXED** |
| RC5-10 | Audit | llama API key permission silent | **FIXED** |
| RC6-5 | Audit | ExecStartPre=sleep 3 fragile | OPEN — systemd |
| RC8-6 | Audit | Restart=always on KAIROS | N/A — separate repo |

## Concurrency WARNs (Appendix A — 0 fixed, all benign)

| # | Finding | Status |
|---|---|---|
| A1 | HalfOpen allows 2 concurrent probes | OPEN (benign) |
| A2 | Verdict cache duplicate entries | OPEN (benign) |
| A3 | Usage flush snapshot/absorb gap | OPEN (safe by design) |
| A4 | Prometheus ratio non-atomic loads | OPEN (informational) |
| A5 | Blake3 hash chain serializes burst | OPEN |
| A6 | Circuit breaker filter stale | OPEN (benign) |
| A7 | Double crystal observation concurrent | OPEN |

---

## Compound Priority (what to fix next)

### Wave 1 — Epistemic Integrity
1. **F15** Crystal epistemic gate (CRITICAL)
2. **F17+F19** VerdictCache domain key (MEDIUM but compounds)
3. **F20** voter_count in verdict (CRITICAL)

### Wave 2 — Security
4. **F2** X-Forwarded-For → ConnectInfo
5. **F23** /events connection limit
6. **F13** content.chars().count()
7. **F16** Crystal observe append-only

### Wave 3 — DeterministicDog
8. **F9** Chess regex context-aware
9. **F10** "100%" detection
10. **F11** Content-only unique_ratio

### Wave 4 — Observability
11. **RC7** #[instrument] + request_id propagation
12. **F22** /ready caching

See `docs/SESSION-2026-03-25-RESEARCH.md` for concrete fix designs per finding.
