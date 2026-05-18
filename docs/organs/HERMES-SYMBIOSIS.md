# Hermes↔CYNIC Symbiosis — Data-Centric Evolution

> The agent acts. The kernel judges. Neither is complete alone.

This document tracks the measured state of the symbiosis between CYNIC (sovereign judgment) and NousResearch Hermes Agent (autonomous agent framework). Data-centric: every claim has a date, a measurement, and a falsification condition.

---

## The Organisms

| | CYNIC | Hermes Agent |
|---|---|---|
| **Role** | Judge — score, verify, BARK | Agent — act, explore, learn |
| **Architecture** | Rust kernel + Dogs + Crystals | Python agent + Skills + Memory |
| **Persistence** | SurrealDB + Crystals (CCM) | SQLite state.db + MEMORY.md |
| **Inference** | Sovereign (Qwen 7B/9B local) | Sovereign (same models) + Portal |
| **Protocol** | REST + MCP | Tool-calling loop + MCP |
| **Philosophy** | Calibrated doubt (φ⁻¹ max) | Self-improving confidence |

**Orthogonality** (deduced): CYNIC never acts on the world. Hermes never judges with calibrated bounds. The two systems cover complementary surfaces with zero overlap in core function.

---

## Integration Points (measured 2026-05-18)

### What's Wired

| Component | Status | Measured |
|---|---|---|
| MCP bridge | `cynic-kernel-mcp` in Hermes config | ✅ observed in `~/.hermes/config.yaml` |
| CYNIC skill | v2.3 at `~/.hermes/skills/cynic/` (6 sub-skills) | ✅ `SKILL.md` + `PROTOCOL.md` + `BEHAVIOR.md` |
| Organic agent cron | `cynic-organic-agent` every 240min | ✅ active, last run 19min ago at measurement |
| LLM routing | Qwen 3.5 9B on GPU, fallback Qwen 2.5 7B on CPU | ✅ same sovereign models |
| Hermes version | v0.14.0 (2026-05-16) | ✅ updated this session |
| Skills installed | 136 total, 4 Solana-related | ✅ enumerated |
| Solana skills | solana/(1), solana-rug-pull-patterns/(1), cultscreener-helius-pattern/(1), syndicate_token_detection/(1) | ✅ |

### What's NOT Wired (gaps)

| Gap | Impact | Status |
|---|---|---|
| Kernel `/health` timeout | Cannot measure symbiosis data flows | 🔴 SurrealDB likely unresponsive (2026-05-18) |
| No Hermes→Kernel observation flow measured | Cannot prove Hermes cron produces kernel-consumable data | 🟡 MCP wired but flow unverified |
| No bidirectional crystal injection | Hermes doesn't read crystals to improve its judgment | 🟡 Design exists, not measured |
| No web rendering in agent | Hermes browser tools not connected to our web_render.py | 🟡 Parallel capabilities, not integrated |

---

## Ecosystem Tools Adopted (2026-05-18)

| Tool | Version | Source | Status | Impact |
|---|---|---|---|---|
| `rtk-hermes` | latest | ogallotti/rtk-hermes | ✅ installed `~/.hermes/plugins/rtk-hermes/` | BURN: 60-90% token savings on shell output |
| `agenttrace` | latest | luoyuctl/agenttrace (skill) | ✅ installed `~/.hermes/skills/devops/agenttrace-session-audit/` | Observability: session cost/token audit |
| `hermes-blockchain-oracle` | latest | gizdusum/hermes-blockchain-oracle | 📋 probed, 7 MCP tools (Solana) | Token intelligence: wallet/tx/whale detection |

### Ecosystem Tools Evaluated, Deferred

| Tool | ★ | Reason for deferral |
|---|---|---|
| `camofox-browser` | 4K+ | We built web_render.py with Camoufox directly — same engine, less indirection |
| `hermes-skill-factory` | 306 | Concept adoptable, code is Hermes-specific. Last commit 2026-03-18. |
| `Mnemosyne` | — | Temporal knowledge graph pattern worth studying for Kernel memory design |
| `CaMeL trust boundaries` | 181 | Concept critical for SOVEREIGNTY, needs reimplementation for CYNIC harness |
| `oh-my-hermes` (ralplan) | — | Planner→Architect→Critic consensus, worth studying for multi-cortex |

