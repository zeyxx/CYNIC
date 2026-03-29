# CYNIC Infrastructure Roadmap — Living Document

*Crystallized 2026-03-16. Updated as reality changes. Every claim is falsifiable.*

---

## Strategic Cap

CYNIC evolves from **judgment endpoint** to **sovereign inference proxy** — agents route through CYNIC for quality-gated, cost-optimized, sovereign-first inference.

```
Agent A (Claude Code)──┐
Agent B (Gemini CLI)───┼──▶ CYNIC ──▶ Route (sovereign/API/cache)
Agent C (OpenCode)─────┘      │       Judge (consensus quality gate)
                              │       CCM (shared memory across sessions)
                              │       Coord (anti-collision)
                              │       Audit (full traceability)
                              ▼
                        SurrealDB (shared state)
```

---

## Fleet — Probed Reality (2026-03-16)

### Ubuntu Desktop (Primary — Kernel Host)
- **Hardware:** Ryzen 7 5700G 8C/16T, 27 GB RAM, Vega 8 iGPU
- **Inference:** llama.cpp Vulkan build, Gemma 3 4B Q4_K_M, 26 GPU layers, 23 t/s gen
- **Services:** cynic-kernel + SurrealDB + llama-server (systemd)
- **Tailscale:** `<TAILSCALE_CORE>`

### S. Windows Desktop (GPU Worker)
- **Hardware:** i5-14400F 10C/16T, RTX 4060 Ti 16 GB VRAM, 16 GB RAM
- **Inference:** Ollama 0.17.7, Gemma 3 12B Q4_K_M (8.1 GB), full GPU offload, 43 t/s gen
- **Models available:** gemma3:12b-it-q4_K_M (active), qwen3.5:9b (thinking issue), qwen3-coder:30b, gpt-oss:20b
- **Access:** `tailscale serve --tcp 11434 tcp://localhost:11434` (Tailscale-only, no firewall needed)
- **Known issue:** Qwen 3.5 thinking mode — Ollama OpenAI API puts all output in `reasoning`, `content` empty
- **Tailscale:** `<TAILSCALE_S>`

### Forge (Backup — No Inference)
- **Hardware:** i5-6500T 4C/4T, 11 GB RAM, no GPU
- **Role:** Kernel replica only, no inference value
- **Status:** Offline

---

## Protocols & Transports — Decision Matrix

| Protocol | Purpose | CYNIC Status | Decision |
|----------|---------|-------------|----------|
| **MCP (stdio)** | Agent → kernel tools | ✅ Production | **KEEP** — zero attack surface, shared state via SurrealDB |
| **MCP (Streamable HTTP)** | Remote multi-agent | ❌ Not implemented | **WAIT** — 30 CVEs/60 days, spec unstable, June 2026 fix. REST API covers this |
| **REST (Axum)** | External clients, frontend | ✅ Production | **KEEP** — secured today (auth, per-IP rate limit, input validation) |
| **gRPC (tonic)** | Dog-to-dog, node-to-node streaming, A2A transport | ⚠️ Proto + services exist (MuscleHAL, KPulse, Vascular) — awaiting real backend wiring | **NEXT** — needed for streaming inference, real-time push notifications, inter-node communication |
| **A2A** | Agent-to-agent standard | ❌ Not implemented | **STRATEGIC** — implement when routing intelligence is ready |

### Why NOT Streamable HTTP (2026-03-16)
- 30 CVEs in 60 days across MCP ecosystem
- 36.7% of exposed MCP servers have SSRF vulnerabilities
- Session hijacking via Mcp-Session-Id (CVE-2025-6515)
- Spec authors acknowledge stateful scaling "doesn't work yet"
- TLS not mandated — plaintext is spec-compliant
- **CYNIC's REST API already handles remote access with Bearer auth on Tailscale**
- Re-evaluate after June 2026 spec revision

---

## Inference Topology

### Sovereign (local, zero API dependency)

| Machine | Model | Quant | VRAM/RAM | Speed | Status |
|---------|-------|-------|----------|-------|--------|
| Ubuntu | Gemma 3 4B | Q4_K_M | ~2.4 GB (Vulkan+CPU) | 23 t/s gen | ✅ Running |
| S. | Qwen 3.5 9B | Q8_0 (target) | ~9.5 GB VRAM | ~40 t/s (expected) | ❌ Down — needs Ollama install |

### API (rate-limited, cost-bearing)

