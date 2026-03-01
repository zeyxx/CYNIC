# NEAR Protocol Integration for CYNIC Governance

## What is NEAR Protocol?

**NEAR** stands for **NEAR Protocol** — a Layer-1 blockchain optimized for:
- **Speed**: 1-2 second finality
- **Scalability**: Sharded architecture supporting millions of transactions
- **Developer Experience**: Simple account model, WebAssembly smart contracts
- **Low Cost**: ~$0.0001 per transaction
- **Interoperability**: Rainbow Bridge to Ethereum ecosystem

## Architecture

```
Discord Bot → CYNIC API → GASdf (Fees) → NEAR (Execution)
                             ↓
                       Community Treasury
                             ↓
                       Smart Contracts
```

## Components

### 1. Types (`cynic/integrations/near/types.py`)

Dataclasses for NEAR interactions:
- `NEARAccount` — Account information
- `NEARTransaction` — Transaction details
- `NEARExecutionResult` — Execution outcomes
- `NEARGovernanceProposal` — On-chain proposals
- `NEARNetworkConfig` — Network settings
- `NEARContractCall` — Smart contract calls

### 2. RPC Client (`cynic/integrations/near/rpc_client.py`)

HTTP client for NEAR RPC API:
- `health()` — Check node status
- `get_account()` — Query account state
- `get_nonce()` — Get next transaction nonce
- `get_block_hash()` — Get current block hash
- `call_contract()` — Read-only contract calls
- `send_transaction()` — Submit signed transactions
- `get_transaction_result()` — Poll for confirmation

### 3. Executor (`cynic/integrations/near/executor.py`)

Governance action executor:
- `submit_proposal()` — Create on-chain proposal with CYNIC verdict
- `record_vote()` — Record community vote
- `execute_proposal()` — Finalize approved proposal
- `query_proposal()` — Get proposal state from blockchain
- `wait_for_confirmation()` — Poll until confirmation

## Governance Workflow

### 1. Discord Proposal
```
User: /proposal "Increase treasury by 5%"
     ↓
CYNIC: 11 Dogs evaluate
     ↓
Verdict: WAG (72.5 Q-Score)
```

### 2. Submit to NEAR
```python
executor = NEARExecutor(config)

result = await executor.submit_proposal(
    proposal_id="prop_1234",
    title="Increase treasury by 5%",
    description="For ecosystem development",
    cynic_verdict="WAG",
    q_score=72.5,
    signer_id="governance.near",
    expires_at=1740000000,
)
```

### 3. On-Chain State
```
NEAR Blockchain
├── Governance Contract
│   └── Proposal #1234
│       ├── Title: "Increase treasury..."
│       ├── CYNIC Verdict: WAG
│       ├── Q-Score: 72.5
│       ├── Created: block 98765432
│       ├── Votes:
│       │   ├── For: 42
│       │   ├── Against: 8
│       │   └── Abstain: 3
│       └── Status: approved
│
└── Treasury Contract
    └── Balance: 1,000 NEAR
```

### 4. Record Votes
```python
# User votes via Discord reaction
# Bot records on-chain
vote_result = await executor.record_vote(
    proposal_id="prop_1234",
    voter_id="user.near",
    vote="for",
    weight=1,
)
```

### 5. Execute Proposal
```python
# Once voting period closes and passes threshold
exec_result = await executor.execute_proposal(
    proposal_id="prop_1234",
    executor_id="governance.near",
)
```

## Configuration

### Setup NEAR Account

```bash
# Install NEAR CLI
npm install -g near-cli

# Create account or use existing
near create-account governance.testnet

# Export private key
export NEAR_ACCOUNT_ID=governance.testnet
export NEAR_PRIVATE_KEY=$(cat ~/.near-credentials/testnet/governance.testnet.json)
```

### Environment Variables

```env
# Network
NEAR_NETWORK=testnet  # or mainnet
NEAR_RPC_URL=https://rpc.testnet.near.org  # or mainnet

# Contract
NEAR_GOVERNANCE_CONTRACT=governance.testnet
NEAR_MASTER_ACCOUNT=governance.testnet

# Account
NEAR_ACCOUNT_ID=governance.testnet
NEAR_PRIVATE_KEY=your_private_key_here
```

### Docker Setup

```dockerfile
# Add to Dockerfile
RUN pip install near-api

ENV NEAR_NETWORK=testnet
ENV NEAR_RPC_URL=https://rpc.testnet.near.org
ENV NEAR_GOVERNANCE_CONTRACT=governance.testnet
ENV NEAR_MASTER_ACCOUNT=governance.testnet
```

## Smart Contract (Rust)

The governance contract handles:

```rust
pub struct Proposal {
    pub proposal_id: String,
    pub title: String,
    pub description: String,

    // CYNIC Integration
    pub cynic_verdict: String,
    pub cynic_q_score: u32,

    // Voting
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub status: ProposalStatus,

    // Timeline
    pub created_at: u64,
    pub expires_at: u64,
}

#[near_bindgen]
pub impl Governance {
    pub fn create_proposal(
        &mut self,
        proposal_id: String,
        title: String,
        description: String,
        cynic_verdict: String,
        cynic_q_score: u32,
        expires_at: u64,
    ) -> Proposal {
        // Create proposal with CYNIC verdict attached
    }

    pub fn vote(
        &mut self,
        proposal_id: String,
        vote: String,
    ) -> VoteResult {
        // Record vote from user
    }

    pub fn execute_proposal(
        &mut self,
        proposal_id: String,
    ) -> ExecutionResult {
        // Execute if passes voting threshold
    }
}
```

