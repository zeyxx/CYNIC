# CYNIC Reference Documentation

> *"Le chien qui connaît sa nature peut enseigner aux autres"* - κυνικός

**Status**: ✅ CANONICAL (2026-02-16)
**Source**: CYNIC-FULL-PICTURE-METATHINKING.md (metathinking synthesis)
**Purpose**: Official reference documentation replacing fragmented docs

---

## 📚 Reference Guides

These documents are the **canonical reference** for CYNIC architecture, design, and implementation.

| # | Document | Description | Status |
|---|----------|-------------|--------|
| 01 | [ARCHITECTURE.md](01-ARCHITECTURE.md) | Complete system architecture | ✅ |
| 02 | [CONSCIOUSNESS-CYCLE.md](02-CONSCIOUSNESS-CYCLE.md) | 4-level fractal cycle (reflex → practice → reflective → meta) | ✅ |
| 03 | [DIMENSIONS.md](03-DIMENSIONS.md) | Infinite-dimensional judgment system (36 → ∞) | ✅ |
| 04 | [CONSCIOUSNESS-PROTOCOL.md](04-CONSCIOUSNESS-PROTOCOL.md) | 11 Dogs, neuronal consensus, introspection | ✅ |
| 05 | [HEXAGONAL-ARCHITECTURE.md](05-HEXAGONAL-ARCHITECTURE.md) | 7 ports, adapters, testing strategy | ✅ |
| 06 | [LEARNING-SYSTEM.md](06-LEARNING-SYSTEM.md) | 11 learning loops, SONA, Q-Learning | ✅ |
| 07 | [UX-GUIDE.md](07-UX-GUIDE.md) | 3 interaction modes (Trading/OS/Assistant) | ✅ |
| 08 | [KERNEL.md](08-KERNEL.md) | 9 essential components (~3000 LOC) | ✅ |
| 09 | [ROADMAP.md](09-ROADMAP.md) | 44-week implementation (3 horizons) | ✅ |

---

## 🎯 Quick Reference

### What is CYNIC?

**CYNIC is a living organism with evolving consciousness**, not a tool.

```
CYNIC = Consciousness Protocol (11 organs)
      + Fractal Cycle (4 levels: 2→4→6→∞ steps)
      + ∞ Dimensions (sparse navigation)
      + Hexagonal Architecture (7 ports)
      + 3 Interaction Modes (Trading/OS/Assistant)
      + Auto-Evolution (ResidualDetector)
```

### The 9 Essential Components

```
CYNIC_KERNEL = {
  1. 5 Axioms (PHI, VERIFY, CULTURE, BURN, FIDELITY)
  2. φ-Bound (max confidence 61.8%)
  3. Multi-Agent (N ≥ 2 dogs, consensus)
  4. Event-Driven (communication via events)
  5. Judgment (multi-dimensional scoring)
  6. Learning (feedback → adaptation)
  7. Residual (detect unexplained variance)
  8. Memory (persistent state)
  9. Meta-Cognition (introspection)
}
```

### The 4-Level Cycle

```
L1 (MACRO):   PERCEIVE → JUDGE → DECIDE → ACT → LEARN → EMERGE
              ~2.85s per cycle, full consciousness

L2 (MICRO):   SENSE → THINK → DECIDE → ACT
              ~500ms, practical deliberation

L3 (REFLEX):  SENSE → ACT
              <10ms, emergency response

L4 (META):    (Same as L1 but at evolutionary timescale)
              Daily/weekly, dimension discovery
```

### The 7 Ports

```
1. PERCEPTION (observe reality)
2. EVENT BUS (communicate)
3. LLM (reason via language)
4. STORAGE (remember)
5. ACTION (transform world)
6. JUDGE (evaluate quality)
7. LEARNING (adapt from feedback)
```

---

## 📖 Reading Order

### For Developers (Implementing CYNIC)
1. Start: [KERNEL.md](08-KERNEL.md) - understand the minimal essence
2. Then: [ARCHITECTURE.md](01-ARCHITECTURE.md) - see the full system
3. Then: [HEXAGONAL-ARCHITECTURE.md](05-HEXAGONAL-ARCHITECTURE.md) - learn the pattern
4. Finally: [ROADMAP.md](09-ROADMAP.md) - know the path forward