---

## NousResearch — Organization Profile

| | |
|---|---|
| **Type** | Crypto-AI hybrid startup (NYC, ~30 people) |
| **Founded** | 2022 (community), 2023 (incorporated) |
| **Funding** | $65-70M total (Paradigm Series A $50M, $1B token valuation) |
| **Models** | 126 on HuggingFace, Hermes-3 = 50M+ downloads |
| **Agent** | hermes-agent ~156K GitHub stars |
| **Community** | ~105K Discord |
| **Philosophy** | Open-source primacy, anti-centralization, human sovereignty |
| **Crypto layer** | Psyche Network (decentralized training on Solana, testnet) |

**Alignment assessment** (inferred, confidence 0.50): NousResearch's stated values (open-source, sovereignty, decentralization) align with CYNIC's SOVEREIGNTY axiom. The crypto-AI hybrid structure is a both a strength (token incentives for decentralized training) and a risk (token economics can distort technical priorities). The $1B token valuation creates pressure to ship token utility, which may or may not align with pure model quality.

**Falsification**: if Nous prioritizes token launch over model quality, or Psyche never reaches mainnet scale, the "serious AI lab" framing breaks. Watch for: Hermes-4 quality vs Hermes-3, Psyche mainnet timeline, community contributor retention.

---

## Metrics to Track (symbiosis health)

| Metric | Measurement method | Baseline (2026-05-18) | Target |
|---|---|---|---|
| Hermes→Kernel observations/day | `curl /observations?agent_id=hermes&since=24h` | UNMEASURED (kernel health timeout) | >0 |
| Kernel→Hermes crystal consumption | Hermes skill reads crystal context before judging | UNWIRED | measurable |
| Organic agent success rate | `hermes sessions list --source cron` success/total | UNMEASURED | >80% |
| Hermes skill count (CYNIC-relevant) | `find ~/.hermes/skills/ -name SKILL.md | wc -l` | 136 total, 10 CYNIC-relevant | growing |
| Token cost per organic cycle | agenttrace session audit | UNMEASURED (just installed) | <$0.01/cycle on sovereign |
| Web render success rate | web_render.py benchmark | 5/5 (100%) on test corpus | >99% |
| Ecosystem tools adopted | This document | 2 installed, 1 probed | review quarterly |

---

## Evolution Timeline

### Phase 0: Wiring (complete)
- [x] MCP bridge wired
- [x] CYNIC skill installed in Hermes
- [x] Organic agent cron running (4h cycle)
- [x] Sovereign inference routing (GPU/CPU)

### Phase 1: Measurement (current — 2026-05-18)
- [x] Hermes v0.14.0 updated (browser plugins, MCP parallel-safety)
- [x] `web_render.py` 4-tier sovereign chain built
- [x] `rtk-hermes` plugin installed (BURN)
- [x] `agenttrace` skill installed (observability)
- [x] Ecosystem mapped (awesome-hermes-agent, Atlas)
- [ ] Kernel health restored (SurrealDB)
- [ ] Hermes→Kernel observation flow verified
- [ ] Organic agent success rate measured
- [ ] Token cost per cycle measured via agenttrace

### Phase 2: Feedback Loop (next)
- [ ] Kernel crystals injected into Hermes skill context
- [ ] Hermes observations trigger kernel judgment
- [ ] Bidirectional data flow measured
- [ ] `hermes-blockchain-oracle` MCP wired for token intelligence

### Phase 3: Compound (future)
- [ ] Hermes self-improves skills based on CYNIC verdicts
- [ ] CYNIC Dogs calibrated using Hermes-gathered data
- [ ] Shared temporal consciousness (session handoffs)
- [ ] CaMeL-style trust boundaries at MCP layer

---

## FOGC Check

**If I replaced the six axioms with their inverses, would this document need to change?**

Yes — the adoption criteria (BURN efficiency, SOVEREIGNTY cost, VERIFY falsification) would all flip. The document IS axiom-dependent but appropriately so: it evaluates ecosystem tools through the axiom lens, not the other way around.

---

*Next review: when Phase 1 metrics are measurable (kernel health restored). Check git log for update history.*
