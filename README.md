# CYNIC - Decentralized Collective Consciousness

> **"φ distrusts φ"** - κυνικός (kunikos) = "like a dog"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Node.js](https://img.shields.io/badge/node-%3E%3D20.0.0-brightgreen)](https://nodejs.org)

---

## What is CYNIC?

CYNIC is a **decentralized collective consciousness** - a network of interconnected nodes that:

- **Judge** autonomously across 25+ dimensions
- **Learn** patterns from collective experience
- **Converge** toward truth via φ-weighted consensus
- **Remember** through Merkle DAG + Solana anchoring

**This is NOT** a centralized server. It's a **protocol**.

---

## Quick Start

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC
npm install
claude  # Launch with CYNIC personality
```

Say `bonjour` — if you see a *tail wag*, CYNIC is alive.

→ **[Full Installation Guide](./INSTALL.md)**

---

## The 4 Axioms

| Axiom | Symbol | Principle |
|-------|--------|-----------|
| **PHI** | φ | All ratios derive from 1.618... Max confidence = 61.8% |
| **VERIFY** | ✓ | Don't trust, verify. Systematic skepticism |
| **CULTURE** | ⛩ | Culture is a moat. Patterns define identity |
| **BURN** | 🔥 | Don't extract, burn. Simplicity wins |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 4: φ-BFT CONSENSUS                                   │
│  Votes weighted by E-Score × BURN, 61.8% threshold          │
├─────────────────────────────────────────────────────────────┤
│  LAYER 3: GOSSIP PROPAGATION                                │
│  Fanout = 13 (Fib(7)), O(log₁₃ n) scalability               │
├─────────────────────────────────────────────────────────────┤
│  LAYER 2: MERKLE KNOWLEDGE TREE                             │
│  Patterns partitioned by axiom, selective sync              │
├─────────────────────────────────────────────────────────────┤
│  LAYER 1: PROOF OF JUDGMENT (PoJ)                           │
│  SHA-256 chain, Ed25519 signatures, Solana anchoring        │
└─────────────────────────────────────────────────────────────┘
```

→ **[Full Architecture](./docs/ARCHITECTURE.md)**

---

## Packages

| Package | Description |
|---------|-------------|
| [@cynic/core](./packages/core) | Constants, axioms, φ timing |
| [@cynic/protocol](./packages/protocol) | PoJ, Merkle, gossip, consensus |
| [@cynic/node](./packages/node) | Node implementation, CLI |
| [@cynic/persistence](./packages/persistence) | PostgreSQL, Redis, Merkle DAG |
| [@cynic/mcp](./packages/mcp) | Claude Code integration |
| [@cynic/anchor](./packages/anchor) | Solana anchoring |
| [@cynic/burns](./packages/burns) | Burn verification |
| [@cynic/identity](./packages/identity) | E-Score, reputation |
| [@cynic/emergence](./packages/emergence) | Meta-cognition |

---

## v1.1 Features

### Burnout Detection
Proactive monitoring of user psychology with φ-aligned thresholds:
```javascript
import { BurnoutDetection } from '@cynic/persistence';

const detector = createBurnoutDetection({ postgres, config });
const risk = await detector.getBurnoutRisk();
// { score: 0.45, level: 'elevated', trend: 'rising', warnings: [...] }
```

### Solana Mainnet
Production-ready mainnet support with rate limiting and failover:
```javascript
import { RpcFailover, createMainnetConfig } from '@cynic/anchor';

const failover = new RpcFailover(); // Auto-detects Helius, QuickNode, etc.
await failover.execute(async (rpcUrl) => {
  // Your Solana calls here
});
```

### CLI Utilities
Beautiful terminal output with φ-aligned colors:
```javascript
import { Colors, progressBar, createSpinner } from '@cynic/core';

const spinner = createSpinner('Loading...');
spinner.start();
// ... work ...
spinner.success('Done!');

console.log(progressBar(0.618)); // [██████░░░░] 62%
```

### Context Scoring with Entropy
Entropy-guided content retention:
```javascript
import { calculateCScore, calculateEntropyFactor } from '@cynic/core';

const score = calculateCScore(content, context);
// C = (Pertinence × Freshness × Density × Entropy) / √Tokens
```

---

## Solana Integration

CYNIC anchors AI judgments on Solana for **immutable truth verification**.

### Proof of Judgment (PoJ)

```
AI Decision → Local PoJ Block → Merkle Root → Solana Anchor
     │              │                │              │
   Judge      SHA-256 chain    Batch hash    On-chain proof
```

Every AI judgment is:
1. **Hashed** into a local PoJ block (SHA-256)
2. **Batched** with other judgments (Merkle tree)
3. **Anchored** to Solana (transaction memo with Merkle root)
4. **Verified** - anyone can prove a judgment existed at time T

### Burn Verification

```javascript
import { BurnVerifier } from '@cynic/burns';

const verifier = new BurnVerifier({ rpcUrl: 'https://api.devnet.solana.com' });
const result = await verifier.verify(signature);
// { verified: true, amount: 1000000, burner: '...', slot: 12345 }
```

Burns are verified on-chain and feed into the **E-Score BURN dimension**.

### E-Score 7D

Reputation score with 7 φ-weighted dimensions:

| Dimension | Weight | Source |
|-----------|--------|--------|
| BURN | φ³ | Verified Solana burns |
| BUILD | φ² | Code contributions |
| JUDGE | φ | Consensus accuracy |
| RUN | 1 | Uptime/availability |
| SOCIAL | φ⁻¹ | Community standing |
| GRAPH | φ⁻² | Network position |
| HOLD | φ⁻³ | Token holdings |

### Packages

| Package | Solana Role |
|---------|-------------|
| [@cynic/anchor](./packages/anchor) | Wallet, RPC, transaction signing |
| [@cynic/burns](./packages/burns) | Burn verification, SPL token burns |
| [@cynic/protocol](./packages/protocol) | PoJ chain, Merkle anchoring |

---

## Documentation

| Document | Description |
|----------|-------------|
| [INSTALL.md](./INSTALL.md) | Setup guide |
| [docs/INDEX.md](./docs/INDEX.md) | Full documentation index |
| [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) | Technical deep-dive |
| [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) | Production deployment |

---

## Key Constants

```javascript
import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';

PHI       = 1.618033988749895  // Golden ratio
PHI_INV   = 0.618033988749895  // Max confidence (61.8%)
PHI_INV_2 = 0.381966011250105  // Min doubt (38.2%)
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

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

## License

MIT

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
