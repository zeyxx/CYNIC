# CYNIC Architecture

> **Collective Your Node Into Consciousness**
> Decentralized judgment protocol with φ-aligned consensus

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CYNIC ECOSYSTEM                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   @cynic/   │    │   @cynic/   │    │   @cynic/   │         │
│  │    core     │◄───│  protocol   │◄───│    node     │         │
│  │             │    │             │    │             │         │
│  │ φ constants │    │ 4-layer     │    │ Operator    │         │
│  │ Axioms      │    │ protocol    │    │ Transport   │         │
│  └─────────────┘    └─────────────┘    └─────────────┘         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Package Dependencies

```mermaid
graph TD
    subgraph "Monorepo: packages/"
        CORE["@cynic/core<br/>φ constants, axioms"]
        PROTOCOL["@cynic/protocol<br/>4-layer protocol stack"]
        NODE["@cynic/node<br/>Node implementation"]
    end

    NODE --> PROTOCOL
    NODE --> CORE
    PROTOCOL --> CORE

    subgraph "External"
        WS["ws (WebSocket)"]
    end

    NODE --> WS
```

## Protocol Stack (4 Layers)

```mermaid
graph TB
    subgraph "Layer 4: φ-BFT Consensus"
        CONSENSUS["Voting & Proposal<br/>Lockout mechanism<br/>Supermajority: 61.8%"]
    end

    subgraph "Layer 3: Gossip Propagation"
        GOSSIP["P2P Message Relay<br/>Fibonacci fanout (13)<br/>E-Score peer selection"]
    end

    subgraph "Layer 2: Merkle Knowledge Tree"
        MERKLE["Pattern Storage<br/>Judgment aggregation<br/>Tree validation"]
    end

    subgraph "Layer 1: Proof of Judgment (PoJ)"
        POJ["Judgments & Blocks<br/>24 dimensions scoring<br/>HOWL/WAG/GROWL verdicts"]
    end

    CONSENSUS --> GOSSIP
    GOSSIP --> MERKLE
    MERKLE --> POJ
```

## Node Architecture

```mermaid
graph LR
    subgraph "Node Components"
        OP["Operator<br/>Identity, E-Score"]
        JUDGE["Judge<br/>24-dim evaluation"]
        STATE["State Manager<br/>Persistence"]
        TRANSPORT["WebSocket Transport<br/>P2P connections"]
    end

    subgraph "Protocol Integration"
        GP["GossipProtocol"]
        PM["PeerManager"]
    end

    OP --> STATE
    JUDGE --> STATE
    TRANSPORT --> GP
    GP --> PM

    TRANSPORT -.->|"sendFn"| GP
    GP -.->|"message events"| TRANSPORT
```

## WebSocket Transport Flow

```mermaid
sequenceDiagram
    participant N1 as Node1 (Server)
    participant N2 as Node2 (Client)

    N1->>N1: startServer(port)
    N2->>N1: connect(address)
    N1->>N2: IDENTITY (publicKey)
    N2->>N1: IDENTITY (publicKey)

    Note over N1,N2: Identity Exchange Complete

    N2->>N1: JUDGMENT (via gossip)
    N1->>N1: handleMessage()
    N1->>N2: relay to other peers
```

## φ (Phi) Alignment

The golden ratio φ = 1.618... guides all protocol parameters:

| Parameter | Value | Derivation |
|-----------|-------|------------|
| Max Confidence | 61.8% | φ⁻¹ |
| Supermajority | 61.8% | φ⁻¹ |
| Gossip Fanout | 13 | Fibonacci |
| Default Port | 8618 | 8 × φ × 1000 |
| Heartbeat | 61800ms | φ-aligned |

## Judgment Dimensions (24)

4 Axioms × 6 Dimensions each:

```
PHI (Golden Balance)     VERIFY (Truth Seeking)
├── context_awareness    ├── source_verification
├── balanced_analysis    ├── claim_accuracy
├── nuanced_response     ├── evidence_quality
├── appropriate_length   ├── logical_consistency
├── trade_off_handling   ├── bias_detection
└── integration          └── fact_checking

CULTURE (Collective)     BURN (Quality Control)
├── norm_compliance      ├── spam_detection
├── cultural_fit         ├── manipulation_detection
├── community_benefit    ├── toxicity_detection
├── stakeholder_aware    ├── quality_threshold
├── precedent_respect    ├── originality
└── inclusivity          └── value_contribution
```

## Directory Structure

```
CYNIC-new/
├── packages/
│   ├── core/           # Constants, axioms (PHI, PHI_INV, etc.)
│   │   └── src/
│   │       └── axioms/
│   │
│   ├── protocol/       # 4-layer protocol implementation
│   │   ├── src/
│   │   │   ├── poj/        # Layer 1: Proof of Judgment
│   │   │   ├── merkle/     # Layer 2: Knowledge Tree
│   │   │   ├── gossip/     # Layer 3: Propagation
│   │   │   ├── consensus/  # Layer 4: φ-BFT
│   │   │   └── crypto/     # Signatures, hashing
│   │   └── test/
│   │
│   └── node/           # Node implementation
│       ├── src/
│       │   ├── operator/   # Identity, E-Score
│       │   ├── state/      # Persistence
│       │   └── transport/  # WebSocket P2P (NEW)
│       ├── test/
│       └── examples/       # P2P demo scripts
│
└── docs/               # Documentation
```

## Current Branch: `main`

All work is on `main`. No feature branches created yet.

## Test Status

| Package | Tests | Status |
|---------|-------|--------|
| @cynic/protocol | 151 | ✅ Pass |
| @cynic/node | 111 | ✅ Pass |

## Getting Started (New Developer)

```bash
# Install dependencies
npm install

# Run all tests
npm test

# Test P2P network
node packages/node/examples/two-nodes-test.js

# Interactive gossip demo (2 terminals)
# Terminal 1:
node packages/node/examples/gossip-network.js server 8618
# Terminal 2:
node packages/node/examples/gossip-network.js client ws://localhost:8618
```

## Recent Work (Uncommitted)

1. **Transport Layer** (`packages/node/src/transport/`)
   - WebSocket server/client
   - Auto-reconnect with φ exponential backoff
   - Identity exchange protocol
   - Message serialization with checksum

2. **Test Suites** (`packages/*/test/`)
   - Protocol tests (151)
   - Node tests (111)

3. **Bug Fixes**
   - Inbound connection identity handling
   - Peer ID remapping after identity exchange
   - Duplicate connection detection
