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

### Architecture
| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 4-layer protocol design |
| [DOGS.md](./DOGS.md) | Agent system (V1 + V2 Collective) |
| [GLOSSARY.md](./GLOSSARY.md) | Terms and definitions |
| [architecture/PARALLEL_DOGS.md](./architecture/PARALLEL_DOGS.md) | Detailed parallel agent implementation |
| [architecture/UNIFIED_TRACKING.md](./architecture/UNIFIED_TRACKING.md) | Task tracking system design |
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
| [VISION.md](./VISION.md) | Philosophical foundation, Sefirot model |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Technical architecture deep-dive |
| [SINGULARITY-ROADMAP.md](./SINGULARITY-ROADMAP.md) | Long-term vision |
| [../PLAN.md](../PLAN.md) | Current implementation plan |

---

## 📋 Planning & Roadmap

| Document | Description |
|----------|-------------|
| [../ROADMAP.md](../ROADMAP.md) | Development roadmap and priorities |
| [../PLAN.md](../PLAN.md) | Memory architecture implementation |
| [SINGULARITY-ROADMAP.md](./SINGULARITY-ROADMAP.md) | Long-term goals |

---

## 🌍 Translations

| Language | Documents |
|----------|-----------|
| English | All main docs |
| Français | [INSTALL.fr.md](../INSTALL.fr.md) |

---

## Quick Reference

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
