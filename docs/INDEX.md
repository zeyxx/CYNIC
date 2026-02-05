# CYNIC Documentation Index

> Navigate to the right documentation for your needs.

---

## 🚀 Start Here

| Document | For | Description |
|----------|-----|-------------|
| [**GETTING-STARTED.md**](../GETTING-STARTED.md) | Everyone | Quick overview, first steps |
| [**INSTALL.md**](../INSTALL.md) | Users | Setup guide for Claude Code + MCP |
| [**README.md**](../README.md) | Everyone | Project vision and architecture overview |

---

## 👤 For Users (Claude Code)

| Document | Description |
|----------|-------------|
| [INSTALL.md](../INSTALL.md) | Complete setup instructions |
| [CLAUDE.md](../CLAUDE.md) | Understanding CYNIC's personality |
| [.claude/README.md](../.claude/README.md) | Plugin structure and customization |

---

## 🛠️ For Developers

### Setup & Configuration
| Document | Description |
|----------|-------------|
| [CONFIGURATION.md](./CONFIGURATION.md) | Environment variables, config files |
| [DEPLOYMENT.md](./DEPLOYMENT.md) | Production deployment (Docker, Render) |
| [TESTING-GUIDE.md](./TESTING-GUIDE.md) | Running and writing tests |

### Tutorials & Guides
| Document | Description |
|----------|-------------|
| [TUTORIAL-BUILD-DOG.md](./TUTORIAL-BUILD-DOG.md) | Build your first custom Dog agent |
| [COOKBOOK.md](./COOKBOOK.md) | Common patterns and recipes |

### Architecture
| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 4-layer protocol design |
| [DIAGRAMS.md](./DIAGRAMS.md) | Mermaid architecture diagrams |
| [BOOT-SEQUENCE.md](./BOOT-SEQUENCE.md) | MCP server boot sequence and lifecycle |
| [DOGS.md](./DOGS.md) | Agent system (V1 + V2 Collective) |
| [GLOSSARY.md](./GLOSSARY.md) | Terms and definitions |
| [architecture/PARALLEL_DOGS.md](./architecture/PARALLEL_DOGS.md) | Detailed parallel agent implementation |
| [architecture/UNIFIED_TRACKING.md](./architecture/UNIFIED_TRACKING.md) | Task tracking system design |
| [architecture/learning-pipeline.md](./architecture/learning-pipeline.md) | Q-Learning, EWC++, φ-governed adaptation |
| [REVIEW-LOG.md](./REVIEW-LOG.md) | Quarterly documentation review log |

### Packages
| Package | README | Description |
|---------|--------|-------------|
| @cynic/core | [README](../packages/core/README.md) | Constants, axioms, φ timing |
| @cynic/protocol | [README](../packages/protocol/README.md) | PoJ, Merkle, gossip, consensus |
| @cynic/node | [README](../packages/node/README.md) | Node implementation, CLI |
| @cynic/persistence | [README](../packages/persistence/README.md) | PostgreSQL, Redis, DAG |
| @cynic/mcp | [README](../packages/mcp/README.md) | MCP server for Claude |
| @cynic/anchor | [README](../packages/anchor/README.md) | Solana anchoring |
| @cynic/burns | [README](../packages/burns/README.md) | Burn verification |
| @cynic/holdex | [README](../packages/holdex/README.md) | HolDex integration |
| @cynic/gasdf | [README](../packages/gasdf/README.md) | GASdf integration |
| @cynic/identity | [README](../packages/identity/README.md) | Identity management |
| @cynic/emergence | [README](../packages/emergence/README.md) | Emergence detection |
| @cynic/zk | [README](../packages/zk/README.md) | Zero-knowledge proofs |

---

## 📐 For Architects

| Document | Description |
|----------|-------------|
| [**philosophy/fractal-matrix.md**](./philosophy/fractal-matrix.md) | **7×7 Matrix - FOUNDATIONAL** |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Technical architecture deep-dive |
| [philosophy/cynic-ontology.md](./philosophy/cynic-ontology.md) | 4 Axiomes, 25 dimensions, fractales |
| [philosophy/human-llm-symbiosis.md](./philosophy/human-llm-symbiosis.md) | CYNIC comme membrane |
| [../PLAN.md](../PLAN.md) | Current implementation plan |

---

## 🔮 Vision (Aspirational)

> **Warning:** These documents describe what CYNIC ASPIRES to become, not what EXISTS.

| Document | Description |
|----------|-------------|
| [vision/README.md](./vision/README.md) | Index des documents vision |
| [vision/CORE-VISION.md](./vision/CORE-VISION.md) | Falsification Pipeline (non implémenté) |
| [vision/SINGULARITY-ROADMAP.md](./vision/SINGULARITY-ROADMAP.md) | Long-term vision |
| [vision/tokenomics.md](./vision/tokenomics.md) | $BURN economy (pas de token actuellement) |

---

## 📋 Planning & Roadmap

| Document | Description |
|----------|-------------|
| [../ROADMAP.md](../ROADMAP.md) | Development roadmap and priorities |
| [../PLAN.md](../PLAN.md) | Memory architecture implementation |
| [vision/SINGULARITY-ROADMAP.md](./vision/SINGULARITY-ROADMAP.md) | Long-term goals (vision) |

---

## 🌍 Translations

| Language | Documents |
|----------|-----------|
| English | All main docs |
| Français | [INSTALL.fr.md](../INSTALL.fr.md) |

---

## Quick Reference

### The 7×7 Fractal Matrix
```
7 Realities: CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS
7 Analyses:  PERCEIVE, JUDGE, DECIDE, ACT, LEARN, ACCOUNT, EMERGE
= 49 cells + THE_UNNAMEABLE (50th gate)
```

### The 4 Axioms
| Axiom | Principle |
|-------|-----------|
| PHI | φ derives all. Max confidence 61.8% |
| VERIFY | Don't trust, verify |
| CULTURE | Culture is a moat |
| BURN | Don't extract, burn |

### Key Constants
```javascript
PHI       = 1.618033988749895  // Golden ratio
PHI_INV   = 0.618033988749895  // Max confidence
PHI_INV_2 = 0.381966011250105  // Min doubt
```

### Timing (φ-hierarchical)
| Level | Time | Purpose |
|-------|------|---------|
| TICK | 23.6ms | Atomic |
| SLOT | 61.8ms | Block proposal |
| BLOCK | 100ms | Finalization |
| EPOCH | 161.8ms | Checkpoint |

---

*🐕 κυνικός | Loyal to truth, not to comfort*
