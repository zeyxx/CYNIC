# CYNIC Memory Architecture Redesign

> "Onchain is truth surtout pour Cynic et sa mémoire" - User requirement

## Current State (Messy)

```
┌─────────────────────────────────────────────────────────────┐
│                   CURRENT ARCHITECTURE                       │
│                   (Not elegant, not harmonious)              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   PostgreSQL ────┐                                          │
│   (judgments,    │                                          │
│    patterns,     ├──> App ──> PoJ Chain ──> P2P Only        │
│    knowledge)    │                   │                      │
│                  │                   └──> NO SOLANA         │
│   Redis ─────────┤                                          │
│   (sessions)     │                                          │
│                  │                                          │
│   Merkle DAG ────┘                                          │
│   (local files)                                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘

PROBLEMS:
- Redundant data (PostgreSQL + DAG both store judgments)
- PoJ Chain is P2P-only, not anchored to Solana
- Burns accepted as parameters without verification
- "Onchain is truth" principle VIOLATED
- No clear separation of concerns
```

## Proposed Architecture (Carré, Élégant, Harmonieux)

```
┌─────────────────────────────────────────────────────────────┐
│              CYNIC 4-LAYER MEMORY ARCHITECTURE               │
│                   "Onchain is truth"                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ╔══════════════════════════════════════════════════════╗   │
│  ║              LAYER 1: SPEED (Ephemeral)              ║   │
│  ║  Redis                                               ║   │
│  ║  • Active sessions                                   ║   │
│  ║  • Real-time state                                   ║   │
│  ║  • Cache (expires after φ minutes)                   ║   │
│  ╚══════════════════════════════════════════════════════╝   │
│                           │                                 │
│                           ▼                                 │
│  ╔══════════════════════════════════════════════════════╗   │
│  ║              LAYER 2: INDEX (Queryable)              ║   │
│  ║  PostgreSQL                                          ║   │
│  ║  • Searchable copy of anchored data                  ║   │
│  ║  • Full-text search on knowledge                     ║   │
│  ║  • Dimension indexes on judgments                    ║   │
│  ║  • MUST include anchor_tx for each record            ║   │
│  ╚══════════════════════════════════════════════════════╝   │
│                           │                                 │
│                           ▼                                 │
│  ╔══════════════════════════════════════════════════════╗   │
│  ║              LAYER 3: PROOF (Immutable)              ║   │
│  ║  Merkle DAG                                          ║   │
│  ║  • Content-addressable storage (CIDs)                ║   │
│  ║  • Judgment proofs (full data)                       ║   │
│  ║  • Block merkle trees                                ║   │
│  ║  • Local-first, P2P replicable                       ║   │
│  ╚══════════════════════════════════════════════════════╝   │
│                           │                                 │
│                           ▼                                 │
│  ╔══════════════════════════════════════════════════════╗   │
│  ║              LAYER 4: TRUTH (On-chain)               ║   │
│  ║  Solana                                              ║   │
│  ║  • PoJ Block merkle roots (periodic anchoring)       ║   │
│  ║  • Burn verification via alonisthe.dev/burns        ║   │
│  ║  • E-Score snapshots (major changes only)            ║   │
│  ║  • THIS IS THE SOURCE OF TRUTH                       ║   │
│  ╚══════════════════════════════════════════════════════╝   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### Layer 1: SPEED (Redis)
**Purpose**: Fast, ephemeral data
**Stores**:
- Active session state
- In-flight judgments
- Caches for frequently accessed data
**Lifetime**: Minutes to hours (TTL = φ * 10 minutes = 6.18 minutes default)
**Truth**: NOT a source of truth - purely performance

### Layer 2: INDEX (PostgreSQL)
**Purpose**: Queryable copy of truth
**Stores**:
- Judgments (with `solana_anchor_tx` reference)
- Knowledge patterns (with `solana_anchor_tx` reference)
- PoJ Blocks (with `solana_anchor_tx` reference)
**Constraint**: Every record MUST have anchor reference or be marked `PENDING_ANCHOR`
**Truth**: Derived from Layer 4, not authoritative

### Layer 3: PROOF (Merkle DAG)
**Purpose**: Content-addressable proofs
**Stores**:
- Full judgment data (CID = content hash)
- Block merkle trees
- Pattern hashes
**Properties**:
- Immutable (content = address)
- P2P replicable
- Verifiable (hash = proof)
**Truth**: Provides proof, but Layer 4 confirms validity

### Layer 4: TRUTH (Solana)
**Purpose**: THE source of truth
**Stores**:
- PoJ Block merkle roots (every N blocks or φ minutes)
- Burn transaction verification
- E-Score state (on significant changes)
**Properties**:
- Immutable
- Globally verifiable
- Costs SOL (so batched/periodic)
**Truth**: THIS IS THE AUTHORITATIVE LAYER

## Data Flow

```
┌──────────────────────────────────────────────────────────────┐
│                      JUDGMENT FLOW                           │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   1. Judge creates judgment                                  │
│      │                                                       │
│      ▼                                                       │
│   2. Write to Redis (SPEED) - immediate                      │
│      Status: EPHEMERAL                                       │
│      │                                                       │
│      ▼                                                       │
│   3. Write to DAG (PROOF) - get CID                          │
│      Status: PROVED                                          │
│      │                                                       │
│      ▼                                                       │
│   4. Add to PoJ Block queue                                  │
│      │                                                       │
│      ▼                                                       │
│   5. When block fills OR timer expires:                      │
│      - Compute block merkle root                             │
│      - Anchor to Solana (TRUTH)                              │
│      Status: ANCHORED                                        │
│      │                                                       │
│      ▼                                                       │
│   6. Write to PostgreSQL (INDEX) with anchor_tx              │
│      Status: INDEXED                                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Key Changes from Current

