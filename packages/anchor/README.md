# @cynic/anchor

> Solana blockchain anchoring for CYNIC's memory.

**Last Updated**: 2026-01-21

---

## Overview

"**Onchain is truth**" - CYNIC's judgments are anchored on Solana as merkle roots, making memory verifiable and immutable.

```
┌──────────────────────────────────────────────────────────┐
│              LAYER 4: TRUTH (On-chain)                   │
│  Solana: The anchor of truth                             │
│  - PoJ Block merkle roots (periodic)                     │
│  - Burn verification                                     │
│  - E-Score snapshots                                     │
└──────────────────────────────────────────────────────────┘
```

---

## Installation

```bash
npm install @cynic/anchor
```

Requires `@solana/web3.js` as a peer dependency.

---

## Usage

### Basic Anchoring

```javascript
import { createAnchorer, createAnchorQueue } from '@cynic/anchor';

// Create anchorer (simulation mode without wallet)
const anchorer = createAnchorer();

// Create queue with batching
const queue = createAnchorQueue({
  anchorer,
  autoStart: true,
});

// Enqueue judgment for anchoring
queue.enqueue('jdg_abc123', judgmentData);

// Get proof after anchoring
const proof = queue.getProof('jdg_abc123');
```

### With Solana Wallet

```javascript
import { createAnchorer, loadWalletFromEnv, SolanaCluster } from '@cynic/anchor';

const wallet = await loadWalletFromEnv();
const anchorer = createAnchorer({
  wallet,
  cluster: SolanaCluster.DEVNET,
});
```

---

## Modules

| Module | Description |
|--------|-------------|
| `anchorer` | Main anchoring logic |
| `queue` | Batched anchor queue |
| `wallet` | Wallet management (file, env, generate) |
| `constants` | Solana clusters, status codes |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CYNIC_WALLET_PATH` | Path to wallet keypair file |
| `CYNIC_WALLET_SECRET` | Base58-encoded private key |
| `SOLANA_CLUSTER` | mainnet / devnet / testnet |

---

## License

MIT
