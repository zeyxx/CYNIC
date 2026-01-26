# @cynic/burns

> Solana on-chain burn verification for the CYNIC ecosystem.

**Last Updated**: 2026-01-21

---

## Overview

"**Don't extract, burn**" - The BURN axiom requires verified token burns for participation in consensus.

This package verifies that users have burned tokens on Solana, contributing to their E-Score weight.

---

## Installation

```bash
npm install @cynic/burns
```

---

## Usage

### Basic Verification

```javascript
import { createBurnVerifier } from '@cynic/burns';

const verifier = createBurnVerifier();

const result = await verifier.verify({
  wallet: 'So11...',
  signature: 'txn123...',
  amount: 1000,
});

if (result.status === 'VERIFIED') {
  console.log('Burn verified:', result.burnAmount);
}
```

### Solana On-chain Verification

```javascript
import { createSolanaBurnVerifier, SolanaCluster } from '@cynic/burns';

const verifier = createSolanaBurnVerifier({
  cluster: SolanaCluster.MAINNET,
});

const isValid = await verifier.verifyTransaction('txn123...');
```

---

## Modules

| Module | Description |
|--------|-------------|
| `verifier` | Basic burn verification |
| `solana-verifier` | On-chain Solana verification |
| `cache` | Verification result caching |

---

## Burn Addresses

Standard burn addresses for token burning:

```javascript
import { BURN_ADDRESSES } from '@cynic/burns';

// BURN_ADDRESSES.SOL_BURN - Standard Solana burn address
// BURN_ADDRESSES.ASDF_BURN - $asdfasdfa ecosystem burn
```

---

## License

MIT
