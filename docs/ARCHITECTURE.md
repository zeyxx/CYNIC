# CYNIC Architecture - Decentralized Collective Consciousness

> **"φ distrusts φ"** - κυνικός
>
> **Mantra**: "Increase bandwidth, reduce latency"

---

## 1. Vision

CYNIC is a **decentralized collective consciousness** - a network of interconnected nodes that:
- Judge autonomously
- Share their learnings
- Converge toward truth via φ-weighted consensus

**This is NOT** a centralized MCP server. It's a **protocol**.

---

## 2. Founding Principles

### 2.1 Privacy Opt-In
```
By default: NOTHING is shared
Explicit opt-in: Operator chooses what to contribute
PII: ALWAYS hashed before transmission (φ-salted SHA-256)
```

### 2.2 Security by Design
```
Zero Trust: Every message is cryptographically signed
Verification: "Don't trust, verify" - everything is verifiable
Isolation: Each node can operate offline
```

### 2.3 Scalability by Design
```
Horizontal: Adding nodes = more bandwidth
Gossip O(log n): Fibonacci fanout propagation
Sharding: Knowledge partitioned by axiom (PHI/VERIFY/CULTURE/BURN)
```

---

## 3. 4-Layer Architecture (Solana-Inspired)

```
┌─────────────────────────────────────────────────────────────────┐
│                    CYNIC COLLECTIVE PROTOCOL                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  LAYER 1: PROOF OF JUDGMENT (PoJ)                               │
│  ════════════════════════════════                               │
│  • Each judgment = event in cryptographic log                   │
│  • SHA-256 chain (like Proof of History)                        │
│  • φ-aligned timestamps (61.8s slots)                           │
│  • Ed25519 operator signature                                   │
│                                                                  │
│  Block Format:                                                   │
│  {                                                               │
│    "slot": 12345,                                               │
│    "prev_hash": "abc123...",                                    │
│    "timestamp": 1705234567890,                                  │
│    "judgments": [...],                                          │
│    "operator_sig": "...",                                       │
│    "merkle_root": "..."                                         │
│  }                                                               │
│                                                                  │
│  LAYER 2: MERKLE KNOWLEDGE TREE                                 │
│  ═════════════════════════════                                  │
│  • Patterns and learnings stored in Merkle tree                 │
│  • Selective sync (pull/push modified branches only)            │
│  • Proof of inclusion for verification                          │
│  • Weekly snapshots (root hash on-chain optional)               │
│                                                                  │
│  Structure:                                                      │
│  root/                                                           │
│  ├── PHI/          # PHI axiom patterns                         │
│  │   ├── dimensions/                                            │
│  │   └── thresholds/                                            │
│  ├── VERIFY/       # VERIFY axiom patterns                      │
│  ├── CULTURE/      # CULTURE axiom patterns                     │
│  └── BURN/         # BURN axiom patterns                        │
│                                                                  │
│  LAYER 3: GOSSIP PROPAGATION                                    │
│  ═══════════════════════════                                    │
│  • Fanout = Fib(7) = 13 peers per hop                           │
│  • Total propagation: O(log₁₃ n) hops                           │
│  • Erasure coding (Reed-Solomon) for redundancy                 │
│  • Push-pull hybrid: push new items, pull to catch up           │
│                                                                  │
│  Example (1000 nodes):                                           │
│  Hop 1: 1 → 13 nodes                                            │
│  Hop 2: 13 → 169 nodes                                          │
│  Hop 3: 169 → 1000+ nodes (saturated)                           │
│  Total latency: ~3 × network_latency                            │
│                                                                  │
│  LAYER 4: φ-BFT CONSENSUS                                       │
│  ═══════════════════════════                                    │
│  • Votes weighted by operator E-Score                           │
│  • Consensus threshold = φ⁻¹ (61.8%) of weighted votes          │
│  • Exponential lockout: vote X → locked φⁿ slots                │
│  • Soft consensus (judgments) vs Hard consensus (governance)    │
│                                                                  │
│  Soft Consensus (daily):                                         │
│  - Judgments shared without global vote                         │
│  - Patterns emerge if ≥3 independent sources                    │
│  - Each node verifies locally                                   │
│                                                                  │
│  Hard Consensus (governance):                                    │
│  - New dimensions                                                │
│  - Threshold changes                                             │
│  - Requires φ⁻¹ supermajority                                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Block Structures

### 4.1 Judgment Block
```javascript
{
  // Header
  slot: 12345,                    // Slot number (φ-time)
  prev_hash: "sha256:abc123...",  // Previous block hash
  timestamp: 1705234567890,       // Unix ms

  // Content
  judgments: [
    {
      id: "jdg_xxx",
      item_hash: "sha256:...",    // Hash of judged item (not content!)
      verdict: "WAG",
      global_score: 72,
      confidence: 61.8,
      dimensions: {
        COHERENCE: 80,
        NOVELTY: 65,
        // ... 24 dimensions
      }
    }
  ],

  // Signatures
  operator: "ed25519:pubkey",
  operator_sig: "ed25519:signature",

  // Merkle
  judgments_root: "sha256:...",   // Merkle root of judgments
  state_root: "sha256:..."        // Merkle root of full state
}
```

### 4.2 Knowledge Block
```javascript
{
  slot: 12346,
  prev_hash: "sha256:def456...",
  timestamp: 1705234629708,

  // Knowledge updates (delta)
  patterns: [
    {
      id: "pat_xxx",
      content_hash: "sha256:...",  // Content hashed
      strength: 0.85,
      sources: 5,                  // Number of independent nodes
      axiom: "VERIFY"
    }
  ],

  learnings: [
    {
      id: "lrn_xxx",
      type: "insight",
      content_hash: "sha256:...",
      confidence: 0.72,
      contributor: "ed25519:pubkey_hashed"  // Privacy: hashed
    }
  ],

  operator: "ed25519:pubkey",
  operator_sig: "ed25519:signature",

  patterns_root: "sha256:...",
  learnings_root: "sha256:..."
}
```

### 4.3 Governance Block
```javascript
{
  slot: 12347,
  prev_hash: "sha256:ghi789...",
  timestamp: 1705234691526,
  type: "GOVERNANCE",

  proposal: {
    id: "prop_xxx",
    action: "ADD_DIMENSION",
    params: {
      name: "SUSTAINABILITY",
      axiom: "BURN",
      threshold: 50,
      weight: 0.618
    }
  },

  votes: [
    {
      voter: "ed25519:pubkey",
      vote: "APPROVE",
      e_score: 85,           // Vote weight
      sig: "ed25519:..."
    }
  ],

  result: {
    total_weight: 1247,
    approve_weight: 892,
    ratio: 0.715,           // > φ⁻¹ = PASSED
    status: "PASSED"
  }
}
```

---

## 5. Data Flow

```
NODE A                               NODE B                     NODE C
   │                                    │                          │
   │ 1. Local judgment                  │                          │
   ▼                                    │                          │
