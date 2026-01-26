# @cynic/gasdf

> GASdf integration - Gasless transactions on Solana.

**Last Updated**: 2026-01-21

---

## Overview

GASdf enables **gasless transactions** on Solana by sponsoring transaction fees. This is part of the $asdfasdfa ecosystem.

CYNIC uses GASdf to lower the barrier for users to burn tokens and participate in the collective.

---

## Installation

```bash
npm install @cynic/gasdf
```

---

## Usage

### Basic Client

```javascript
import { GASdfClient } from '@cynic/gasdf';

const client = new GASdfClient({
  apiUrl: process.env.GASDF_API_URL,
});

// Request gasless burn
const result = await client.requestGaslessBurn({
  wallet: 'So11...',
  amount: 1000,
  token: 'ASDF...',
});

if (result.sponsored) {
  console.log('Transaction:', result.signature);
}
```

---

## How It Works

1. User signs a burn transaction without SOL for fees
2. GASdf relayer submits and pays for the transaction
3. CYNIC verifies the burn via `@cynic/burns`
4. User's E-Score is updated

---

## Integration with CYNIC

```javascript
import { GASdfClient } from '@cynic/gasdf';
import { createBurnVerifier } from '@cynic/burns';

const gasdf = new GASdfClient();
const verifier = createBurnVerifier();

// Gasless burn → verify → update E-Score
const tx = await gasdf.requestGaslessBurn({ wallet, amount, token });
const verified = await verifier.verify({ signature: tx.signature });
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GASDF_API_URL` | GASdf relayer endpoint |
| `GASDF_API_KEY` | API key (if required) |

---

## License

MIT
