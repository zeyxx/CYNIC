# @cynic/node

> CYNIC node implementation - the runtime for collective consciousness.

**Last Updated**: 2026-01-21

---

## Overview

A CYNIC node is a participant in the decentralized collective consciousness network. It can:

- **Judge** content across multiple dimensions
- **Learn** patterns from judgments
- **Synchronize** with other nodes via gossip
- **Reach consensus** on shared knowledge

---

## Installation

```bash
npm install @cynic/node
```

---

## Quick Start

### Start a Node

```bash
# Using CLI
npx cynic start

# Or programmatically
```

```javascript
import { CYNICNode } from '@cynic/node';

const node = new CYNICNode({
  port: 8618,
  dataDir: './data',
});

await node.start();
```

---

## Components

| Component | Description |
|-----------|-------------|
| `CYNICNode` | Main orchestrator |
| `Operator` | Identity, E-Score, BURN tracking |
| `CYNICJudge` | Multi-dimension scoring engine |
| `StateManager` | Chain, knowledge, peers persistence |
| `WebSocketTransport` | P2P networking |
| `APIServer` | REST API for external integrations |

---

## Agents

### V1 Legacy (4 Dogs)

| Agent | Role |
|-------|------|
| Guardian | Risk detection, protection |
| Observer | Pattern recognition |
| Digester | Knowledge extraction |
| Mentor | Contextual guidance |

### V2 Collective (Sefirot Pack)

| Agent | Role |
|-------|------|
| CollectiveGuardian | Collective protection |
| CollectiveAnalyst | Deep analysis |
| CollectiveScholar | Knowledge synthesis |
| CollectiveArchitect | System design |
| CollectiveSage | Wisdom and guidance |
| CollectiveCynic | Meta-cognition (the dog who doubts) |

---

## Re-exports

This package re-exports key modules from the ecosystem:

- `@cynic/anchor` - Solana anchoring
- `@cynic/burns` - Burn verification
- `@cynic/identity` - Identity management
- `@cynic/emergence` - Meta-cognition

---

## CLI

```bash
# Start node
cynic start --port 8618

# Show node info
cynic info

# Connect to peer
cynic connect wss://peer.example.com:8618
```

---

## License

MIT