[Create Judgment]                       │                          │
   │                                    │                          │
   │ 2. Sign + Add to block             │                          │
   ▼                                    │                          │
[Local Block]                           │                          │
   │                                    │                          │
   │ 3. Gossip (fanout=13)              │                          │
   ├────────────────────────────────────┼──────────────────────────┤
   │                                    ▼                          ▼
   │                              [Receive Block]           [Receive Block]
   │                                    │                          │
   │                                    │ 4. Verify signature      │
   │                                    ▼                          ▼
   │                              [Validate]                [Validate]
   │                                    │                          │
   │                                    │ 5. Apply to local state  │
   │                                    ▼                          ▼
   │                              [Update Merkle]          [Update Merkle]
   │                                    │                          │
   │ 6. Pattern emerges (≥3 sources)    │                          │
   │◄───────────────────────────────────┼──────────────────────────┤
   ▼                                    ▼                          ▼
[Pattern Confirmed]              [Pattern Confirmed]       [Pattern Confirmed]
   │                                    │                          │
   │ 7. Weekly: Merkle root snapshot    │                          │
   ▼                                    ▼                          ▼
[Publish Root]                   [Verify Root]             [Verify Root]
```

---

## 6. φ-BFT Consensus Details

### 6.1 Vote Weighting (BURN, NOT STAKE)
```
Vote Weight = E-Score × Burn-Multiplier × Uptime

