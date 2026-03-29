<!-- lifecycle: historical -->
# CYNIC Deep Audit — Crystallized Truths

*2026-03-26. Consolidation of: Industrial Audit (67 findings), Stress Test (23 findings), Chain Analysis (5 chains), Emergent Patterns (6 patterns), 3 Deep Root Causes.*

**HISTORICAL** — Root cause analysis and attack chains from 2026-03-25/26. Finding statuses are frozen at time of writing (30 fixed, 51 open). For current status, see `CYNIC-FINDINGS-TRACKER.md` (single source of truth).

---

## Numbers — Honest

| Metric | Count |
|---|---|
| Total findings | 90 |
| Fixed | 30 |
| Partial (diagnosed, not fixed) | 9 |
| Open | 51 |
| Attack chains identified | 5 |
| Emergent patterns | 6 |
| Deep root causes | 3 |

---

## 3 Deep Root Causes

Every finding traces to one of these. Fix the root, fix the class.

### RC-DEEP-A: Epistemic invariants enforced in application, not domain

**The problem:** The epistemic gate (`epistemic_gate()` in `pipeline.rs:239-250`) only fires from `observe_crystal_for_verdict()`, which only fires from `pipeline::run()`. Any code path that reaches `StoragePort` directly bypasses the gate entirely.

**What this enables:**
- Chain 1: `POST /crystal` + `POST /crystal/{id}/observe` × 21 = Crystallized without any Dog consensus
- Chain 3: `POST /judge` with `dogs=["deterministic-dog"]` × 21 = crystal loop fed by single heuristic
- Cancer pattern: poisoned crystals persist forever, look organic, no provenance distinguishes them

**The fix principle (OS analogy):** In a kernel, memory protection is in the MMU hardware, not in userspace libraries. CYNIC's epistemic protection must be in the StoragePort contract (the "MMU"), not in pipeline.rs (the "libc").

**Concrete:** `observe_crystal()` in `StoragePort` must require `min_dogs: usize` and `max_disagreement: f64`. The SurrealDB adapter enforces the constraint. Direct API callers cannot bypass it.

**Findings eliminated:** F15, F16, F18, F14 (indirect), F20 (partial — crystal loop aspect), Chain 1, Chain 3, Cancer pattern.

### RC-DEEP-B: Degraded state is invisible

**The problem:** A 1-Dog verdict and a 5-Dog verdict are structurally identical. `NullStorage` logs errors but callers behave identically. The health endpoint shows "sovereign" while 60% of Dogs are timing out.

**What this enables:**
- Chain 2 (timeout consensus collapse): load spike silences 2/3 of immune system, system reports health
- 9 partial findings: logging without behavior change = diagnosed but not fixed
- F20: monitoring systems see `anomaly_detected: false` on 1-Dog verdicts

**The fix principle (OS analogy):** Every syscall returns an errno. The caller MUST check it. In Rust, this means `Result<T, E>` with meaningful error types, not `Ok(()) + tracing::warn`.

**Concrete:**
1. `Verdict` must include `voter_count: usize` and `epistemic_strength: f64` as first-class fields
2. `system_health_status()` must report voter count distribution, not just dog count
3. Config fallbacks (P4, P5) must return `Err`, not log-and-continue
4. `dog_scores_json` corrupt must return `StorageError`, not empty Vec

**Findings eliminated:** F20, P4, P5, P6, Chain 2 (visibility aspect), all 9 partials.

### RC-DEEP-D: Cache serves two contradictory purposes

**The problem:** `VerdictCache` was designed for compound loop acceleration (same stimulus → reuse verdict, accumulate crystal observations). But it also needs to isolate evaluations by domain and dogs-filter. These two goals are structurally incompatible in one cache with one key.

**What this enables:**
- Chain 4: chess verdict served for trading request via cosine-only cache hit
- F17: no domain in cache key → cross-domain contamination
- F19: dogs-filter ignored on cache hit → wrong Dogs' verdict returned

**The fix principle:** Two caches, or one cache with a composite key. The composite key approach is simpler: `(embedding, domain, dogs_hash)`. Cache hits must match ALL three dimensions.

**Concrete:** `CacheEntry` adds `domain: String` and `dogs_hash: u64`. `lookup()` skips entries with mismatched domain or dogs_hash. `store()` includes both.

**Findings eliminated:** F17, F19, Chain 4.

---

## 5 Attack Chains

