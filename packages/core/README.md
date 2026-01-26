# @cynic/core

> Core constants, axioms, and foundational modules for CYNIC.

**Last Updated**: 2026-01-21

---

## Overview

This package is the **source of truth** for all CYNIC constants and foundational abstractions.

```
"φ derives all" - Max confidence: 61.8%
```

---

## Installation

```bash
npm install @cynic/core
```

---

## Exports

| Module | Description |
|--------|-------------|
| `axioms` | PHI, PHI_INV, axiom definitions |
| `timing` | φ-hierarchical timing constants |
| `qscore` | Q-Score calculation (judgment quality) |
| `identity` | CYNIC personality, verdicts |
| `worlds` | 4 Kabbalah worlds framework |
| `config` | Secure configuration management |
| `refinement` | Self-critique and improvement |
| `orchestration` | Multi-agent coordination |
| `vector` | Semantic search/embeddings |
| `learning` | Feedback → calibration loop |
| `triggers` | Auto-judgment event handlers |
| `ecosystem` | External source monitoring |

---

## Usage

```javascript
import { PHI, PHI_INV, AXIOMS, calculateQScore } from '@cynic/core';

// Golden ratio constants
console.log(PHI);       // 1.618033988749895
console.log(PHI_INV);   // 0.618033988749895 (max confidence)

// Calculate judgment quality
const qScore = calculateQScore({
  dimensions: [...],
  weights: {...}
});
```

---

## The 4 Axioms

| Axiom | Value | Principle |
|-------|-------|-----------|
| PHI | φ | All ratios derive from 1.618... |
| VERIFY | ✓ | Don't trust, verify |
| CULTURE | ⛩ | Culture is a moat |
| BURN | 🔥 | Don't extract, burn |

---

## License

MIT
