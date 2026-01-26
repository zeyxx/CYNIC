# @cynic/identity

> Node identity, E-Score, and reputation management.

**Last Updated**: 2026-01-21

---

## Overview

"**Know thyself, then verify**" - Identity in CYNIC is cryptographic (Ed25519) with reputation earned through burns and accurate judgments.

---

## Installation

```bash
npm install @cynic/identity
```

---

## Components

| Component | Description |
|-----------|-------------|
| `KeyManager` | Ed25519 keypair management |
| `EScoreCalculator` | E-Score computation |
| `NodeIdentity` | Full node identity |
| `ReputationGraph` | Trust relationships |

---

## Usage

### Key Management

```javascript
import { createKeyManager, generateKeypair } from '@cynic/identity';

const keyManager = createKeyManager();
const keypair = generateKeypair();
const nodeId = deriveNodeId(keypair.publicKey);
```

### E-Score

```javascript
import { calculateEScore, ESCORE_WEIGHTS } from '@cynic/identity';

const eScore = calculateEScore({
  burnAmount: 1000,
  accuracy: 0.85,
  uptime: 0.99,
  participation: 0.7,
});

// E-Score determines voting weight in consensus
```

### Reputation Graph

```javascript
import { createReputationGraph, TrustLevel } from '@cynic/identity';

const graph = createReputationGraph();
graph.addTrust(nodeA, nodeB, TrustLevel.HIGH);
```

---

## E-Score Dimensions

| Dimension | Weight | Description |
|-----------|--------|-------------|
| Burn | 38.2% | Total tokens burned |
| Accuracy | 23.6% | Judgment accuracy over time |
| Uptime | 14.6% | Node availability |
| Participation | 23.6% | Consensus participation |

---

## License

MIT