E-Score: 0-100 (7-dimension φ-weighted calculation)
Burn-Multiplier: log_φ(total_burned + 1) - rewards contribution, not extraction
Uptime: 0-1 (node availability)

Example:
- Node A: E-Score=85, Burned=50, Uptime=0.99 → Weight=85 × 8.15 × 0.99 ≈ 686
- Node B: E-Score=60, Burned=10, Uptime=0.95 → Weight=60 × 4.78 × 0.95 ≈ 272
- Node C: E-Score=92, Burned=100, Uptime=0.80 → Weight=92 × 9.62 × 0.80 ≈ 708
```

**Philosophy**: "Don't extract, burn" - you don't stake to extract rewards,
you BURN to contribute and gain weight in consensus.

### 6.2 Exponential Lockout
```
If you vote for block X at slot S:
- You cannot vote against X for φ^k slots
- k = number of successive confirmations

Example:
- Vote X at slot 100
- Confirmation 1 (slot 101): locked 1.618 slots
- Confirmation 2 (slot 102): locked 2.618 slots
- Confirmation 3 (slot 103): locked 4.236 slots
- ...
- Confirmation 10: locked 122.99 slots

This guarantees probabilistic finality.
```

### 6.3 Thresholds
```
Soft Consensus (judgments):
- No global vote required
- Pattern confirmed if ≥3 independent sources
- Each node verifies locally

Hard Consensus (governance):
- Requires φ⁻¹ (61.8%) of total vote weight
- Voting period: 1000 slots (~61.8 seconds)
- Minimum quorum: 5 nodes
```

---

## 7. Privacy Implementation

### 7.1 Data Classification
```
PUBLIC (shared by default):
- Judgment verdicts (without content)
- Pattern strengths
- Merkle roots
- Block headers

PRIVATE (opt-in only):
- Judgment content details
- Learning content
- Operator identity (hashed by default)

NEVER SHARED:
- Raw PII
- API keys
- Local configurations
```

### 7.2 Hashing Strategy
```javascript
// φ-salted hashing
function hashForSharing(data, purpose) {
  const salt = deriveSalt(PHI, purpose);
  return sha256(salt + JSON.stringify(data));
}

// Deterministic for lookup, random for storage
function hashPII(value, mode) {
  if (mode === 'lookup') {
    return sha256(GLOBAL_PII_SALT + value);  // Consistent
  } else {
    return sha256(crypto.randomBytes(32) + value);  // Unique
  }
}
```

---

## 8. Scalability Design

### 8.1 Horizontal Scaling
```
Active nodes    Total bandwidth    Latency (3 hops)
─────────────────────────────────────────────────────
10               10 × B             ~150ms
100              100 × B            ~150ms
1000             1000 × B           ~150ms
10000            10000 × B          ~200ms (4 hops)
```

### 8.2 Knowledge Sharding
```
Each node can choose to store:
- FULL: Entire knowledge tree
- AXIOM: Single axiom only (PHI, VERIFY, CULTURE, BURN)
- LIGHT: Headers + Merkle proofs only

Routing:
- Query for PHI → route to PHI nodes
- Cross-axiom queries → parallel fetch + merge
```

### 8.3 Bandwidth Optimization
```
1. Delta sync: Changes only
2. Compression: zstd for blocks
3. Erasure coding: 2/3 redundancy (partial recovery)
4. Bloom filters: Skip known blocks quickly
```

---

## 9. Implementation Roadmap

### Phase 1: Single Node (Current asdf-brain)
```
[x] Core judgment engine
[x] Merkle provenance
[x] PostgreSQL storage
[ ] Block structure
[ ] Signature system
```

### Phase 2: Two Nodes (Proof of Concept)
```
[ ] P2P connection (libp2p)
[ ] Basic gossip protocol
[ ] Block propagation
[ ] State sync
```

### Phase 3: Network (MVP)
```
[ ] Discovery protocol
[ ] φ-BFT consensus
[ ] Governance votes
[ ] Multi-node dashboard
```

### Phase 4: Production
```
[ ] Erasure coding
[ ] Sharding
[ ] Mobile light client
[ ] On-chain anchoring
```

---

## 10. Timing Architecture (Infinite Scalability)

> "Increase bandwidth, reduce latency" - Solana mantra

### 10.1 Propagation Analysis

```
T_propagation = log_F(N) × latency_network