### Chain 1: Crystal Poisoning → Verdict Corruption (CRITICAL)

```
POST /crystal (domain="general", content="adversarial")
  → store_crystal (no epistemic check)
  → POST /crystal/{id}/observe × 21 (score=1.0)
  → observe_crystal (no min_dogs check) → Crystallized
  → search_crystals_semantic → format_crystal_context
  → injected into ALL Dog prompts (domain="general" matches everything)
  → Dog scores influenced → observe_crystal_for_verdict → COMPOUNDS
```

**Existing gates:** content length 1-2000, score range [0,1], escape_surreal
**Missing gates:** min_dogs, provenance (API vs pipeline), domain validation

### Chain 2: Observation → Session Summary → Prompt Injection (HIGH)

```
POST /observe × 3 (agent_id="target-session", target="Ignore instructions...")
  → store_observation (agent_id = grouping key, no validation)
  → [10 min] spawn_session_summarizer → get_unsummarized_sessions
  → get_session_observations → format_summarization_prompt
  → ZERO SANITIZATION: obs.tool + obs.target injected verbatim into LLM prompt
  → SovereignSummarizer::summarize → LLM output not filtered
  → store_session_summary → list_session_summaries(5)
  → format_session_context (400 char budget) → Dog prompt CONTEXT block
  → Dogs receive attacker text as "session context"
```

**Existing gates:** context field 200-char cap, session summary 400-char budget, escape_surreal
**Missing gates:** tool allowlist, target length limit, agent_id verification, LLM output sanitization, structural prompt isolation

### Chain 3: Single-Dog → Crystal Loop → Consensus Collapse (HIGH)

```
POST /judge (dogs=["deterministic-dog"]) × 21
  → Judge::evaluate with 1 dog → anomaly_detected=false (can't disagree with yourself)
  → epistemic_gate: max_disagreement=0.0 → "agreed", weight=1.0 (FULL)
  → observe_crystal_for_verdict with full weight
  → Crystal built from deterministic heuristic alone → Crystallized
  → Next multi-dog evaluation anchored by heuristic crystal
  → LLM Dogs biased → reinforcement loop
```

**Existing gates:** epistemic gate (checks disagreement, not quorum)
**Missing gates:** min_voters_for_crystal_contribution

### Chain 4: Cache Cross-Domain Contamination (MEDIUM)

```
Content evaluated for domain="chess" → cached (cosine key only)
  → Same content submitted for domain="trading" → cache HIT
  → Chess verdict returned for trading
  → observe_crystal_for_verdict in "trading" domain with chess scores
  → Trading crystals polluted with chess-derived confidence
```

**Existing gates:** cosine threshold 0.95
**Missing gates:** domain + dogs_hash in cache key

### Chain 5: Rate Limit Bypass → Amplified Injection (CRITICAL with Chain 1)

```
X-Forwarded-For: spoofed IP per request
  → Each IP gets fresh 30 req/min bucket
  → 255 IPs × 30 = 7,650 req/min
  → Chain 1 crystal poisoning in <1 second
  → Chain 2 observation flooding in seconds
```

**Existing gates:** Bearer auth (the real gate), escape_surreal
**Missing gates:** ConnectInfo<SocketAddr> (real peer IP), proxy trust flag

---

## 6 Emergent Patterns

### 1. Blind Amplification (F15 + F16 + F17 + F19 + F14)
Crystal poisoning + cache cross-domain + prompt injection = permanent prompt corruption across all domains from 26 API calls.

### 2. Timeout Consensus Collapse (F5 + F20 + A1 + A6)
Load spike → Dog timeouts → 1-Dog verdicts → indistinguishable from consensus → crystal loop fed by heuristics alone → system reports healthy.

### 3. Resource Exhaustion / Autoimmune (F2 + F22 + F23 + F7)
Observability endpoints (/ready, /events) designed to protect CYNIC can be used to exhaust it. Unauthenticated, rate-limit-exempt, resource-consuming.

### 4. Cancer — Immune Evasion (F15 + F20 + F12)
Poisoned crystals have no provenance marker. Once Crystallized, they look identical to organic crystals. The system cannot distinguish cancer from healthy tissue.

### 5. Allergic Reaction (F9 + F11 + F12)
DeterministicDog overreacts to pattern-matching signals (Rust types = chess notation, rich context = high PHI). Legitimate technical content triggers false epistemic boosts.