| Backend | Model | Speed | Cost | Status |
|---------|-------|-------|------|--------|
| Gemini | Gemini 3 Flash Preview | 2-5s | Sponsor quota (limited) | ✅ Active |
| HuggingFace | Llama 3.1 8B Instruct | 3-8s | Free (rate limited) | ✅ Active |
| HuggingFace | Qwen 2.5 72B Instruct | 3-8s | Free (rate limited) | ✅ Active |

### Model Selection Rationale (for 16 GB VRAM)

Qwen 3.5 9B is the optimal sovereign model:
- GPQA Diamond 81.7 (beats models 3x larger)
- IFEval 91.5 (best instruction-following in class — critical for structured JSON)
- Q8_0 fits in 9.5 GB with 5.5 GB headroom
- Apache 2.0 license

Alternatives evaluated and rejected:
- DeepSeek-R1 14B: strong reasoning but verbose, weak JSON output
- Phi-4 14B: good math but IFEval 63.0 (too low for structured output)
- Gemma 3 12B: GPQA 40.9 (too low for judgment tasks)
- Any 24B+: exceeds 16 GB VRAM budget at Q4_K_M minimum

### Inference Evolution Path
```
NOW:   Ollama Windows (S.)     → 85-90% of max GPU perf
NEXT:  llama.cpp CUDA Linux (dual-boot) → 100% GPU perf
THEN:  vLLM Linux (concurrent agents)   → multi-request throughput
```

---

## Security Hardening (done 2026-03-16)

| Fix | File | Status |
|-----|------|--------|
| Input validation (content max 4000, context max 2000, reject empty) | `judge.rs` | ✅ |
| Default bind `127.0.0.1:3030` (was `0.0.0.0`) | `main.rs` | ✅ |
| Per-dog timeout 15s + partial results | `judge.rs` | ✅ |
| Per-IP rate limiting (token bucket per IP, 2min eviction) | `types.rs`, `middleware.rs` | ✅ |
| Constant-time API key comparison | `middleware.rs` | ✅ |
| CORS restricted to known origins + `CYNIC_CORS_ORIGINS` env | `mod.rs` | ✅ |
| Error messages sanitized (no internal leak) | `judge.rs` | ✅ |
| Rate limit double-counting fixed | `middleware.rs` | ✅ |

### Remaining
- Deploy script missing `CYNIC_API_KEY` in env file
- No TLS (acceptable on Tailscale, not for public exposure)
- 170 unwrap() in 5000 LOC (crash risk on unexpected input)
- Audit log missing source IP

---

## Agent Orchestration Architecture

### How agents connect today
```
Claude Code ──stdio──▶ cynic-kernel --mcp ──▶ SurrealDB (shared state)
                                              ├── Coord (claims, agents)
                                              ├── Verdicts
                                              ├── Crystals (CCM)
                                              └── Audit trail
```
N agents = N stdio processes, all sharing the same DB. This works.

### Stdio limitations (acceptable for now)
- 1:1 (one process per agent) — low overhead (~10 MB each)
- Local only (agent must run binary) — REST API bridges remote
- No server→agent push — agents poll via `coord_who`

### Agent control interfaces
| Agent | Headless mode | Structured output | MCP support | Orchestrable by CYNIC? |
|-------|--------------|-------------------|-------------|----------------------|
| Claude Code | `claude -p --output-format stream-json` | ✅ | ✅ Native | ✅ via Agent SDK |
| Gemini CLI | `gemini -p --output-format stream-json` | ✅ | ✅ Native | ✅ via CLI |
| OpenCode | CLI mode | ✅ | ✅ | ✅ via CLI |

### Claude Agent SDK (production-ready)
- Spawn Claude agents programmatically with CYNIC MCP pre-configured
- Hooks: PreToolUse, PostToolUse, SessionStart, SessionEnd
- Subagent support (Agent tool)
- Session resume/fork
- This is the primary orchestration vector

### Claude Code hidden TeammateTool (feature-gated)
- 13 operations: spawnTeam, broadcast, requestJoin, approvePlan...
- Filesystem coordination at `~/.claude/teams/`
- Architecture mirrors CYNIC coord — when ungated, CYNIC could be the backend

---

## Feature Compound Map

### Phase 1 — Sovereign Foundation (DONE 2026-03-16)
```
✅ Install Ollama on S. via Tailscale MCP — Gemma 3 12B Q4_K_M, 43 t/s, 6.1s latency
✅ Fix gemma-sovereign auth — SOVEREIGN_API_KEY set in ~/.cynic-env
✅ Sovereign in backends.toml — Ollama on S. via tailscale serve :11434
✅ 6 Dogs active (deterministic + gemini + huggingface + qwen-hf + sovereign + gemma-sovereign)
✅ Benchmark: Sicilian=HOWL Q=0.589, 6/6 Dogs healthy
⚠️ Qwen 3.5 9B thinking mode breaks Ollama OpenAI API (content empty, all in reasoning)
   → Switched to Gemma 3 12B. Qwen 3.5 needs Ollama native API adapter or per-request thinking control.
```