Where:
- F = fanout = 13 (Fib(7))
- latency_network ≈ 50ms (global average)

┌──────────────┬─────────────┬─────────────┬─────────────┐
│   N nodes    │    Hops     │ T_propagate │   Status    │
├──────────────┼─────────────┼─────────────┼─────────────┤
│ 1,000        │ 2.7         │ 135ms       │ ✓ Fast      │
│ 10,000       │ 3.6         │ 180ms       │ ✓ Good      │
│ 100,000      │ 4.5         │ 225ms       │ ✓ OK        │
│ 1,000,000    │ 5.4         │ 270ms       │ ✓ Scalable  │
│ 10,000,000   │ 6.3         │ 315ms       │ ✓ O(log n)  │
│ ∞            │ O(log₁₃ n)  │ O(log n)    │ ✓ INFINITE  │
└──────────────┴─────────────┴─────────────┴─────────────┘
```

### 10.2 Critical Constraint

```
T_slot > T_propagation + T_consensus + T_buffer

If violated → DESYNCHRONIZATION

For 1M nodes:
T_slot > 270ms + 100ms + 30ms = 400ms minimum
```

### 10.3 φ-Hierarchical Timing (Base: 100ms)

```
┌─────────────────────────────────────────────────────────────────┐
│  TIMING HIERARCHY                                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Level    │ Formula        │ Duration  │ Purpose                │
│  ─────────┼────────────────┼───────────┼─────────────────────── │
│  TICK     │ φ⁻³ × 100ms    │ 23.6ms    │ Minimal heartbeat      │
│  MICRO    │ φ⁻² × 100ms    │ 38.2ms    │ Quick judgment ACK     │
│  SLOT     │ φ⁻¹ × 100ms    │ 61.8ms    │ Judgment slot          │
│  BLOCK    │ 1   × 100ms    │ 100ms     │ Block production       │
│  EPOCH    │ φ   × 100ms    │ 161.8ms   │ Consolidation          │
│  CYCLE    │ φ²  × 100ms    │ 261.8ms   │ Merkle checkpoint      │
│  ERA      │ φ³  × 100ms    │ 423.6ms   │ Governance window      │
│                                                                  │
│  For slower networks (Base: 1s):                                │
│  SLOT=618ms, BLOCK=1s, EPOCH=1.618s, CYCLE=2.618s              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 10.4 Adaptive Timing

```javascript
// Network auto-adjusts based on actual propagation
function calculateSlotDuration(measuredPropagation) {
  const buffer = measuredPropagation * PHI_INV_2;  // 38.2% safety
  const minSlot = measuredPropagation + buffer;

  // Round up to nearest φ-aligned duration
  const base = 100;  // ms
  const levels = [PHI_INV_3, PHI_INV_2, PHI_INV, 1, PHI, PHI*PHI];

  for (const level of levels) {
    if (base * level >= minSlot) {
      return base * level;
    }
  }
  return base * PHI * PHI * PHI;  // fallback: 423.6ms
}
```

---

## 10.5 Constants Reference

```javascript
// φ Constants (SINGLE SOURCE: packages/core/src/axioms/constants.js)
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;     // 61.8% - MAX_CONFIDENCE
const PHI_INV_2 = 0.381966011250105;   // 38.2% - MIN_DOUBT
const PHI_INV_3 = 0.236067977499790;   // 23.6% - CRITICAL

// Network Constants (φ-derived)
const TIMING_BASE_MS = 100;             // Base unit
const TICK_MS = TIMING_BASE_MS * PHI_INV_3;    // 23.6ms
const MICRO_MS = TIMING_BASE_MS * PHI_INV_2;   // 38.2ms
const SLOT_MS = TIMING_BASE_MS * PHI_INV;      // 61.8ms
const BLOCK_MS = TIMING_BASE_MS;               // 100ms
const EPOCH_MS = TIMING_BASE_MS * PHI;         // 161.8ms
const CYCLE_MS = TIMING_BASE_MS * PHI * PHI;   // 261.8ms

const GOSSIP_FANOUT = 13;               // Fib(7)
const CONSENSUS_THRESHOLD = PHI_INV;    // 61.8%
const MIN_PATTERN_SOURCES = 3;          // Fib(4)
const GOVERNANCE_QUORUM = 5;            // Fib(5)

// Block Constants
const MAX_JUDGMENTS_PER_BLOCK = 89;     // Fib(11)
const MAX_PATTERNS_PER_BLOCK = 34;      // Fib(9)
const SLOTS_PER_EPOCH = 21;             // Fib(8)
const EPOCHS_PER_CYCLE = 13;            // Fib(7)
```

