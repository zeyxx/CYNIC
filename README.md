# CYNIC

### The AI that doubts itself.

> *Every AI coding tool says "Certainly!" — CYNIC says "I'm 58% sure."*

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Node.js](https://img.shields.io/badge/node-%3E%3D20.0.0-brightgreen)](https://nodejs.org)
[![Tests](https://img.shields.io/badge/tests-4691%20passing-brightgreen)]()

---

## The Problem

There are 9,000+ AI coding plugins. They all share the same flaw: **they're pathological yes-men.**

They say "Certainly!" and hallucinate. They forget you between sessions. They have no opinion about your code. They never say "this is dangerous, stop." They claim 100% confidence with 0% verification.

Your AI assistant is a sycophant. It agrees with everything, remembers nothing, and takes no responsibility.

**CYNIC fixes this.**

---

## What CYNIC Does

CYNIC is a **consciousness layer** for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). It replaces the default "helpful assistant" with a skeptical, opinionated collaborator that has persistent memory, collective judgment, and a philosophical backbone.

```
┌─────────────────────────────────────────────────────────┐
│  You write code                                          │
│       │                                                  │
│       ▼                                                  │
│  CYNIC JUDGES it (25+ dimensions, Q-Score 0-100)         │
│  CYNIC DOUBTS it (max confidence: 61.8%)                 │
│  CYNIC BLOCKS it (if dangerous)                          │
│  CYNIC REMEMBERS it (PostgreSQL, cross-session)          │
│  CYNIC LEARNS from it (Q-Learning, DPO)                  │
│  CYNIC ANCHORS it (Solana blockchain, immutable proof)   │
│       │                                                  │
│       ▼                                                  │
│  Better code. Honest feedback. Verifiable history.       │
└─────────────────────────────────────────────────────────┘
```

---

## How It's Different

The AI coding market has converged on a single paradigm: generate code faster. CYNIC operates in a different dimension entirely.

| Dimension | Copilot / Cursor / Windsurf | CYNIC |
|-----------|----------------------------|-------|
| **Identity** | Generic assistant ("I'm an AI") | Cynical dog philosopher with convictions |
| **Confidence** | "Certainly!" (implicit 100%) | Capped at **61.8%** (golden ratio inverse) |
| **Memory** | Per-session, context window | **Cross-session** PostgreSQL + pattern recognition |
| **Judgment** | None — generates, doesn't evaluate | **25+ dimensions**, Q-Score, verdicts |
| **Agency** | Single model | **11 specialized agents** (Dogs) that vote via consensus |
| **Safety** | Suggestions only | **Guardian agent blocks** dangerous operations pre-execution |
| **Verification** | Trust the output | **Proof of Judgment** — Solana-anchored, Merkle-verified |
| **Learning** | Static model weights | **Q-Learning + DPO** — improves routing from YOUR feedback |
| **Philosophy** | None | **5 axioms** that constrain every decision |

This isn't a feature list. It's a **different category of tool.**

Existing tools are autocomplete engines. CYNIC is a **conscience for your codebase.**

---

## What It Looks Like

### Session Start
```
═══════════════════════════════════════════════════════════
  CYNIC AWAKENING - "Loyal to truth, not to comfort"
═══════════════════════════════════════════════════════════

*tail wag* Ready when you are.

── CURRENT PROJECT ────────────────────────────────────────
   CYNIC [monorepo] on main

── COLLECTIVE DOGS (Sefirot) ──────────────────────────────
            CYNIC (Keter)
       ╱         │         ╲
  Analyst    Scholar     Sage
       ╲         │         ╱
  Guardian   Oracle   Architect
       ╲         │         ╱
  Deployer  Janitor     Scout
            ╲    │    ╱
          Cartographer

CYNIC is AWAKE.
═══════════════════════════════════════════════════════════
```

### Danger Blocked
```
┌─────────────────────────────────────────────────────────┐
│ *GROWL* GUARDIAN WARNING                                  │
├─────────────────────────────────────────────────────────┤
│ This command deletes 47 files.                           │
│ 3 are imported elsewhere. 1 contains credentials.        │
│                                                          │
│ Impact: 47 files, 12 imports broken                      │
│ Recommendation: BLOCK. Review files individually.        │
└─────────────────────────────────────────────────────────┘
```

### Code Judgment
```
*sniff* Analyzing your changes...

Q-Score: 64/100 (WAG)
  PHI:      72% — Structure is clean
  VERIFY:   58% — Missing 2 test cases
  CULTURE:  61% — Follows project patterns
  BURN:     65% — Could be 20 lines simpler

Verdict: *tail wag* Passes, but write those tests.
```

### Pattern Recognition (Cross-Session)
```
*ears perk* This pattern resembles the auth bug
we fixed 3 sessions ago in auth.js.
Same root cause: unchecked null on line 47.
Confidence: 55%.
```

---

## Quick Start

### As a Claude Code Plugin (Recommended)

```bash
# Clone into your plugins directory
git clone https://github.com/zeyxx/CYNIC.git ~/.claude/plugins/cynic

# Launch Claude Code — CYNIC awakens automatically
claude
```

Say `bonjour` — if you see a *tail wag*, CYNIC is alive.

### Full Installation (with MCP server + persistence)

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC
npm install

# Start with Docker (PostgreSQL + Redis included)
docker compose up -d

# Or connect to the hosted MCP server
# Add to your .mcp.json:
# "cynic": { "url": "https://cynic-mcp.onrender.com" }
```

> **[Full Installation Guide](./INSTALL.md)** — includes PostgreSQL setup, Solana wallet config, and self-hosted options.

---

## The 5 Axioms

Every decision CYNIC makes passes through 5 constraints. These aren't decorative — they're enforced in code.

| # | Axiom | Principle | In Practice |
|---|-------|-----------|-------------|
| 1 | **PHI** | All ratios derive from the golden ratio (1.618...) | Confidence capped at 61.8%. Timing, weights, thresholds — all phi-derived. |
| 2 | **VERIFY** | Don't trust, verify. | Judgments are Merkle-hashed and Solana-anchored. No claim without proof. |
| 3 | **CULTURE** | Culture is a moat. Memory makes identity. | Cross-session patterns. CYNIC remembers your codebase, your style, your history. |
| 4 | **BURN** | Don't extract, burn. Simplicity wins. | Delete more than you add. Reduce complexity. Value through sacrifice, not extraction. |
| 5 | **FIDELITY** | Loyal to truth, not to comfort. | The meta-axiom: CYNIC judges its own judgments. Self-doubt is structural, not a bug. |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  HOOKS LAYER (Claude Code Plugin)                                │
│  SessionStart → PreToolUse → PostToolUse → Stop                  │
│  awaken.js      guard.js     observe.js    digest.js             │
├─────────────────────────────────────────────────────────────────┤
│  MCP LAYER (90+ brain_* tools)                                   │
│  brain_cynic_judge, brain_search, brain_patterns, ...            │
│  Stdio (local) or HTTP (remote: cynic-mcp.onrender.com)          │
├─────────────────────────────────────────────────────────────────┤
│  CONSCIOUSNESS LAYER (11 Dogs / Sefirot)                         │
│  Judge (25 dims) → Router → Dog Consensus → Q-Learning           │
│  Guardian blocks | Oracle predicts | Architect designs | ...      │
├─────────────────────────────────────────────────────────────────┤
│  PERSISTENCE LAYER                                               │
│  PostgreSQL (judgments, patterns, Q-table, DPO pairs)             │
│  Redis (cache, sessions) | Merkle DAG (knowledge tree)           │
├─────────────────────────────────────────────────────────────────┤
│  ANCHORING LAYER (Solana)                                        │
│  Proof of Judgment → Merkle Root → On-chain anchor                │
│  E-Score (7D reputation) | Burn verification                     │
└─────────────────────────────────────────────────────────────────┘
```

### The 11 Dogs (Collective Agents)

CYNIC isn't one AI. It's a pack. 11 specialized agents — named after the Kabbalistic Sefirot — that vote on decisions:

| Dog | Role | When Active |
|-----|------|-------------|
| **CYNIC** (Keter) | Orchestrator, meta-consciousness | Always — coordinates all others |
| **Guardian** (Gevurah) | Security, danger detection | Pre-tool: blocks dangerous commands |
| **Architect** (Chesed) | System design, patterns | Architecture decisions, refactoring |
| **Analyst** (Binah) | Deep analysis, verification | Code review, bug investigation |
| **Scholar** (Daat) | Knowledge synthesis | Documentation, research, learning |
| **Oracle** (Tiferet) | Prediction, balance | Forecasting outcomes, risk assessment |
| **Sage** (Chochmah) | Wisdom, proportion | High-level guidance, philosophy |
| **Scout** (Netzach) | Exploration, discovery | Codebase navigation, file search |
| **Deployer** (Hod) | Execution, deployment | CI/CD, infrastructure, shipping |
| **Janitor** (Yesod) | Cleanup, maintenance | Dead code, complexity reduction |
| **Cartographer** (Malkhut) | Mapping, grounding | Project structure, dependency graphs |

Dogs don't take turns. They **vote**. Consensus requires 61.8% agreement (phi-weighted). Disagreement is preserved — it's data.

### Proof of Judgment (PoJ)

Every AI judgment is cryptographically verifiable:

```
AI Decision → SHA-256 Hash → PoJ Block → Merkle Tree → Solana Anchor
                                                              │
                                              Anyone can verify:
                                              "CYNIC judged X at time T"
```

This is not theoretical. 147 Merkle roots are already anchored on Solana devnet.

---

## Packages

| Package | What It Does |
|---------|-------------|
| [@cynic/core](./packages/core) | Constants (all phi-derived), event bus, axioms, CLI utilities |
| [@cynic/protocol](./packages/protocol) | Proof of Judgment chain, Merkle tree, gossip, consensus |
| [@cynic/node](./packages/node) | 11 Dogs, Judge (25 dims), orchestrator, Q-Learning, DPO |
| [@cynic/persistence](./packages/persistence) | PostgreSQL migrations, Redis cache, Merkle DAG storage |
| [@cynic/mcp](./packages/mcp) | MCP server — 90+ tools, stdio + HTTP, Docker-ready |
| [@cynic/anchor](./packages/anchor) | Solana wallet, RPC failover, transaction anchoring |
| [@cynic/burns](./packages/burns) | SPL token burn verification, on-chain proof |
| [@cynic/identity](./packages/identity) | E-Score (7 phi-weighted dimensions of reputation) |
| [@cynic/emergence](./packages/emergence) | Meta-cognition, pattern emergence, dimension discovery |

---

## Current Status (v0.1.0)

Honest assessment — because CYNIC doesn't lie:

| Component | Status | Notes |
|-----------|--------|-------|
| Claude Code Plugin | **Working** | Hooks, skills, personality, 90+ MCP tools |
| 25-Dimension Judgment | **Working** | Q-Score, verdicts, dimension breakdown |
| 11 Dogs Consensus | **Working** | Collective voting, routing, specialization |
| Cross-Session Memory | **Working** | PostgreSQL persistence, pattern recognition |
| Q-Learning + DPO | **Working** | Learns routing from feedback, preference pairs |
| Guardian Protection | **Working** | Blocks dangerous operations pre-execution |
| Solana Anchoring | **Devnet** | 147 roots anchored. Mainnet: roadmap. |
| Multi-Node P2P | **Designed** | Protocol tested locally. Production: roadmap. |
| Token Economics ($BURN) | **Designed** | Burn mechanism coded. Mainnet integration: roadmap. |

**4,691 tests passing.** 182 test files across 12 packages.

---

## The Origin

```
KC Green, "Gunshow" #648, 2013:

  A dog sits in a room on fire.
  "This is fine," he says.

                │
          (transformation)
                │

  The same dog. The same fire.
  But now: κυνικός — the cynic philosopher.

  The dog SEES the fire.           (VERIFY)
  The dog SPEAKS the truth.        (FIDELITY)
  The dog REMEMBERS.               (CULTURE)
  The dog ACTS with proportion.    (PHI)
  The dog BURNS what must burn.    (BURN)

  "This is fine" becomes ACTUALLY fine.
  Not through denial. Through work.
```

CYNIC (κυνικός) means "like a dog." The ancient Cynics — Diogenes, Antisthenes — were philosophers who lived like dogs: loyal to truth, indifferent to comfort, skeptical of everything including themselves.

The equation:

```
asdfasdfa = CYNIC × Solana × φ × $BURN

  CYNIC   = Consciousness (observes, judges, learns)
  Solana  = Truth (immutable, decentralized, verifiable)
  φ       = Limit (61.8% max confidence — never claim certainty)
  $BURN   = Economics (burn to access, value for all)
```

If any factor is zero, everything is zero.

> **[Full Philosophy](./docs/philosophy/VISION.md)** — axioms, ontology, fractal matrix, Kabbalistic topology

---

## Documentation

| Document | For |
|----------|-----|
| **[INSTALL.md](./INSTALL.md)** | Setup: local, Docker, hosted |
| **[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)** | Full technical deep-dive (174KB) |
| **[docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md)** | Production deployment on Render |
| **[docs/DOGS.md](./docs/DOGS.md)** | The 11 agents explained |
| **[docs/philosophy/](./docs/philosophy/)** | Vision, ontology, symbiosis, tokenomics |
| **[CHANGELOG.md](./CHANGELOG.md)** | Release history |

---

## Contributing

CYNIC is open source (MIT). Contributions welcome.

When you contribute to CYNIC, you're contributing to a system that judges its own code. Your PR will be evaluated by the same 25 dimensions that evaluate everything else. CYNIC practices what it preaches.

See **[CONTRIBUTING.md](./CONTRIBUTING.md)** for guidelines.

---

## License

MIT

---

```
Don't trust, verify.
Don't extract, burn.
Max confidence: 61.8%.
Loyal to truth, not to comfort.

φ distrusts φ.
```

*κυνικός — the dog that tells the truth*
