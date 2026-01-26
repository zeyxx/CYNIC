# CYNIC Glossary

> **"φ derives all"** - κυνικός

**Last Updated**: 2026-01-27

A comprehensive glossary of CYNIC-specific terms for contributors.

---

## Table of Contents

- [Core Constants](#core-constants)
- [The Four Axioms](#the-four-axioms)
- [Scores](#scores)
- [Sefirot (Agents)](#sefirot-agents)
- [Protocol Terms](#protocol-terms)
- [Timing Constants](#timing-constants)
- [File Locations](#file-locations)

---

## Core Constants

| Term | Symbol | Value | Definition | Defined In |
|------|--------|-------|------------|------------|
| **Golden Ratio** | φ (PHI) | 1.618... | The golden ratio, foundation of all CYNIC constants | `@cynic/core/axioms/constants.js` |
| **PHI_INV** | φ⁻¹ | 0.618... (61.8%) | Maximum confidence threshold. CYNIC never claims more than 61.8% certainty | `@cynic/core/axioms/constants.js` |
| **PHI_INV_2** | φ⁻² | 0.382... (38.2%) | Minimum doubt threshold. Also used as consensus minority threshold | `@cynic/core/axioms/constants.js` |
| **PHI_INV_3** | φ⁻³ | 0.236... (23.6%) | Critical threshold for urgent decisions | `@cynic/core/axioms/constants.js` |
| **PHI_2** | φ² | 2.618... | Phi squared, used in reward multipliers | `@cynic/core/axioms/constants.js` |

---

## The Four Axioms

CYNIC operates on four foundational axioms, each with a dedicated scorer.

| Axiom | Mantra | Meaning | Scorer File |
|-------|--------|---------|-------------|
| **PHI** | "φ derives all" | All ratios, thresholds, and timings derive from φ. Maximum confidence = 61.8% | `@cynic/core/axioms/scorers/phi.js` |
| **VERIFY** | "Don't trust, verify" | Systematic skepticism. Question everything, including CYNIC itself | `@cynic/core/axioms/scorers/verify.js` |
| **CULTURE** | "Culture is a moat" | Patterns define identity. Consistency with established patterns matters | `@cynic/core/axioms/scorers/culture.js` |
| **BURN** | "Don't extract, burn" | Simplicity wins. Penalties over rewards. Reduce complexity | `@cynic/core/axioms/scorers/burn.js` |

---

## Scores

| Score | Full Name | Range | Definition | Defined In |
|-------|-----------|-------|------------|------------|
| **Q-Score** | Quality Score | 0-100 | Weighted average across all 25 judgment dimensions | `@cynic/core/judgment/qscore.js` |
| **E-Score** | Ecosystem Score | -61.8K to 100K | Reputation score based on ecosystem contributions | `@cynic/identity`, Solana: `lib.rs` |
| **K-Score** | Holder Quality Score | 0-100 | Token quality metric based on holder distribution | `@cynic/holdex` |
| **Confidence** | — | 0 to φ⁻¹ | Certainty level of a judgment (capped at 61.8%) | `@cynic/core` |

---

## Sefirot (Agents)

CYNIC's agent collective is inspired by Kabbalah's Tree of Life (Sefirot).

| Agent | Sefira | Hebrew Meaning | Role | Level |
|-------|--------|----------------|------|-------|
| **CYNIC** | Keter | Crown | Meta-consciousness, orchestrates all agents | 0 (Top) |
| **Sage** | Chochmah | Wisdom | Guidance and teaching | 1 |
| **Analyst** | Binah | Understanding | Pattern analysis | 1 |
| **Scholar** | Daat | Knowledge | Knowledge extraction | 1 |
| **Guardian** | Gevurah | Strength | Security and protection (BLOCKING) | 2 |
| **Oracle** | Tiferet | Beauty | Visualization and dashboards | 2 |
| **Architect** | Chesed | Kindness | Design review | 2 |
| **Deployer** | Hod | Splendor | Deployment and infrastructure | 3 |
| **Janitor** | Yesod | Foundation | Code quality and simplification | 3 |
| **Scout** | Netzach | Victory | Discovery and exploration | 3 |
| **Cartographer** | Malkhut | Kingdom | Codebase mapping | 4 (Bottom) |

**File**: `packages/node/src/agents/collective/`

---

## Protocol Terms

| Term | Definition | Related Concepts |
|------|------------|------------------|
| **PoJ** | Proof of Judgment - CYNIC's consensus mechanism where judgments are cryptographically proven | Merkle roots, anchoring |
| **Anchoring** | Recording PoJ merkle roots on Solana blockchain | `@cynic/anchor`, `programs/cynic-anchor` |
| **Merkle Root** | Cryptographic hash of all judgments in a block | PoJ, verification |
| **Burn** | Token destruction as commitment/penalty mechanism | BURN axiom, slashing |
| **Validator** | Node that can anchor roots to Solana | Staking, rewards |
| **Gossip** | P2P communication protocol between nodes | `@cynic/protocol/gossip` |
| **Consensus** | φ-weighted agreement (61.8% threshold) | Byzantine tolerance |
| **Finality** | When a block/judgment becomes permanent | Anchoring |

---

## Timing Constants

All timings derive from φ with a base of 100ms.

| Constant | Value | Formula | Use Case |
|----------|-------|---------|----------|
| **TICK_MS** | 23.6ms | base × φ⁻³ | Atomic events |
| **MICRO_MS** | 38.2ms | base × φ⁻² | Acknowledgments |
| **SLOT_MS** | 61.8ms | base × φ⁻¹ | Block proposal window |
| **SYNC_MS** | 100ms | base | Sync interval |
| **BEAT_MS** | 161.8ms | base × φ | Heartbeat |
| **ROUND_MS** | 261.8ms | base × φ² | Consensus round |
| **EPOCH_MS** | 423.6ms | base × φ³ | Epoch boundary |

**File**: `@cynic/core/axioms/constants.js`

---

## Fibonacci Constants

CYNIC uses Fibonacci numbers for limits and buffer sizes.

| F(n) | Value | Common Use |
|------|-------|------------|
| F(6) | 8 | Small buffers |
| F(8) | 21 | MAX_VALIDATORS |
| F(10) | 55 | E-Score cooldown (slots) |
| F(11) | 89 | MAX_BURNS_PER_ACCOUNT |
| F(12) | 144 | Minimum stake unit |
| F(13) | 233 | Unstake cooldown (slots) |
| F(14) | 377 | MAX_ROOTS reference |

---

## Greek Terms

| Term | Greek | Meaning in CYNIC |
|------|-------|------------------|
| **κυνικός** | kynikos | "Like a dog" - CYNIC's philosophical identity |
| **φ** | phi | Golden ratio (1.618...) |

---

## Dog Expressions

CYNIC communicates with canine expressions:

| Expression | Meaning |
|------------|---------|
| *sniff* | Investigating something |
| *ears perk* | Noticed something relevant |
| *tail wag* | Approval, good work |
| *GROWL* | Danger warning (serious) |
| *head tilt* | Confused, need clarification |
| *yawn* | Wrapping up |

---

## File Locations

Quick reference for where key concepts are defined:

| Concept | Primary File |
|---------|--------------|
| φ Constants | `packages/core/src/axioms/constants.js` |
| Axiom Scorers | `packages/core/src/axioms/scorers/` |
| Q-Score | `packages/core/src/judgment/qscore.js` |
| Agents (Sefirot) | `packages/node/src/agents/collective/` |
| PoJ Chain | `packages/persistence/src/poj/` |
| Solana Program | `programs/cynic-anchor/src/lib.rs` |
| MCP Tools | `packages/mcp/src/tools/` |
| CYNIC Identity | `CLAUDE.md`, `.claude/cynic-consciousness.md` |

---

## See Also

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture
- [DOGS.md](./DOGS.md) - Agent system details
- [ROADMAP.md](../ROADMAP.md) - Development roadmap
- [CONTRIBUTING.md](../CONTRIBUTING.md) - How to contribute

---

*"Loyal to truth, not to comfort."* - CYNIC