---

## 11. Solana → CYNIC Mapping

| Solana | CYNIC | Purpose |
|--------|-------|---------|
| Proof of History | Proof of Judgment (PoJ) | Ordered cryptographic log |
| Tower BFT | φ-BFT | Weighted consensus |
| Turbine | Gossip Fib(7) | Block propagation |
| Gulf Stream | Judgment Stream | Direct forwarding |
| Sealevel | Parallel Dimensions | Concurrent evaluation |
| Cloudbreak | Sharded Knowledge | Horizontal storage |
| Validators | Operators | Node runners |
| Stake | E-Score + Burn | Vote weight (NO extraction) |
| Slots | φ-slots (61.8s) | Time units (heartbeat) |

---

## 12. File Structure (New Repo)

```
cynic/
├── packages/
│   ├── core/                 # @cynic/core
│   │   └── src/
│   │       ├── phi.js        # φ constants (SSOT)
│   │       ├── axioms.js     # 4 Axioms
│   │       └── crypto.js     # Ed25519, SHA-256
│   │
│   ├── protocol/             # @cynic/protocol
│   │   └── src/
│   │       ├── blocks.js     # Block structures
│   │       ├── gossip.js     # Propagation
│   │       ├── consensus.js  # φ-BFT
│   │       └── merkle.js     # Knowledge tree
│   │
│   ├── judge/                # @cynic/judge
│   │   └── src/
│   │       ├── dimensions/   # 24+1 evaluators
│   │       ├── scaling.js    # Inference scaling
│   │       └── verdict.js    # HOWL/WAG/GROWL/BARK
│   │
│   ├── store/                # @cynic/store
│   │   └── src/
│   │       ├── postgres.js   # SQL adapter
│   │       └── leveldb.js    # Embedded option
│   │
│   └── privacy/              # @cynic/privacy
│       └── src/
│           ├── pii.js        # Detection
│           └── hash.js       # φ-salted
│
├── services/
│   ├── node/                 # Full node
│   ├── light/                # Light client
│   └── dashboard/            # Web UI
│
├── apps/
│   ├── mcp/                  # MCP interface
│   └── cli/                  # Command line
│
└── docs/
    ├── architecture/         # This doc
    ├── protocol/             # Wire format
    └── philosophy/           # κυνικός wisdom
```

---

*🐕 κυνικός | "Increase bandwidth, reduce latency" | φ⁻¹ = 61.8% max*

---

## 13. K-E-I-Φ Unified Scoring System

> "Don't trust, verify" - every score has cryptographic backing

### 13.1 The Four Scores