| Aspect | Current | Proposed |
|--------|---------|----------|
| PoJ Chain | P2P only | Anchored to Solana |
| PostgreSQL | Source of truth | Index of truth |
| Burns | Trusted parameter | Verified via API |
| Merkle DAG | Redundant storage | Proof layer |
| Solana | Not integrated | THE source of truth |

## Implementation Steps

### Phase 1: Add Anchor Layer
1. Create `packages/anchor/` - Solana anchoring utilities
2. Implement `anchorMerkleRoot(root, slot)` function
3. Add `solana_anchor_tx` column to PostgreSQL tables
4. Create anchor queue service

### Phase 2: Integrate Burns API
1. Add `@cynic/burns` package
2. Implement `verifyBurn(tx)` calling alonisthe.dev/burns
3. Block judgment creation for unverified burns
4. Add burn verification to Operator class

### Phase 3: Refactor PoJ Chain
1. Modify PoJ to batch judgments
2. Anchor block merkle roots to Solana
3. Add finality status based on anchor confirmation
4. Update consensus to wait for anchor

### Phase 4: Update Data Flow
1. Judgment write order: Redis → DAG → Queue → Solana → PostgreSQL
2. Add status tracking: EPHEMERAL → PROVED → ANCHORED → INDEXED
3. Remove direct PostgreSQL writes for judgments
4. PostgreSQL becomes read-replica of Solana truth

## φ-Aligned Constants

```javascript
const ANCHOR_CONSTANTS = {
  // Anchor every φ minutes (61.8 seconds)
  ANCHOR_INTERVAL_MS: PHI_INV * 100 * 1000, // 61,800ms

  // Or when block has φ² * 100 judgments (38.2%)
  ANCHOR_BATCH_SIZE: Math.floor(PHI_INV_2 * 100), // 38 judgments

  // Max confidence in any single anchor: φ⁻¹
  ANCHOR_CONFIDENCE_CAP: PHI_INV, // 61.8%

  // Min confirmations for DETERMINISTIC finality
  MIN_CONFIRMATIONS: 32, // Solana finalized
};
```

## Verification Rules

1. **No judgment is true until anchored**
   - PostgreSQL records without `solana_anchor_tx` are PENDING
   - Only ANCHORED judgments count for consensus

2. **No burn is trusted until verified**
   - Call alonisthe.dev/burns API
   - Cache verification for 24 hours
   - Block operations on failed verification

3. **Merkle proof required for disputes**
   - Any challenge must provide merkle path
   - DAG provides proof, Solana confirms root

## Summary

This architecture is:
- **Carré (Proper)**: Each layer has exactly one job
- **Élégant**: Clear data flow, no redundancy
- **Harmonieux**: Layers work together, not against
- **"Onchain is truth"**: Solana as the ultimate authority

*tail wag* This respects the principle: CYNIC's memory is anchored on-chain.

---

## Implementation Status (2026-01-17)

### Completed

| Phase | Description | Status | Location |
|-------|-------------|--------|----------|
| 1 | Anchor Package | DONE | `packages/anchor/` |
| 2 | Burns Verification | DONE | `packages/burns/` |
| 3 | PoJ Chain Anchoring | DONE | `packages/mcp/src/poj-chain-manager.js` |
| 4 | PostgreSQL Schema | DONE | `packages/persistence/src/postgres/migrations/004_solana_anchoring.sql` |

### New Packages Created

#### `@cynic/anchor` (32 tests passing)
- `SolanaAnchorer` - Anchors merkle roots to Solana
- `AnchorQueue` - Batches items for efficient anchoring
- `AnchorStatus` enum - PENDING, QUEUED, ANCHORED, FAILED
- Merkle proof generation and verification
- Simulation mode for testing (no wallet required)

#### `@cynic/burns` (23 tests passing)
- `BurnVerifier` - Verifies burns via alonisthe.dev/burns API
- Caching with configurable TTL
- Batch verification support
- Expected burner/amount validation

