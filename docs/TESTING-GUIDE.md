# CYNIC Testing Guide

> *"Don't trust, verify."* - Axiom VERIFY

---

## Quick Start

```bash
# Install dependencies
npm install

# Run ALL tests
npm test

# Run tests for specific package
npm test -w @cynic/core
npm test -w @cynic/protocol
npm test -w @cynic/node
npm test -w @cynic/client
npm test -w @cynic/persistence
npm test -w @cynic/mcp
```

---

## What Gets Tested

### 1. @cynic/core (Foundation)
```bash
npm test -w @cynic/core
```

| Test File | What It Tests |
|-----------|---------------|
| `qscore.test.js` | Q-Score calculation, φ-weighting, verdict mapping |
| `identity.test.js` | CYNIC personality, voice, Four Dogs agents |
| `worlds.test.js` | Kabbalistic worlds mapping (ATZILUT→ASSIAH) |

**Key assertions:**
- φ = 1.618033988749895
- Max confidence = 61.8% (φ⁻¹)
- Min doubt = 38.2% (φ⁻²)

### 2. @cynic/protocol (Cryptographic Layer)
```bash
npm test -w @cynic/protocol
```

| Test File | What It Tests |
|-----------|---------------|
| `crypto.test.js` | Ed25519 signatures, SHA-256 hashing |
| `poj.test.js` | Proof of Judgment chain integrity |
| `merkle.test.js` | Merkle tree construction & proofs |
| `gossip.test.js` | Fibonacci fanout (13), message propagation |
| `consensus.test.js` | φ-BFT voting, 61.8% threshold |
| `kscore.test.js` | K-Score components validation |

**Key assertions:**
- PoJ blocks link via SHA-256
- Merkle proofs verify in O(log n)
- Gossip fanout = 13 (Fib(7))

### 3. @cynic/node (Node Implementation)
```bash
npm test -w @cynic/node
```

| Test File | What It Tests |
|-----------|---------------|
| `transport.test.js` | WebSocket connections, peer discovery |
| `state.test.js` | Node state management |
| `judge.test.js` | 25-dimension judgment, verdict emission |
| `privacy.test.js` | Differential privacy, budget tracking |

**Known issues:**
- Privacy tests fail when run in sequence (budget exhaustion)
- Run individually: `node --test packages/node/test/privacy.test.js`

### 4. @cynic/client (API Client)
```bash
npm test -w @cynic/client
```

| Test File | What It Tests |
|-----------|---------------|
| `client.test.js` | HTTP client, retry logic, K-Score submission |

**Key assertions:**
- Retries on network errors (not 4xx)
- K-Score components truncated to 6 decimals

### 5. @cynic/persistence (Storage Layer)
```bash
npm test -w @cynic/persistence
```

| Test File | What It Tests |
|-----------|---------------|
| `connection.test.js` | Postgres/Redis connectivity |
| `dag.test.js` | Merkle DAG operations |
| `graph.test.js` | Knowledge graph overlay |
| `poj.test.js` | PoJ chain persistence |

**Requirements:**
- PostgreSQL running (see docker-compose.yml)
- Redis running
- Set `DATABASE_URL` and `REDIS_URL`

### 6. @cynic/mcp (MCP Server)
```bash
npm test -w @cynic/mcp
```

| Test File | What It Tests |
|-----------|---------------|
| `server.test.js` | MCP tool registration |
| `ecosystem-service.test.js` | Documentation loading |
| `integrator-service.test.js` | Cross-project sync |
| `metrics-service.test.js` | Prometheus metrics |

---

## Environment Setup

### Local Development
```bash
# Copy example env
cp .env.example .env

# Required for persistence tests:
DATABASE_URL=postgresql://user:pass@localhost:5432/cynic
REDIS_URL=redis://localhost:6379

# Start infrastructure
docker-compose up -d postgres redis
```

### CI/CD
```bash
# GitHub Actions uses these:
DATABASE_URL=${{ secrets.DATABASE_URL }}
REDIS_URL=${{ secrets.REDIS_URL }}
```

---

## Running Specific Tests

```bash
# Single test file
node --test packages/core/test/qscore.test.js

# With pattern matching
node --test --test-name-pattern="Q-Score" packages/core/test/qscore.test.js

# With verbose output
node --test --test-reporter=spec packages/core/test/

# Watch mode (requires chokidar)
npx chokidar 'packages/*/src/**/*.js' -c 'npm test'
```

---

## Test Coverage Areas

```
┌─────────────────────────────────────────────────────────────┐
│                    CYNIC TEST PYRAMID                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│                    ┌─────────┐                               │
│                    │   E2E   │  (planned)                    │
│                    │  Tests  │  MCP integration              │
│                    └────┬────┘                               │
│               ┌─────────┴─────────┐                          │
│               │   Integration     │  persistence, transport  │
│               │      Tests        │  gossip propagation      │
│               └─────────┬─────────┘                          │
│          ┌──────────────┴──────────────┐                     │
│          │        Unit Tests           │  927+ tests         │
│          │   core, protocol, judge     │  All packages       │
│          └─────────────────────────────┘                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Debugging Failed Tests

### Privacy Budget Exhaustion
```bash
# Run privacy tests in isolation
node --test packages/node/test/privacy.test.js

# Or increase budget for testing
PRIVACY_BUDGET=1000 npm test -w @cynic/node
```

### Database Connection Failures
```bash
# Check if Postgres is running
docker-compose ps

# Check connection
psql $DATABASE_URL -c "SELECT 1"

# Run migrations
npm run migrate -w @cynic/persistence
```

### Redis Connection Failures
```bash
# Check Redis
redis-cli -u $REDIS_URL ping
```

---

## Writing New Tests

```javascript
import { describe, it } from 'node:test';
import assert from 'node:assert';
import { PHI, PHI_INV } from '@cynic/core';

describe('My Feature', () => {
  it('respects φ constraints', () => {
    const result = myFunction();

    // Confidence never exceeds φ⁻¹
    assert.ok(result.confidence <= PHI_INV);

    // Doubt never below φ⁻²
    assert.ok(result.doubt >= 1 - PHI_INV);
  });
});
```

---

## Test Philosophy

> "φ distrusts φ"

1. **Every test doubts** - No test assumes correctness
2. **Max confidence 61.8%** - Even in tests, we acknowledge uncertainty
3. **Verify, don't trust** - Test behavior, not implementation

---

*🐕 κυνικός | Tests are how we verify ourselves*