## Integration Points

### 1. Discord Bot → NEAR
When `/proposal` is submitted:
```python
from cynic.kernel.organism.perception.integrations.near import NEARExecutor, NEARNetworkConfig

config = NEARNetworkConfig(
    network_id="testnet",
    rpc_url=os.getenv("NEAR_RPC_URL"),
    contract_id=os.getenv("NEAR_GOVERNANCE_CONTRACT"),
    master_account=os.getenv("NEAR_MASTER_ACCOUNT"),
)

executor = NEARExecutor(config)
result = await executor.submit_proposal(
    proposal_id=judgment_id,
    title=question,
    description=context,
    cynic_verdict=verdict,
    q_score=q_score,
    signer_id="governance.near",
    expires_at=int(time.time()) + 7_days,
)
```

### 2. CYNIC Learning Loop
After proposal executes:
```python
# Query on-chain result
proposal = await executor.query_proposal(proposal_id)

# Feed back to CYNIC
q_table.update(
    state=state,
    action=verdict,
    reward=calculate_reward(proposal.votes_for, proposal.votes_against),
    next_state=next_state,
)
```

### 3. GASdf Fee Handling
Fees for on-chain actions:
```
User submits proposal via Discord
     ↓
GASdf quotes fee in community token
     ↓
NEAR transaction executes
     ↓
Fee burned to community treasury (76.4%)
     ↓
CYNIC records learning feedback
```

## Testing

### Testnet Deployment

```bash
# Set environment
export NEAR_NETWORK=testnet

# Initialize testnet contract
near deploy governance.testnet \
  --accountId governance.testnet \
  ./governance.wasm

# Test proposal creation
python -m pytest tests/integrations/test_near_executor.py
```

### Local Node (Optional)

```bash
# Run local NEAR node
docker run --rm -p 3030:3030 nearprotocol/nearcore

# Update RPC URL
export NEAR_RPC_URL=http://localhost:3030
```

## API Reference

### NEARExecutor Methods

```python
# Initialize
executor = NEARExecutor(config)

# Check health
healthy = await executor.health()

# Submit proposal
result = await executor.submit_proposal(
    proposal_id: str,
    title: str,
    description: str,
    cynic_verdict: str,  # HOWL/WAG/GROWL/BARK
    q_score: float,      # 0-100
    signer_id: str,
    expires_at: int,
)

# Record vote
result = await executor.record_vote(
    proposal_id: str,
    voter_id: str,
    vote: str,           # for/against/abstain
    weight: int = 1,
)

# Execute proposal
result = await executor.execute_proposal(
    proposal_id: str,
    executor_id: str,
)

# Query proposal
proposal = await executor.query_proposal(proposal_id: str)

# Wait for confirmation
result = await executor.wait_for_confirmation(
    tx_hash: str,
    signer_id: str,
    timeout_seconds: int = 60,
)
```

## Performance

| Metric | Value |
|--------|-------|
| Block Time | 1-2 seconds |
| Finality | Near-instant (optimistic) |
| Gas Cost | ~0.0001 NEAR per transaction |
| Contract Call Cost | ~0.00025 NEAR |
| Average TPS | 5,000+ (with sharding) |

## Complete Stack

```
┌─────────────────────────────────────────────────────┐
│         Memocoin Community Governance              │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Discord Bot (UI)                                  │
│  └─ /ask_cynic, /proposal, /vote                   │
│                                                     │
│  CYNIC API (Intelligence)                          │
│  └─ 11 Dogs, 5 Axioms, Q-Learning                  │
│                                                     │
│  GASdf (Economics)                                 │
│  └─ Fee quotes, token burning, community treasury  │
│                                                     │
│  NEAR Protocol (Execution)                         │
│  └─ On-chain proposals, voting, treasury execution │
│                                                     │
│  Smart Contracts (Governance)                      │
│  └─ create_proposal, vote, execute_proposal        │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Deployment Timeline

- **Week 1**: Deploy governance contract to testnet
- **Week 2**: Test Discord → NEAR integration
- **Week 3**: Run pilot with test community
- **Week 4**: Measure outcomes and refine
- **Week 5+**: Deploy to mainnet with real communities

## Security Considerations

1. **Private Key Management**: Use secure key storage (not env vars in production)
2. **Rate Limiting**: Prevent spam proposals (require deposit)
3. **Access Control**: Only governance.near can execute proposals
4. **Audit Logging**: All on-chain actions immutable and traceable
5. **Contract Audits**: NEAR contract reviewed before mainnet

## Next Steps

1. Deploy governance contract to testnet
2. Integrate executor into CYNIC action handler
3. Test end-to-end: Discord → CYNIC → GASdf → NEAR
4. Measure governance quality improvements
5. Scale to production

## Resources

- [NEAR Protocol Docs](https://docs.near.org/)
- [NEAR RPC API](https://docs.near.org/api/rpc/introduction)
- [near-sdk-rs](https://github.com/near/near-sdk-rs) (contract development)
- [near-api-py](https://github.com/nearprotocol/near-api-py) (Python SDK)