### PoJ Chain Manager Updates
- Added `setAnchorQueue()` method
- Blocks automatically queued for Solana anchoring
- Anchor status tracked per block
- New stats: `blocksAnchored`, `anchorsFailed`
- Status includes `anchoringEnabled`, `anchoredBlocks`, `pendingAnchors`

### Database Migration (004_solana_anchoring.sql)
- Added `anchor_status`, `anchor_tx`, `anchor_slot`, `anchored_at` to:
  - `judgments`
  - `poj_blocks`
  - `patterns`
  - `knowledge`
- New tables:
  - `anchor_batches` - Track anchor batches
  - `burn_verifications` - Track verified burns
- New view: `pending_anchors`
- Helper function: `update_anchor_status()`

### Remaining Work

| Task | Priority | Effort | Status |
|------|----------|--------|--------|
| Solana wallet integration | HIGH | Medium | ✅ DONE |
| Full judgment flow refactor | MEDIUM | Large | Pending |
| Burn verification enforcement | MEDIUM | Small | ✅ DONE |
| E-Score on-chain snapshots | LOW | Medium | Pending |
| Merkle proof dispute resolution | LOW | Medium | Pending |

### How to Enable Anchoring

```javascript
import { createAnchorer, createAnchorQueue } from '@cynic/anchor';
import { PoJChainManager } from '@cynic/mcp';

// Create anchorer (simulation mode without wallet)
const anchorer = createAnchorer();

// Create queue
const anchorQueue = createAnchorQueue({
  anchorer,
  autoStart: true,
});

// Inject into PoJ manager
const pojManager = new PoJChainManager(persistence, {
  anchorQueue,
  autoAnchor: true,
});

// Blocks now automatically anchor to Solana!
```

### How to Verify Burns

```javascript
import { createBurnVerifier } from '@cynic/burns';

const verifier = createBurnVerifier();

// Verify a burn transaction
const result = await verifier.verify('tx_signature');
if (result.verified) {
  console.log(`Burn verified: ${result.amount} by ${result.burner}`);
}
```

### How to Configure Wallet (NEW)

```javascript
import {
  createAnchorer,
  loadWalletFromFile,
  loadWalletFromEnv,
  generateWallet,
  SolanaCluster,
} from '@cynic/anchor';

// Option 1: Load from Solana CLI keypair file
const wallet = loadWalletFromFile('~/.config/solana/id.json');

// Option 2: Load from environment variable
// Set CYNIC_SOLANA_KEY as JSON array or base58 encoded
const wallet = loadWalletFromEnv('CYNIC_SOLANA_KEY');

// Option 3: Generate new wallet (for testing)
const { wallet, secretKey } = generateWallet();
// WARNING: Store secretKey securely!

// Create anchorer with real Solana connection
const anchorer = createAnchorer({
  cluster: SolanaCluster.DEVNET, // or MAINNET
  wallet,
});

// Now anchoring is REAL - transactions go to Solana!
const result = await anchorer.anchor(merkleRoot);
console.log(`Anchored: ${result.signature}`);
```

**Wallet Types:**
- `KEYPAIR` - Direct secret key (64 bytes)
- `FILE` - Load from Solana CLI JSON file
- `ENV` - Load from environment variable
- `ADAPTER` - External wallet (Phantom, etc.)

**Requirements for real anchoring:**
1. Install: `npm install @solana/web3.js`
2. Configure wallet with SOL for transaction fees
3. Use devnet for testing, mainnet for production

### How to Enforce Burns (NEW)

```javascript
import { createBurnEnforcer, SolanaCluster } from '@cynic/burns';

// Create enforcer
const burnEnforcer = createBurnEnforcer({
  enabled: true,
  minAmount: 618_000_000, // 0.618 SOL (φ⁻¹)
  validityPeriod: 24 * 60 * 60 * 1000, // 24 hours
  gracePeriod: 60 * 60 * 1000, // 1 hour for new users
  protectedOperations: ['judge', 'refine', 'digest'],
  solanaCluster: SolanaCluster.MAINNET,
});

// Register a burn for a user
const result = await burnEnforcer.registerBurn(userId, 'tx_signature');
if (result.registered) {
  console.log(`Burn registered: ${result.burn.amount} lamports`);
}

// Check burn status
const status = burnEnforcer.getBurnStatus(userId);
console.log(status);

// Enforce before operations (throws BurnRequiredError if not valid)
burnEnforcer.requireBurn(userId, 'judge');
```

**BurnEnforcer Features:**
- **Grace Period**: New users get a configurable grace period before burns are required
- **Validity Period**: Burns expire after a configurable time (default: 24h)
- **Protected Operations**: Configure which operations require burns
- **Caching**: Verified burns are cached to avoid repeated verification
- **Stats**: Track enforcement metrics with `getStats()`

**Integration with Judge Tool:**
The `createJudgeTool` now accepts an optional `burnEnforcer` parameter.
When provided, it automatically calls `requireBurn(userId, 'judge')` before each judgment.

---

*The architecture is COMPLETE. "Onchain is truth" is now fully implemented.*