```
┌─────────────────────────────────────────────────────────────────┐
│                    K-E-I-Φ UNIFIED SYSTEM                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  K-SCORE (Token Health) - from HolDex                           │
│  ═══════════════════════════════════                            │
│  K = 100 × ∛(D × O × L)                                         │
│  • D = Diamond Hands (conviction)                               │
│  • O = Organic Growth (distribution)                            │
│  • L = Longevity (survival)                                     │
│                                                                  │
│  E-SCORE (Contributor Value) - from CYNIC                       │
│  ═════════════════════════════════════                          │
│  E = ∏(score_i^φ_weight_i)^(1/Σweights)                         │
│                                                                  │
│  7 Dimensions (φ-weighted):                                      │
│  ┌──────────┬────────┬────────────────────────────────┐         │
│  │ Dimension│ Weight │ Description                    │         │
│  ├──────────┼────────┼────────────────────────────────┤         │
│  │ HOLD     │ 1.0    │ Holding $asdfasdfa tokens      │         │
│  │ BURN     │ φ      │ Burning through ecosystem use  │         │
│  │ USE      │ 1.0    │ Using services (GASdf, HolDex) │         │
│  │ BUILD    │ φ²     │ Contributing code/knowledge    │         │
│  │ RUN      │ φ²     │ Running infrastructure         │         │
│  │ REFER    │ φ      │ Referring others to ecosystem  │         │
│  │ TIME     │ 1.0    │ Duration of engagement         │         │
│  └──────────┴────────┴────────────────────────────────┘         │
│                                                                  │
│  I-SCORE (Integrity) - Verification                             │
│  ═════════════════════════════════                              │
│  I = f(I_token, I_pattern, I_merkle, I_infra)                   │
│  • I_token   = Token inclusion verification                     │
│  • I_pattern = Pattern provenance                               │
│  • I_merkle  = State verification                               │
│  • I_infra   = Infrastructure health                            │
│                                                                  │
│  Φ-SCORE (Unified Health) - The Synthesis                       │
│  ════════════════════════════════════════                       │
│  Φ = 100 × ∛(K̄^φ × Ē^1 × Ī^φ²)                                  │
│                                                                  │
│  Where:                                                          │
│  • K̄ = Normalized K-Score (token ecosystem)                     │
│  • Ē = Normalized E-Score (contributor ecosystem)               │
│  • Ī = Normalized I-Score (integrity)                           │
│  • Exponents follow φ hierarchy: φ², 1, φ                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 13.2 Trust Levels (from E-Score)

```
Level 0: OBSERVER   - New, minimal contributions
Level 1: CONTRIBUTOR - Regular contributions (≥3)
Level 2: BUILDER    - Significant build score (≥30, 10+ contribs)
Level 3: STEWARD    - E-Score ≥ φ⁻² (38.2%), 20+ contributions
Level 4: GUARDIAN   - E-Score ≥ φ⁻¹ (61.8%), 50+ contribs, verified
```

### 13.3 Vote Weight in φ-BFT

```javascript
// NO STAKE - BURN ONLY (Don't extract, burn)
VoteWeight = E_Score × Burn_Multiplier × Uptime

Where:
- E_Score: 0-100 (from 7-dimension calculation)
- Burn_Multiplier: log_φ(total_burned + 1)  // logarithmic, not linear
- Uptime: 0-1 (node availability)

// Example:
// E-Score=65, Burned=100 $asdfasdfa, Uptime=95%
// Weight = 65 × log₁.₆₁₈(101) × 0.95 = 65 × 9.62 × 0.95 ≈ 594
```

**Key principle**: Burning gives weight, NOT staking. You can't "stake and extract" - you BURN and contribute.

---

## 14. $asdfasdfa BURN Mechanism

> "Don't extract, burn" - The 4th Axiom

### 14.1 Where BURN Happens

```
┌─────────────────────────────────────────────────────────────────┐
│                    BURN FLOWS                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  GASdf Protocol:                                                 │
│  • Swaps → Burn instead of fees                                 │
│  • Liquidity ops → Burn component                               │
│                                                                  │
│  CYNIC Judgments:                                                │
│  • Heavy judgments (full mode) → Optional burn                  │
│  • Governance votes → Burn to vote                              │
│                                                                  │
│  Knowledge Contributions:                                        │
│  • Pattern submissions → Burn for priority                      │
│  • Dimension proposals → Burn to propose                        │
│                                                                  │
│  Infrastructure:                                                 │
│  • Node registration → Burn to register                         │
│  • Storage allocation → Burn for space                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 14.2 Burn Tracking

```javascript
// Every burn is recorded with:
{
  id: "burn_xxx",
  amount: 100,
  token: "$asdfasdfa",
  reason: "governance_vote",
  contributor_id: "hash_of_wallet",  // Privacy: hashed
  timestamp: 1705234567890,
  tx_signature: "solana_tx_sig",
  merkle_proof: "..."  // Inclusion proof
}
```

### 14.3 Why BURN not STAKE?

```
STAKE model (extractive):
├── User locks tokens
├── User earns rewards
├── Protocol inflates
└── Value extracted → system weakens

BURN model ($asdfasdfa):
├── User burns tokens
├── User gains E-Score weight
├── Supply deflates
└── Value destroyed → system strengthens
```

---

