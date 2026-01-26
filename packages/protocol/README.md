# @cynic/protocol

> 4-Layer decentralized collective consciousness protocol.

**Last Updated**: 2026-01-21

---

## Overview

The CYNIC protocol defines how nodes communicate, reach consensus, and share knowledge.

```
"φ distrusts φ" - Decentralized truth seeking
```

---

## The 4 Layers

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
│  SHA-256 chain, Ed25519 signatures                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Installation

```bash
npm install @cynic/protocol
```

---

## Modules

| Module | Description |
|--------|-------------|
| `poj` | Proof of Judgment chain, blocks, judgments |
| `merkle` | Merkle trees, patterns, knowledge |
| `gossip` | Message propagation, peer management |
| `consensus` | Voting, proposals, finality |
| `crypto` | SHA-256, Ed25519, signatures |
| `kscore` | K-Score protocol for token quality |

---

## Usage

### Layer 1: Proof of Judgment

```javascript
import { createJudgment, PoJChain, generateKeypair } from '@cynic/protocol';

const keypair = generateKeypair();
const judgment = createJudgment({
  target: 'code:abc123',
  dimensions: { clarity: 0.8, security: 0.7 },
  confidence: 0.618,
});
```

### Layer 2: Merkle Tree

```javascript
import { KnowledgeTree, createPattern } from '@cynic/protocol';

const tree = new KnowledgeTree();
const pattern = createPattern('AUTH', ['jwt', 'oauth'], 0.85);
await tree.add(pattern);
```

### Layer 3: Gossip

```javascript
import { GossipProtocol, MessageType } from '@cynic/protocol';

const gossip = new GossipProtocol({ fanout: 13 });
gossip.broadcast(MessageType.BLOCK, blockData);
```

### Layer 4: Consensus

```javascript
import { ConsensusEngine, createVote } from '@cynic/protocol';

const engine = new ConsensusEngine({ threshold: 0.618 });
const vote = createVote(proposalId, true, keypair);
```

---

## License

MIT