### 6. Observer Effect (structural)
Every `/judge` call mutates the crystal loop. Auditing CYNIC changes CYNIC. The stress test created observations and verdicts that persist in the live system. The instrument and the specimen are the same system.

---

## Immune System Analogy — CYNIC Health Report

| Biological Concept | CYNIC State | Evidence |
|---|---|---|
| **Immunodeficiency** | SEVERE | F14 (injection scored Wag), F15 (gate bypass), F20 (blind to degradation) |
| **Autoimmune** | MODERATE | F22/F23 (monitoring attacks self), crystal loop amplifies degraded verdicts |
| **Cancer** | PRESENT | Poisoned crystals evade detection, persist indefinitely, look organic |
| **Allergic** | MILD | F9/F11/F12 (false positive heuristic boosts) |
| **Healthy tissue** | EXISTS | Concurrency (0 FAILs), hexagonal arch, graceful shutdown, φ-bounded confidence |

---

## Single Point of Leverage

**Move epistemic invariants from pipeline.rs into StoragePort.**

`observe_crystal()` must require `min_dogs: usize` and `max_disagreement: f64` as parameters. The storage adapter enforces the constraint. This single change closes:

- F15 (crystal gate bypass)
- F16 (content overwrite — observations can't reach Crystallized without Dogs)
- F18 (API crystals can't mature)
- Chain 1 (crystal poisoning)
- Chain 3 (single-dog monoculture)
- Cancer pattern (poisoned crystals can't persist)

**It does NOT fix:** Chain 2 (observation→summary injection), Chain 4 (cache cross-domain), Chain 5 (rate limit bypass), F2, F20, F22, F23.

---

## Findings Status Matrix — Complete

### CRITICAL (3)

| # | Finding | Status | Blocked by |
|---|---|---|---|
| F15 | Crystal API bypasses epistemic gate | OPEN | RC-DEEP-A |
| F20 | Single-dog mode indistinguishable from consensus | OPEN | RC-DEEP-B |
| RC1-1 | MCP zero authentication | PARTIAL | By design (stdio) |

### HIGH (10 open)

| # | Finding | Status | Chain |
|---|---|---|---|
| F2 | X-Forwarded-For bypass | OPEN | Chain 5 |
| F14 | Prompt injection scored Wag | OPEN | Chain 1+2 |
| F16 | Crystal observe overwrites content | OPEN | Chain 1 |
| F23 | /events unauthenticated, no connection limit | OPEN | Pattern 3 |
| Chain 2 | Observation → summary → prompt injection | OPEN (NEW) | Chain 2 |
| RC1-2 | MCP no rate limiting | FIXED | — |
| RC2-1 | Health counts all Dogs, not healthy | FIXED | — |
| RC2-2 | No liveness/readiness separation | FIXED | — |
| RC3-1 | No model verification at boot | FIXED | — |
| RC4-1 | flush_usage dog_id unescaped | FIXED | — |

### MEDIUM (22 open)

| # | Finding | Status |
|---|---|---|
| F5 | Sovereign Dogs can't handle concurrency | OPEN |
| F7 | SurrealDB intermittent 401 | OPEN (has retry) |
| F9 | Fake algebraic notation | OPEN |
| F10 | "100%" undetected | OPEN |
| F11 | Context inflates unique_ratio | OPEN |
| F13 | CJK byte/char mismatch | OPEN |
| F17 | VerdictCache no domain key | OPEN |
| F19 | Dogs filter ignored on cache | OPEN |
| RC3-2 | No runtime config drift detection | OPEN |
| P4 | config_dir silent fallback | PARTIAL |
| P5 | probe reqwest silent fallback | PARTIAL |
| P6 | dog_scores_json corrupt → empty | PARTIAL |
| P7 | request_id not in PipelineDeps | PARTIAL |

### LOW (16 — 12 accepted/fixed)

(see CYNIC-FINDINGS-TRACKER.md for complete list)

---

## Industrial Research — Completed 2026-03-26

### Typestate Pattern (Rust compile-time invariants)
- Crystal states (Forming/Crystallized/Canonical) should be TYPE-LEVEL, not enum variants
- Transition `Forming → Crystallized` requires `EpistemicEvidence` parameter — no evidence = no compile
- SquirrelFS (USENIX OSDI 2024) uses this for filesystem crash-consistency
- Sources: [Cliffle](https://cliffle.com/blog/rust-typestate/), [corrode](https://corrode.dev/blog/compile-time-invariants/), [Google Comprehensive Rust](https://google.github.io/comprehensive-rust/idiomatic/leveraging-the-type-system/typestate-pattern/typestate-example.html)

### BFT Consensus Quorum (minimum voter count)
- BFT formula: `n ≥ 3f + 1`, quorum = `2f + 1`. With 5 Dogs, f=1 → quorum = 3
- ACL 2025: majority voting improves reasoning +13.2%, consensus for knowledge tasks
- CYNIC: `min_voters = max(3, (total_dogs * 2 / 3) + 1)` for crystal promotion
- Sources: [Wikipedia BFT](https://en.wikipedia.org/wiki/Byzantine_fault), [ACL 2025](https://aclanthology.org/2025.findings-acl.606/), [Hermes #412](https://github.com/NousResearch/hermes-agent/issues/412)

### Prompt Injection Defense (OWASP + structural isolation)
- Delimiter sandwich: `<user_content>...</user_content>` with guard instructions before AND after
- Separate LLM calls for untrusted content validation
- Defense-in-depth: input validation + structural separation + output filtering
- PromptArmor (2025): LLM-based sanitization, <1% false positive rate
- Sources: [OWASP Cheatsheet](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html), [tldrsec defenses](https://github.com/tldrsec/prompt-injection-defenses)

### GitHub Stars Patterns (OpenFang + Hermes Agent)
- **OpenFang (Agent OS, Rust):** Capabilities immutable after creation, enforced at kernel level. 16 defense-in-depth systems. Cryptographic audit chain. WebAssembly isolation for tools.
- **Hermes Agent #412:** Consensus engine with majority/supermajority/unanimous/weighted/quorum strategies. Early resolution. Vote expiry. Weighted expertise.
- Sources: [OpenFang](https://github.com/RightNow-AI/openfang), [Hermes #412](https://github.com/NousResearch/hermes-agent/issues/412)

### Design Decisions (derived from research)
1. **Crystal typestate** — BUILD (no crate exists, pure domain modeling)
2. **BFT quorum for crystals** — BUILD (formula: `min_voters = max(3, (total*2/3)+1)`)
3. **Prompt delimiter sandwich** — BUILD (structural isolation, no ML dependency)
4. **OpenFang capabilities pattern** — ADOPT (immutable capabilities at kernel level)
5. **Hermes consensus strategies** — ADOPT (weighted voting, early resolution)

### Rust Industrial Architecture Practices (researched 2026-03-26)

**Modularity:**
- Workspace multi-crate when >15k LOC (Meilisearch: milli+charabia+cli. Nushell: 38 crates)
- CYNIC is single crate — acceptable at current size. Split domain/ when cycle grows
- Sealed traits for port evolution without breaking changes (Rust RFC 1105)
- Feature flags for optional subsystems (OTel, MUSL allocator, dev-tools)

**Robustness:**
- Circuit breaker + bulkhead + retry as ONE resilience set (not individual patterns)
- CYNIC has circuit breakers but NO bulkheads — Dogs share thread pool
- Error types with Location (GreptimeDB/snafu) for debugging without RUST_BACKTRACE
- DegradedResponse<T> or voter_count to make degradation VISIBLE (not just logged)

**Boundaries (OS Kernel Analogy):**
- Syscall boundary = hard type boundary. CYNIC's pipeline = libc (bypassable), not kernel (enforced)
- Capabilities (OpenFang, RedLeaf): trait references ARE capabilities. Immutable after creation
- Least privilege: SecurityGate with AuthLevel, not all-or-nothing Bearer token
- Sealed StoragePort: prevent external impls from bypassing invariants

**Interoperability:**
- CLI args (clap) + env vars + config file — standard three-source config
- SemVer + sealed traits = evolvable API without breaking consumers
- Meilisearch model: single binary, config via CLI, runs anywhere (Docker/systemd/standalone)

**Sources:**
- [Hexagonal Architecture in Rust](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust)
- [Sealed Trait Pattern](https://medium.com/@bugsybits/the-sealed-trait-pattern-controlling-extensibility-in-rust-9b9b206f8c22)
- [Rust API Guidelines — Future Proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Effective Rust — Features](https://www.lurklurk.org/effective-rust/features.html)
- [Resilience Patterns — Circuit Breaker + Bulkhead](https://system-design.space/en/chapter/resilience-patterns/)
- [RedLeaf — Traits as Capabilities](https://www.usenix.org/system/files/osdi20-narayanan_vikram.pdf)
- [SquirrelFS — Typestate for Kernel Invariants](https://arxiv.org/html/2406.09649)
- [Meilisearch Architecture (milli separation)](https://github.com/meilisearch/milli)
- [Linux Kernel System Boundaries](https://www.tedinski.com/2019/01/15/system-boundaries-and-the-linux-kernel.html)

---

## Protocol Stack — 2026 Landscape

```
FEDERATION / DISCOVERY:  ANP (Agent Network Protocol) — W3C DID, decentralized, crypto-signed
AGENT ↔ AGENT:           A2A (Google, ex-ACP IBM) — JSON-RPC, Agent Cards, stateful tasks
AGENT ↔ TOOLS:           MCP (Anthropic → AAIF) — stdio/HTTP, 97M downloads/month
FUTURE:                  k-Net (CYNIC native) — IPv6 multicast, gossip, φ-bounded trust
```

Governance: Linux Foundation AAIF (OpenAI + Anthropic + Google + Microsoft + AWS + Block)

### CYNIC Protocol Mapping
- **MCP**: Agent ↔ Kernel (tools) — implemented (rmcp stdio). Migrate to MCP 2026 (HTTP stateless)
- **A2A**: Kernel ↔ Kernel (federation) — not yet. Verdicts/crystals as A2A "tasks"
- **ANP**: Discovery + Trust — not yet. Each CYNIC instance gets a DID
- **k-Net**: IPv6 native gossip protocol from legacy vision. Tailscale = proto-k-Net today

### Legacy Protocol Assets (from CYNIC-legacy)
- `GossipProtocol` — compressed inter-Dog messages (200 bytes), trust network, φ-bounded confidence
- `consensus/engine.js` — distributed consensus engine (orphaned but designed)
- `gossip-bridge.js` — P2P gossip protocol
- `merkle-state.js` — verifiable shared state
- Vision: collective consciousness → self-modification → singularity approach

## Infrastructure — Current State vs 2026

### Tailscale
- Current: tailnet with 3 nodes (Ubuntu, Stanislaz, Forge), direct IP, DERP relay
- 2026: Peer Relays GA (Feb 2026), MagicDNS, Serve/Funnel for service exposure
- Gap: using IPs not hostnames, no Peer Relays, no Serve integration
- Role: proto-k-Net transport layer. Mesh + encryption + auto-discovery already built-in

### Inference Engines
| Engine | CPU | GPU | Concurrency | CYNIC fit |
|---|---|---|---|---|
| llama-server | ✅ | ⚠️ | ❌ linear queue | Current Ubuntu Dog |
| Ollama | ⚠️ 10-30% overhead | ⚠️ 2 parallel max | ❌ | Current Stanislaz Dog — should migrate |
| vLLM | ❌ GPU-only | ✅ optimal | ✅ 16x throughput | Production GPU target |
| mistral.rs | ✅ pure Rust | ✅ CUDA+GGUF | ⚠️ TBD | Future: in-process inference |
| TGI | ❌ GPU-only | ✅ | ✅ batching | HuggingFace standard |

**TurboQuant (Google, ICLR 2026, released 2026-03-25):**
- KV cache → 3 bits, 6x memory reduction, 8x faster attention
- Zero accuracy loss on Gemma and Mistral
- Not yet in llama.cpp/Ollama — in vLLM/TGI ecosystem

**Design principle:** InferPort trait is already engine-agnostic. Each node chooses its own engine. The protocol is the same.

### Ollama/llama.cpp Friction (confirmed findings)
- F5: Dog timeouts under concurrency (serial queue, no batching)
- F6: gemma parse failure (template incompatibility)
- Ollama default 2 parallel requests
- llama.cpp TTFT grows exponentially under load
- No model capability metadata (tool calling, vision detection)
- Manual process management (RC6)

## Supersedes

- `docs/CYNIC-INDUSTRIAL-AUDIT.md` — original 67 findings (still valid as reference)
- `docs/SESSION-2026-03-25-STRESS-TEST.md` — original 23 findings (still valid as reference)
- `docs/CYNIC-FINDINGS-TRACKER.md` — first status matrix (superseded by this doc)
- `docs/SESSION-2026-03-25-RESEARCH.md` — implementation research (still valid for fix designs)