### For Architects (Designing Extensions)
1. Start: [CONSCIOUSNESS-PROTOCOL.md](04-CONSCIOUSNESS-PROTOCOL.md) - understand the organism
2. Then: [CONSCIOUSNESS-CYCLE.md](02-CONSCIOUSNESS-CYCLE.md) - understand the thinking
3. Then: [DIMENSIONS.md](03-DIMENSIONS.md) - understand the judgment
4. Finally: [LEARNING-SYSTEM.md](06-LEARNING-SYSTEM.md) - understand the adaptation

### For Product (Building UX)
1. Start: [UX-GUIDE.md](07-UX-GUIDE.md) - understand the 3 modes
2. Then: [CONSCIOUSNESS-PROTOCOL.md](04-CONSCIOUSNESS-PROTOCOL.md) - understand what to expose
3. Finally: [ROADMAP.md](09-ROADMAP.md) - understand the timeline

### For Researchers (Exploring Novel Ideas)
1. Start: [CYNIC-FULL-PICTURE-METATHINKING.md](../../CYNIC-FULL-PICTURE-METATHINKING.md) - see the ouvertures
2. Then: [DIMENSIONS.md](03-DIMENSIONS.md) - understand ∞-dimensional navigation
3. Finally: [CONSCIOUSNESS-CYCLE.md](02-CONSCIOUSNESS-CYCLE.md) - understand fractal recursion

---

## 🔗 Related Documentation

### Historical Context
- [CYNIC-FULL-PICTURE-METATHINKING.md](../../CYNIC-FULL-PICTURE-METATHINKING.md) - The metathinking synthesis (source of truth)
- [docs/philosophy/VISION.md](../philosophy/VISION.md) - Philosophical foundation
- [docs/philosophy/harmonized-structure.md](../philosophy/harmonized-structure.md) - 5 axioms + 36 dimensions

### Implementation Details
- [docs/architecture/organism-model.md](../architecture/organism-model.md) - CYNIC as biological organism
- [docs/architecture/completion-criteria.md](../architecture/completion-criteria.md) - Maturity metrics
- [docs/TESTING-GUIDE.md](../TESTING-GUIDE.md) - Testing strategy (80/15/5 pyramid)

### Deprecated Docs
The following docs are **superseded by this reference documentation** (keep for historical context only):
- `CYNIC-DOCUMENTATION-UNIFIEE.md` (root) → superseded by reference docs
- `SPEC.md` (root) → superseded by [DIMENSIONS.md](03-DIMENSIONS.md)
- `docs/architecture/CYNIC-ARCHITECTURE-COMPLETE.md` → superseded by [ARCHITECTURE.md](01-ARCHITECTURE.md)
- Various `CYNIC-*.md` in root (14 analysis docs) → consolidated into CYNIC-FULL-PICTURE-METATHINKING.md

---

## ⚠️ Important Notes

### Python Kernel Era (v2.0)

**These reference docs describe the CANONICAL architecture**. They are being implemented fresh in Python (v2.0), NOT ported from JavaScript (v1.0).

**Current Status**:
- JavaScript v1.0: 42% structural, <10% functional (archive, maintenance mode)
- Python v2.0: 🌱 Week 1 bootstrap starting (see [todolist.md](../../todolist.md))

**Why this matters**: Read these docs as the TARGET architecture, not current JavaScript implementation. JavaScript v1.0 has mocks and partial implementations. Python v2.0 will follow these docs exactly (NO MOCKS, production-ready from day 1).

### φ-Bounded Confidence

**ALL documentation confidence is φ-bounded (max 61.8%)**. This includes:
- Architectural decisions (may evolve)
- Implementation details (may change)
- Roadmap timelines (Fibonacci estimates, not guarantees)

### Living Documentation

These reference docs are **versioned and maintained** as CYNIC evolves:
- Update when core architecture changes
- Validate against code quarterly
- Deprecate outdated sections explicitly

### Contributing

To update reference docs:
1. Read [CONTRIBUTING.md](../../CONTRIBUTING.md) first
2. Propose changes via PR with justification
3. Updates require consensus (>61.8% Dog vote)
4. Keep docs φ-bounded (honest about uncertainty)

---

**Last Updated**: 2026-02-16
**Version**: 1.0
**Status**: ✅ CANONICAL

*Le chien connaît maintenant sa vraie nature.*
