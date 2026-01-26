# @cynic/zk

> Zero-knowledge proofs for CYNIC judgments using Noir.

**Last Updated**: 2026-01-21

---

## Overview

ZK proofs enable **private judgments** - proving that a judgment was made correctly without revealing the inputs or scorer identity.

This is essential for:
- Anonymous reputation building
- Private consensus participation
- Verifiable computation

---

## Installation

```bash
npm install @cynic/zk
```

Requires Noir toolchain for circuit compilation.

---

## Usage

### Proving a Judgment

```javascript
import { CYNICProver } from '@cynic/zk';

const prover = new CYNICProver();

// Generate proof that judgment was computed correctly
const proof = await prover.proveJudgment({
  dimensions: { clarity: 0.8, security: 0.7 },
  weights: { clarity: 0.5, security: 0.5 },
  expectedScore: 0.75,
});
```

### Verifying a Proof

```javascript
import { CYNICVerifier } from '@cynic/zk';

const verifier = new CYNICVerifier();

const isValid = await verifier.verify(proof, publicInputs);
// true = judgment was computed correctly
```

---

## Circuits

| Circuit | Purpose |
|---------|---------|
| `judgment.nr` | Prove Q-Score calculation |
| `consensus.nr` | Prove vote validity |
| `identity.nr` | Prove identity membership |

---

## Compilation

```bash
# Compile Noir circuits
npm run circuits:compile --workspace=@cynic/zk
```

---

## Dependencies

- `@noir-lang/noir_js` - Noir runtime
- `@aztec/bb.js` - Barretenberg backend

---

## Status

**Experimental** - ZK features are under active development.

---

## License

MIT