### Phase 2 — Intelligence Loops
```
✅ CCM feedback loop — crystals injected into Dog prompts (format_crystal_context, 800 char budget)
□ Thinking mode control — per-REQUEST (not per-backend) enable/disable:
    - BackendConfig: api_style (openai/ollama/vllm), thinking (always/never/auto), max_thinking_tokens
    - Stimulus.complexity hint (Simple/Deep/Auto) → routing decides think on/off
    - Simple tasks (chess judgment, classification) → think:false, fast
    - Deep tasks (architecture analysis, debugging) → think:true, more tokens
    - Requires OllamaBackend adapter (native /api/chat, not OpenAI compat) for think param
    - Qwen 3.5 thinking buggy on llama.cpp (#20182, #20196) — Ollama native works
    - JSON structured output + thinking are fundamentally in tension (grammar suppresses <think> token)
□ Temporal integration — call aggregate_temporal in Judge, expose in verdict
□ cynic_infer routing — replace pass-through with BackendRouter dispatch
□ Auto-e2e post-deploy — chain /e2e after /deploy in Makefile
```

### Phase 3 — Multi-Agent Velocity
```
□ Study rtk-ai/rtk token reduction patterns
□ Study Thinklanceai/agentkeeper memory persistence
□ Study claude-mem session capture patterns
□ Automate coord in hooks (no manual claim/release)
□ Claude Agent SDK integration prototype
□ Gemini CLI orchestration prototype
```

### Phase 4 — Proxy d'Inférence
```
□ BackendRouter complete (model hint, latency-weighted, fallback chain)
□ /v1/chat/completions endpoint (OpenAI-compatible proxy)
□ Token caching (rtk-style, reduce API consumption 60-90%)
□ Routing intelligence (simple→sovereign, complex→API)
□ CCM informs routing ("chess → sovereign scores better")
```

### Phase 5 — Inter-Agent Protocol
```
□ A2A Agent Cards for each Dog
□ gRPC transport for dog-to-dog (replace hardcoded pulse)
□ A2A task lifecycle mapped to verdict lifecycle
□ External agent discovery via A2A
```

### Phase 6 — Scale
```
□ S. dual-boot Linux
□ llama.cpp CUDA native build
□ vLLM for concurrent inference
□ LMCache for KV cache optimization
```

---

## Starred Repos — Study Queue

| Repo | Why | Priority |
|------|-----|----------|
| `rtk-ai/rtk` | Token reduction proxy — blueprint for CYNIC proxy | 🔴 High |
| `Thinklanceai/agentkeeper` | Cross-model memory — CCM evolution reference | 🔴 High |
| `thedotmack/claude-mem` | Session capture — solves "context lost" bottleneck | 🔴 High |
| `RightNow-AI/openfang` | Agent OS in Rust — architecture reference | 🟡 Medium |
| `code-yeongyu/oh-my-openagent` | Multi-provider harness — orchestration patterns | 🟡 Medium |
| `alibaba/zvec` | Vector DB for CCM embeddings | 🟡 Medium |
| `ForLoopCodes/contextplus` | Semantic RAG MCP server — enrich Dog prompts | 🟡 Medium |
| `LMCache/LMCache` | KV cache for vLLM — Phase 6 | 🔵 Later |
| `tripleyak/SkillForge` | Skill router — compound with 17 CYNIC skills | 🔵 Later |

---

## Bottlenecks — User-Reported (2026-03-16)

1. **Contexte perdu entre sessions** → CCM feedback loop + claude-mem patterns
2. **Coordination manuelle** → Automate coord in hooks (claim on edit, release on session end)
3. **Coût API / rate limits** → Sovereign inference + token caching (rtk patterns)
4. **Workflow mécanique** → Automate repetitive steps, keep creative/research work for human

## Design Principles

- **Sovereign first, APIs as hedge** — local inference must work before relying on rate-limited APIs
- **Measure then fix** — never diagnose blindly, probe reality, falsify hypotheses
- **Quality > quantity** — max velocity without compromising quality
- **stdio + REST > Streamable HTTP** — zero attack surface beats convenience
- **Compound interest** — each phase enables the next, skip nothing

---

*φ distrusts φ — max confidence 61.8%*

© 2026 — All rights reserved.
