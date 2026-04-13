# CYNIC Findings Tracker — v0.8.0-dev

*Updated 2026-03-29. Honest inventory — no overclaims.*

**Sources:** Industrial Audit (67 findings, 2026-03-24) + Stress Test (23 findings, 2026-03-25)
**Total: 90 findings. 43 fixed. 9 partial/accepted. 38 open.**

**v0.8 Wave 0+1 (2026-03-27):** Crystal lifecycle integrity + consensus enforcement.
Defends the crystal feedback loop (permanent damage path). Does NOT fix direct stimulus injection (fundamental LLM limitation, mitigated by multi-Dog consensus — arxiv 2504.18333).

**v0.8 Wave 2 (2026-03-27):** Live bug fixes + straightforward findings.
2 live production bugs (usage flush, integrity chain SQL). 6 findings (F2, F13, F22, F23, RC1-6 + 2 unlisted DB bugs). Tests: +6 regression (F13 CJK, F22 ReadyCache, F23 SSE limit, RC1-6 x2, coord control chars).

---

## CRITICAL (1 open, 2 fixed)

| # | Source | Finding | Status |
|---|---|---|---|
| F15 | Stress | Crystal API bypasses epistemic gate — 25 calls = Crystallized | **FIXED** — v0.8: REST observe returns 422 (quorum gate T8), StoragePort rejects voter_count < MIN_QUORUM. POST /crystal still creates Forming (harmless — can't promote without Dogs). |
| F20 | Stress | Single-dog mode indistinguishable from consensus | **FIXED** — v0.8: voter_count in Verdict struct + REST response. Quorum gate (T8) blocks single-Dog crystallization. Verdict still served (availability). |
| RC1-1 | Audit | MCP zero authentication (stdio = process trust, but no shared ServiceLayer) | PARTIAL — rate limit added, no auth |

## HIGH (8 open, 8 fixed)

| # | Source | Finding | Status |
|---|---|---|---|
| F2 | Stress | X-Forwarded-For spoofing bypasses rate limiter | **FIXED** — v0.8w2: ConnectInfo peer addr, X-Forwarded-For removed. Verified: 30 requests then 429 despite spoofed headers. |
| F14 | Stress | Prompt injection scored Wag by LLM Dogs | ACCEPTED — **fundamental LLM limitation (OWASP LLM01:2025). Mitigations: multi-Dog consensus, epistemic gate, φ-bounding, crystal amplification defended (T7+T8). Structural isolation (ChatPort multi-turn) planned v0.9 (StruQ USENIX Sec'25). Accepted 2026-03-29: risk inherent to LLM inference, mitigations proportional.** |
| F16 | Stress | Crystal observe overwrites content | **FIXED** — v0.8: SQL `content = content ?? '{content}'` (set-once, event sourcing pattern). Content preserved from first observation. |
| F23 | Stress | /events unauthenticated, no connection limit, FD exhaustion | **FIXED** — v0.8w2: sse_semaphore(32) limits concurrent SSE connections. 503 when full. Permit held via stream lifetime. Still unauthenticated (operational data). |
| RC1-2 | Audit | MCP no rate limiting | **FIXED** — McpRateLimit 10/min judge, 30/min other |
| RC1-3 | Audit | cynic_infer MCP-only, unprotected | PARTIAL — rate-limited, no auth |
| RC2-1 | Audit | Health counts all Dogs, not healthy ones | **FIXED** — system_health_status() |
| RC2-2 | Audit | No liveness/readiness separation | **FIXED** — /live + /ready |
| RC3-1 | Audit | No model name verification at boot | **FIXED** — verify_model_loaded() |
| RC4-1 | Audit | flush_usage dog_id unescaped | **FIXED** — sanitize + escape |
| RC5-1 | Audit | NullStorage.store_verdict returns Ok(()) | **FIXED** — returns Err |
| RC5-2 | Audit | Claim verification DB error → "race" | **FIXED** — propagates CoordError |
| RC7-1 | Audit | No trace IDs on any request | **FIXED** — request_id in pipeline span + #[instrument] on all Dog/Judge/Storage methods. Commit a2472ae. |
| RC8-1 | Audit | Real Tailscale IPs in tracked repo | **FIXED** |
| RC7-2 | Audit | Multi-hop correlation — CYNIC needs request tracing | **FIXED** — v0.7.7-115: #[instrument] on Dog evaluate() (deterministic + inference), Judge evaluate/evaluate_progressive, and all StoragePort impl methods. Full multi-hop trace from REST/MCP → Judge → Dog → Storage. Commit a2472ae. |
| RC8-5 | Audit | CORS allow_methods(Any) too broad | **FIXED** — explicit methods |
| CH2 | Chain | Observation → session summary → prompt injection via unsanitized obs.target in summarizer prompt | **FIXED** — v0.7.4: sanitize_observation_target() strips directives + 256-char cap at /observe input. Defense-in-depth with existing 200-char context cap and 400-char summary budget. |

## MEDIUM (22 open, 12 fixed)

| # | Source | Finding | Status |
|---|---|---|---|
| F5 | Stress | Sovereign Dogs can't handle concurrency (serial) | OPEN |
| F7 | Stress | SurrealDB intermittent 401 | OPEN (has retry) |
| F9 | Stress | Fake algebraic notation (f64, Rc4 match chess regex) | OPEN |
| F10 | Stress | "100%" undetected as absolute claim | OPEN |
| F11 | Stress | Context inflates unique_ratio | OPEN |
| F13 | Stress | CJK byte/char mismatch in validation | **FIXED** — v0.8w2: .chars().count() in all free-text validation (REST+MCP content, context, prompt, intent). Regression test: 1000 CJK chars (3000 bytes) accepted. |
| F17 | Stress | VerdictCache key: no domain, no dogs filter | **FIXED** — v0.8w0: CacheContext newtype (domain + dogs_hash). Lookup skips mismatched entries. 3 contract tests. |
| F19 | Stress | Dogs filter ignored on cache hits | **FIXED** — v0.8w0: CacheContext.dogs_hash from Judge::available_dogs_hash(filter). Different Dog config = cache miss. |
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
| F18 | Stress | Direct-API crystals lack embeddings | OPEN (reduced risk — can't crystallize via REST, backfill job exists) |
| F21 | Stress | Dogs filter works (no cache hit) | PASS (not a bug) |
| F22 | Stress | /ready pings DB every call | **FIXED** — v0.8w2: ReadyCache (AtomicBool+AtomicU64, 30s TTL). First /ready call pings, subsequent use cache. |
| RC1-6 | Audit | Event injection via register | **FIXED** — v0.8w2: intent 1-500 chars, agent_type/claim_type ≤64, target ≤256 on all coord endpoints. Storage layer already uses escape_surreal(). |
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

## v0.8 What Was Actually Done

### Crystal Lifecycle Integrity (Wave 0+1)
- **MatureCrystal newtype** (T4) — compile-time enforcement: only Crystallized|Canonical reach Dog prompts
- **Content sanitization** (T7) — directive stripping at StoragePort level, delimiter wrapping at read time
- **Content set-once** (F16) — SQL `content = content ?? new` prevents observation drift
- **voter_count** (T5/T9) — explicit field in Verdict, persisted to DB, visible in REST response
- **Quorum gate** (T8) — pipeline + StoragePort reject observations with voter_count < 2
- **REST observe blocked** — POST /crystal/{id}/observe returns 422 (quorum required)

### Live Bug Fixes + Straightforward Findings (Wave 2)
- **Usage flush** — REMOVE INDEX dog_usage_id_idx (redundant UNIQUE constraint broke UPSERT)
- **Integrity chain** — SELECT must include ORDER BY field (SurrealDB 2.x requirement)
- **F2** — ConnectInfo peer addr replaces X-Forwarded-For (Tailscale = no proxy)
- **F13** — .chars().count() in all 10 free-text validation points (REST+MCP+DeterministicDog)
- **F22** — ReadyCache (AtomicBool, 30s TTL) avoids DB ping on every /ready probe
- **F23** — sse_semaphore(32) limits concurrent SSE connections, 503 when full
- **RC1-6** — Input validation on all coord endpoint fields (intent, agent_type, claim_type, target)

### What v0.8 Does NOT Fix (honest)
- **F14 (direct stimulus injection)** — fundamental LLM limitation. Multi-Dog consensus is the defense. Structural isolation (ChatPort multi-turn) is v0.9.
- **F17/F19 (cache cross-domain)** — **Already fixed** in v0.8w0 (CacheContext). Tracker was stale.
- **F6** — gemma parse failure (needs Dog prompt research).
- **DeterministicDog (F9/F10/F11)** — needs research on false positive patterns.
- **Observability (RC7)** — needs request_id propagation design.
- **Infrastructure (RC2/RC6)** — systemd/ops work.

## Compound Priority (remaining)

### Next
1. **F6** gemma parse failure (Dog prompt research)

### Research Required
3. **F9/F10/F11** DeterministicDog heuristics
4. **RC7** Observability propagation

### v0.9 Architectural
5. **F14** ChatPort multi-turn for structural stimulus isolation (StruQ USENIX Sec'25)