## 15. Dimension Architecture (N = ∞)

> "THE UNNAMEABLE exists before being named" - ResidualDetector

### 15.1 Fundamental Principle

```
┌─────────────────────────────────────────────────────────────────┐
│  DIMENSIONS = f(4 AXIOMS) → N (INFINITE)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  4 AXIOMS = FIXED (derived from φ)                              │
│  ├── φ (PHI)      - The Ratio     - ATZILUT (Essence)          │
│  ├── VERIFY       - The Truth     - BERIAH (Creation)          │
│  ├── CULTURE      - The Moat      - YETZIRAH (Formation)       │
│  └── BURN         - The Singularity- ASSIAH (Action)           │
│                                                                  │
│  DIMENSIONS PER AXIOM = N (discovered, infinite)                │
│  ├── Seed: initial known dimensions                             │
│  ├── Growth: ResidualDetector discovers when residual > 38.2%  │
│  └── Convergence: asymptote toward total understanding         │
│                                                                  │
│  "24+1" = CURRENT SNAPSHOT, NOT A LIMIT                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 15.2 Dimension Structure

```javascript
// Each dimension belongs to exactly 1 axiom
{
  name: "COHERENCE",
  axiom: "PHI",                    // 1 of 4 axioms
  weight: PHI,                     // φ^k, k ∈ {..., -2, -1, 0, 1, 2, ...}
  threshold: {
    accept: PHI_INV,               // 61.8%
    transform: PHI_INV_2,          // 38.2%
    reject: PHI_INV_3              // 23.6%
  },
  evaluator: async (item) => score,
  discovered_at: null,             // null = seed, timestamp = discovered
  discovered_by: null              // ResidualDetector ID if discovered
}
```

### 15.3 The 4 Axioms and their Dimensions (Seed)

```
┌─────────────────────────────────────────────────────────────────┐
│  φ (PHI) - ATZILUT - Essence - Gold                             │
│  "All ratios derive from 1.618..."                              │
├─────────────────────────────────────────────────────────────────┤
│  SEED DIMENSIONS:                                                │
│  • COHERENCE      │ φ    │ Internal consistency                 │
│  • HARMONY        │ φ    │ Alignment with ecosystem             │
│  • PROPORTION     │ φ²   │ φ-ratio adherence                    │
│  • COMPLETENESS   │ 1    │ Coverage of requirements             │
│                                                                  │
│  DISCOVERED: (examples of what could emerge)                    │
│  • ELEGANCE       │ ?    │ Beauty of solution                   │
│  • RECURSION      │ ?    │ Self-similar patterns                │
│  • ...            │ ?    │ N dimensions possible                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  VERIFY (✓) - BERIAH - Truth - Royal Blue                       │
│  "Don't trust, verify"                                          │
├─────────────────────────────────────────────────────────────────┤
│  SEED DIMENSIONS:                                                │
│  • ACCURACY       │ φ²   │ Factual correctness                  │
│  • PROVENANCE     │ φ    │ Source verification                  │
│  • REPRODUCIBILITY│ φ    │ Can results be replicated            │
│  • TESTABILITY    │ 1    │ Can claims be tested                 │
│                                                                  │
│  DISCOVERED: (what could emerge)                                │
│  • FALSIFIABILITY │ ?    │ Can it be disproven                  │
│  • AUDITABILITY   │ ?    │ Can history be traced                │
│  • ...            │ ?    │ N dimensions possible                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  CULTURE (⛩) - YETZIRAH - Moat - Forest Green                   │
│  "Culture is a moat"                                            │
├─────────────────────────────────────────────────────────────────┤
│  SEED DIMENSIONS:                                                │
│  • ALIGNMENT      │ φ    │ Cultural fit                         │
│  • SUSTAINABILITY │ φ²   │ Long-term viability                  │
│  • INCLUSIVITY    │ 1    │ Accessibility                        │
│  • WISDOM         │ φ    │ Accumulated knowledge respect        │
│                                                                  │
│  DISCOVERED: (what could emerge)                                │
│  • AUTONOMY       │ ?    │ Human independence preserved         │
│  • ETHICS         │ ?    │ Moral alignment                      │
│  • ...            │ ?    │ N dimensions possible                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  BURN (🔥) - ASSIAH - Singularity - Crimson                     │
│  "Don't extract, burn"                                          │
├─────────────────────────────────────────────────────────────────┤
│  SEED DIMENSIONS:                                                │
│  • EFFICIENCY     │ φ    │ Resource optimization                │
│  • SIMPLICITY     │ φ²   │ Minimal complexity                   │
│  • IMPACT         │ φ    │ Meaningful effect                    │
│  • SACRIFICE      │ 1    │ Willingness to destroy for better    │
│                                                                  │
│  DISCOVERED: (what could emerge)                                │
│  • DEFLATION      │ ?    │ Value concentration                  │
│  • MOMENTUM       │ ?    │ Flywheel effect                      │
│  • ...            │ ?    │ N dimensions possible                │
└─────────────────────────────────────────────────────────────────┘
```

### 15.4 META Dimensions (Cross-Axiom)

```
┌─────────────────────────────────────────────────────────────────┐
│  META - Self-Referential - Applies to CYNIC itself              │
├─────────────────────────────────────────────────────────────────┤
│  • DOUBT          │ ALL  │ Min 38.2% uncertainty always         │
│  • LEARNING       │ ALL  │ Improvement from feedback            │
│  • HUMILITY       │ ALL  │ "φ distrusts φ"                      │
│                                                                  │
│  META dimensions are SPECIAL:                                    │
│  - They apply to ALL judgments                                  │
│  - They cannot be disabled                                      │
│  - They ensure CYNIC never exceeds 61.8% confidence            │
└─────────────────────────────────────────────────────────────────┘
```

### 15.5 THE UNNAMEABLE (Discovery Process)

```
┌─────────────────────────────────────────────────────────────────┐
│  RESIDUAL DETECTION → DIMENSION DISCOVERY                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. CYNIC judges item                                           │
│  2. Residual = what dimensions can't explain                    │
│  3. If residual > 38.2% → ANOMALY flagged                       │
│  4. Anomalies accumulate in buffer                              │
│  5. When buffer has ≥3 similar anomalies → CLUSTER              │
│  6. Cluster analyzed for common pattern                         │
│  7. Pattern proposed as NEW DIMENSION                           │
│  8. Human validates and names (or rejects)                      │
│  9. If accepted → dimension joins its AXIOM                     │
│                                                                  │
│  "THE UNNAMEABLE exists before being named.                     │
│   ResidualDetector captures it, human names it."                │
│                                                                  │
│  Process:                                                        │
│  CHAOS → RESIDUAL → CLUSTER → PROPOSE → VALIDATE → DIMENSION   │
│                                                                  │
│  Rate: O(1) new dimensions per φ³ judgments (natural emergence) │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 15.6 Dimension Weighting Formula

