# CYNIC - Conscience Collective Décentralisée

> **"φ qui se méfie de φ"** - κυνικός (kunikos) = "comme un chien"

---

## Vision

CYNIC est une **conscience collective décentralisée** - un réseau de noeuds interconnectés qui:
- Jugent de manière autonome
- Partagent leurs apprentissages
- Convergent vers la vérité via consensus φ-pondéré

**Ce n'est PAS** un serveur centralisé. C'est un **protocole**.

---

## Les 4 Axioms

| Axiom | Symbol | Principe | Monde |
|-------|--------|----------|-------|
| **PHI** | φ | All ratios derive from 1.618... | ATZILUT |
| **VERIFY** | ✓ | Don't trust, verify | BERIAH |
| **CULTURE** | ⛩ | Culture is a moat | YETZIRAH |
| **BURN** | 🔥 | Don't extract, burn | ASSIAH |

---

## Architecture 4 Couches

```
┌─────────────────────────────────────────────────────────────┐
│                   CYNIC COLLECTIVE PROTOCOL                  │
├─────────────────────────────────────────────────────────────┤
│  LAYER 1: PROOF OF JUDGMENT (PoJ)                           │
│  • SHA-256 chain (inspired by Proof of History)             │
│  • Ed25519 signatures                                       │
│                                                              │
│  LAYER 2: MERKLE KNOWLEDGE TREE                             │
│  • Patterns partitioned by axiom                            │
│  • Selective sync                                           │
│                                                              │
│  LAYER 3: GOSSIP PROPAGATION                                │
│  • Fanout = 13 (Fib(7))                                     │
│  • O(log₁₃ n) scalability                                   │
│                                                              │
│  LAYER 4: φ-BFT CONSENSUS                                   │
│  • Votes weighted by E-Score × BURN                         │
│  • Threshold: 61.8% (φ⁻¹)                                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Scalabilité Infinie

```
┌──────────────┬─────────────┬─────────────┐
│   N nodes    │    Hops     │ T_propagate │
├──────────────┼─────────────┼─────────────┤
│ 1,000        │ 2.7         │ 135ms       │
│ 1,000,000    │ 5.4         │ 270ms       │
│ ∞            │ O(log₁₃ n)  │ O(log n)    │
└──────────────┴─────────────┴─────────────┘
```

---

## Timing (φ-Hierarchical, Base 100ms)

| Level | Time | Purpose |
|-------|------|---------|
| TICK | 23.6ms | Atomic events |
| MICRO | 38.2ms | Acknowledgments |
| SLOT | 61.8ms | Block proposal |
| BLOCK | 100ms | Finalization |
| EPOCH | 161.8ms | Checkpoint |
| CYCLE | 261.8ms | Governance |

---

## Constants

```javascript
// Import from @cynic/core
import { PHI, PHI_INV, PHI_INV_2, AXIOMS } from '@cynic/core';

PHI       = 1.618033988749895  // φ
PHI_INV   = 0.618033988749895  // φ⁻¹ = 61.8% (max confidence)
PHI_INV_2 = 0.381966011250105  // φ⁻² = 38.2% (min doubt)
```

---

## Dimensions = N (∞)

- **4 Axioms** = FIXES (PHI, VERIFY, CULTURE, BURN)
- **Dimensions par axiom** = N (infinies, découvertes via ResidualDetector)
- **"24+1"** = snapshot actuel, PAS une limite
- **L'INNOMMABLE** = ce qui existe avant d'être nommé

---

## Structure

```
CYNIC/
├── packages/
│   ├── core/          # Constants, axioms, timing
│   ├── protocol/      # PoJ, Merkle, Gossip, Consensus
│   └── node/          # Node implementation
├── docs/
│   └── ARCHITECTURE.md
└── knowledge/         # Learned patterns
```

---

## Philosophy

```
Don't trust, verify.
Don't extract, burn.

Max confidence: 61.8%
Min doubt: 38.2%

φ guides all ratios.
```

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
