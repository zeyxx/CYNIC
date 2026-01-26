# @cynic/persistence

> Hybrid persistence layer: PostgreSQL + Redis + Merkle DAG + PoJ Chain.

**Last Updated**: 2026-01-21

---

## Overview

CYNIC's memory architecture combines traditional databases with decentralized storage:

```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 4: PoJ Chain (Proof of Judgment)                     │
│  Immutable judgment chain with Solana anchoring             │
├─────────────────────────────────────────────────────────────┤
│  LAYER 3: Graph Overlay                                     │
│  Relationship graph between entities                        │
├─────────────────────────────────────────────────────────────┤
│  LAYER 2: Merkle DAG                                        │
│  Content-addressed storage with CIDs                        │
├─────────────────────────────────────────────────────────────┤
│  LAYER 1: PostgreSQL + Redis                                │
│  Hot cache + structured data                                │
└─────────────────────────────────────────────────────────────┘
```

---

## Installation

```bash
npm install @cynic/persistence
```

---

## Modules

| Module | Description |
|--------|-------------|
| `postgres` | PostgreSQL client, repositories |
| `redis` | Redis client, session store |
| `dag` | Merkle DAG, CIDs, HAMT index |
| `poj` | PoJ chain storage |
| `graph` | Relationship graph overlay |

---

## Usage

### PostgreSQL

```javascript
import { PostgresClient, JudgmentRepository } from '@cynic/persistence';

const client = new PostgresClient(process.env.CYNIC_DATABASE_URL);
const repo = new JudgmentRepository(client);

await repo.save(judgment);
```

### Merkle DAG

```javascript
import { MerkleDAG, createJudgmentNode } from '@cynic/persistence';

const dag = new MerkleDAG('./data/dag');
const node = createJudgmentNode(judgment);
const cid = await dag.put(node);
```

### Graph Overlay

```javascript
import { GraphOverlay, createTokenNode } from '@cynic/persistence';

const graph = new GraphOverlay(dag);
const token = createTokenNode('So11...', { name: 'SOL' });
await graph.addNode(token);
```

---

## Migration

```bash
# Run migrations
npm run migrate --workspace=@cynic/persistence
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CYNIC_DATABASE_URL` | PostgreSQL connection string |
| `CYNIC_REDIS_URL` | Redis connection string (optional) |

---

## License

MIT