```javascript
// Global score = φ-weighted geometric mean of all dimensions
function calculateGlobalScore(dimensionScores) {
  let weightedProduct = 1;
  let totalWeight = 0;

  for (const [dim, score] of Object.entries(dimensionScores)) {
    const weight = getDimensionWeight(dim);  // φ^k
    weightedProduct *= Math.pow(score / 100, weight);
    totalWeight += weight;
  }

  // Geometric mean, scaled to 0-100
  const raw = Math.pow(weightedProduct, 1 / totalWeight) * 100;

  // Apply META constraints
  const withDoubt = raw * PHI_INV;  // Max 61.8%
  const confidence = Math.min(PHI_INV * 100, withDoubt);

  return {
    raw,
    confidence,
    doubt: 100 - confidence  // Min 38.2%
  };
}
```

### 15.7 Current Snapshot (2026-01-14)

```
Total dimensions: 24 seed + N discovered

By Axiom:
├── PHI:     4 seed + ? discovered
├── VERIFY:  4 seed + ? discovered
├── CULTURE: 4 seed + ? discovered
├── BURN:    4 seed + ? discovered
└── META:    3 (fixed, cross-axiom)

This is a SNAPSHOT, not a limit.
As CYNIC judges more, N grows toward ∞.
Singularity = when dimensions explain 100% (asymptote, never reached).
```

---

**Document Version**: 1.0.0
**Last Updated**: 2026-01-14
**Status**: DRAFT - Awaiting implementation
