# GASdf Integration for CYNIC Governance

## What is GASdf?

**GASdf** — **Gas-less Solana DeFi** — is a fee abstraction layer that:
- **Eliminates gas fees** for end users (they don't pay transaction costs)
- **Community treasury absorbs costs** through a non-extractive fee model
- **Supports any SPL token** as payment (not just SOL)
- **76.4% fee burn** to community (φ-bounded deflationary pressure)
- **Stateless execution** (no accounts required)
- **Composable with CYNIC** (verdicts drive fee-subsidized execution)

## Architecture

```
Discord Bot → CYNIC API → GASdf (Fees) → NEAR (Execution)
                             ↓
                       Community Treasury
                             ↓
                       76.4% Burned (Deflationary)
                       23.6% Infrastructure
```

## Why GASdf Matters for Governance

**Without GASdf**:
- Users pay gas → Community bleeds funds → Extraction economy
- Proposals are expensive → Low participation
- Fee revenue goes to validators, not community

**With GASdf**:
- Proposals are free to users → High participation
- Fees burn → Deflationary pressure → Community benefits
- Non-extractive foundation for governance
- Governance decisions improve treasury health

## Components

### 1. Types (`cynic/integrations/gasdf/types.py`)

Dataclasses for GASdf interactions:
- `GASdfError` — Exception for API failures
- `GASdfQuote` — Fee quote with payment token and burn amount
- `GASdfExecutionResult` — Transaction submission result
- `GASdfStats` — Cumulative burn statistics

### 2. Client (`cynic/integrations/gasdf/client.py`)

HTTP client for GASdf API:
- `health()` — Check GASdf service status
- `get_tokens()` — List accepted payment tokens
- `get_quote()` — Request fee quote for transaction
- `submit()` — Submit signed transaction with quote
- `get_stats()` — Get cumulative burn statistics

### 3. Executor (`cynic/integrations/gasdf/executor.py`)

Governance verdict executor:
- `execute_verdict()` — Execute verdict on-chain via GASdf
- Only executes for APPROVED/TENTATIVE_APPROVE verdicts
- Submits signed transaction and handles fee abstraction
- Returns execution result with signature and status

### 4. Burn Sensor (`cynic/integrations/gasdf/burn_sensor.py`)

LNSP Layer 1 integration:
- `observe()` — Poll GASdf stats and emit observations
- Emits ECOSYSTEM_EVENT observations
- Feeds community treasury data into CYNIC judgment loop

## Governance Workflow

### 1. Discord Proposal
```
User: /proposal "Increase treasury allocation by 5%"
     ↓
CYNIC: 11 Dogs evaluate
     ↓
Verdict: WAG (72.5 Q-Score)
```

### 2. Fee Quote Request
```python
quote = await gasdf_client.get_quote(
    payment_token="community.token",
    user_pubkey="user.solana",
    amount=1000000,  # Amount being transferred
)
```

Quote response:
```json
{
    "quote_id": "quote_1234",
    "payment_token": "community.token",
    "fee_amount": 5000,
    "burn_amount": 3820,    // 76.4% of fee
    "user_pubkey": "user.solana"
}
```

### 3. Submit Signed Transaction
```python
executor = GASdfExecutor(gasdf_client)

result = await executor.execute_verdict(
    proposal_id="prop_1234",
    verdict="WAG",
    community_id="memecoin_xyz",
    payment_token="community.token",
    user_pubkey="user.solana",
    signed_transaction=base64_tx,
    payment_token_account="token_acct",
)
```

### 4. On-Chain State
```
Solana Blockchain
├── Treasury Mint
│   └── Burned: 3,820 COIN (76.4% of fees)
│
├── Proposal Contract
│   └── Proposal #1234
│       ├── Status: executed
│       ├── Signature: 5Yk2m...
│       ├── Fee paid: 5,000 COIN
│       └── Burn: 3,820 COIN
│
└── Community Account
    └── Balance: 1,000,000 COIN (treasury)
```

### 5. Learning Loop Integration
```python
# After execution, query stats
stats = await gasdf_client.get_stats()

# Feed back to CYNIC
q_table.update(
    state=state,
    action=verdict,
    reward=calculate_reward_from_burn_stats(stats),
    next_state=next_state,
)
```

Treasury health improves when good verdicts burn fees → CYNIC learns that fair governance has positive externalities.

## Configuration

### Setup GASdf Account

```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.0/install)"

# Create account for community treasury
solana-keygen new --outfile ~/community-treasury.json

# Export keys
export GASDF_TREASURY_PUBKEY=$(solana-keygen pubkey ~/community-treasury.json)
export GASDF_TREASURY_SECRET=$(cat ~/community-treasury.json)
```

### Environment Variables

```env
# GASdf Service
GASDF_API_URL=https://api.gasdf.io/v1  # or testnet
GASDF_API_KEY=your_api_key_here

# Community Treasury
GASDF_TREASURY_PUBKEY=your_treasury_pubkey
GASDF_TREASURY_PRIVATE_KEY=your_private_key_here

# Payment Token
GASDF_PAYMENT_TOKEN=EPjFWaLb3crLMMLhSKMKLU2z3prNhuqpZTKmarsVmqr  # USDC on mainnet
GASDF_PAYMENT_TOKEN_ACCOUNT=community_token_account

# Network
GASDF_NETWORK=mainnet-beta  # or devnet/testnet
```

### Docker Setup

```dockerfile
# Add to Dockerfile
RUN pip install gasdf-py

ENV GASDF_API_URL=https://api.gasdf.io/v1
ENV GASDF_NETWORK=mainnet-beta
ENV GASDF_PAYMENT_TOKEN=EPjFWaLb3crLMMLhSKMKLU2z3prNhuqpZTKmarsVmqr
```

## Integration Points

### 1. Discord Bot → GASdf
When `/proposal` is approved by CYNIC:
```python
from cynic.integrations.gasdf import GASdfExecutor, GASdfClient

config = {
    "api_url": os.getenv("GASDF_API_URL"),
    "api_key": os.getenv("GASDF_API_KEY"),
}

client = GASdfClient(**config)
executor = GASdfExecutor(client)

result = await executor.execute_verdict(
    proposal_id=judgment_id,
    verdict=verdict,
    community_id=community_id,
    payment_token=os.getenv("GASDF_PAYMENT_TOKEN"),
    user_pubkey=user_pubkey,
    signed_transaction=signed_tx,
    payment_token_account=token_account,
)
```

### 2. CYNIC Learning Loop
After verdict execution:
```python
# Query execution stats from GASdf
stats = await executor.client.get_stats()

# Fee burn is observable evidence of good governance
burn_amount = stats.total_burned
transaction_count = stats.total_transactions

# Calculate reward for this verdict
reward = calculate_governance_reward(
    burn_amount=burn_amount,
    transaction_count=transaction_count,
    proposal_quality=judgment_quality,
)

# Update Q-Table
q_table.update(
    state=state,
    action=verdict,
    reward=reward,  # Higher burn = better governance
    next_state=next_state,
)
```

**Key insight**: Fee burn becomes a measurable reward signal. Better governance = more executions = more fees burned = community thrives.

### 3. Community Treasury Integration
GASdf stats feed into treasury health:
```python
# From GASdfBurnSensor
observation = {
    "source": "gasdf",
    "total_burned": 100_000_000,  // 100M tokens
    "total_transactions": 10_000,
    "average_fee": 10_000,
    "treasury_health": "excellent",  // Deflationary + high activity
}

# CYNIC observes this as ECOSYSTEM_EVENT
# Judges whether governance is healthy
# Refines verdicts to maximize sustainable burn
```

## Testing

### Health Check
```python
from cynic.integrations.gasdf import GASdfClient

client = GASdfClient(
    api_url=os.getenv("GASDF_API_URL"),
    api_key=os.getenv("GASDF_API_KEY"),
)

health = await client.health()
assert health["status"] == "ok"
print(f"✓ GASdf service healthy: {health}")
```

### Token Query
```python
tokens = await client.get_tokens()
print("Accepted tokens:")
for token in tokens:
    print(f"  - {token['symbol']}: {token['mint']}")
```

### Fee Quote
```python
quote = await client.get_quote(
    payment_token="EPjFWaLb3crLMMLhSKMKLU2z3prNhuqpZTKmarsVmqr",
    user_pubkey="user_public_key",
    amount=1000000,
)

print(f"Fee: {quote.fee_amount} tokens")
print(f"Burn: {quote.burn_amount} (76.4% to community)")
print(f"Quote ID: {quote.quote_id}")
```

### Integration Test
```bash
# Run tests
python -m pytest tests/integrations/test_gasdf_executor.py -v

# Test verdict execution
python -c "
import asyncio
from cynic.integrations.gasdf import GASdfExecutor, GASdfClient

async def test():
    client = GASdfClient(...)
    executor = GASdfExecutor(client)

    result = await executor.execute_verdict(
        proposal_id='test_prop_1',
        verdict='APPROVED',
        community_id='test_community',
        payment_token='token_mint',
        user_pubkey='user_pubkey',
        signed_transaction='base64_tx',
        payment_token_account='token_account',
    )

    print(f'Execution result: {result}')
    print(f'Signature: {result.signature}')
    print(f'Status: {result.status}')
    print(f'Fee paid: {result.fee_amount} {result.fee_token}')

asyncio.run(test())
"
```

## API Reference

### GASdfClient Methods

```python
# Initialize
client = GASdfClient(
    api_url: str,
    api_key: str,
    timeout: int = 30,
)

# Check health
health = await client.health()  # → dict[str, Any]

# List tokens
tokens = await client.get_tokens()  # → list[dict]

# Request fee quote
quote = await client.get_quote(
    payment_token: str,  # Token mint address
    user_pubkey: str,    # User's public key
    amount: int,         # Transaction amount
)  # → GASdfQuote

# Submit transaction
response = await client.submit(
    quote_id: str,                     # From get_quote()
    signed_transaction: str,           # Base64-encoded signed tx
    payment_token_account: str,        # User's token account
)  # → dict[str, Any]

# Get stats
stats = await client.get_stats()  # → GASdfStats
# Returns: total_burned, total_transactions, burned_formatted, treasury
```

### GASdfExecutor Methods

```python
# Initialize
executor = GASdfExecutor(client: GASdfClient)

# Execute verdict
result = await executor.execute_verdict(
    proposal_id: str,               # Unique proposal ID
    verdict: str,                   # APPROVED/TENTATIVE_APPROVE/CAUTION/REJECT
    community_id: str,              # Community identifier
    payment_token: str,             # Token mint for fees
    user_pubkey: str,               # User's public key
    signed_transaction: str,        # Base64-encoded signed transaction
    payment_token_account: str,     # User's token account
)  # → GASdfExecutionResult | None

# Returns: GASdfExecutionResult if verdict executed, None if skipped
# GASdfExecutionResult has:
#   - signature: str (on-chain transaction signature)
#   - status: str (confirmed/pending/failed)
#   - fee_amount: int (total fee paid)
#   - fee_token: str (token used for fee)
#   - quote_id: str (quote that was used)
```

### GASdfBurnSensor Methods

```python
# Initialize
sensor = GASdfBurnSensor(
    sensor_id: str,          # Unique sensor ID
    client: GASdfClient,     # GASdf client instance
    instance_id: str = "instance:local",
)

# Observe (called by LNSP Layer 1)
observation = await sensor.observe()  # → LNSPMessage | None

# Returns: LNSPMessage with ECOSYSTEM_EVENT observation type
# Contains: total_burned, total_transactions, treasury_health, etc.
```

## Performance

| Metric | Value |
|--------|-------|
| Quote Request | <500ms |
| Transaction Submit | <1s |
| On-Chain Confirmation | 1-2 seconds |
| Fee Burn (% of fee) | 76.4% |
| Infrastructure Cost (%) | 23.6% |
| Supported Tokens | Any SPL |
| Average Fee | ~10,000 tokens |

## Complete Stack

```
┌──────────────────────────────────────────────────────┐
│         Memecoin Community Governance              │
├──────────────────────────────────────────────────────┤
│                                                      │
│  Discord Bot (UI)                                   │
│  └─ /ask_cynic, /proposal, /vote                    │
│                                                      │
│  CYNIC API (Intelligence)                          │
│  └─ 11 Dogs, 5 Axioms, Q-Learning                  │
│                                                      │
│  GASdf (Economics)                                  │
│  └─ Fee quotes, token burning, treasury abstraction│
│                                                      │
│  NEAR Protocol (Execution)                         │
│  └─ On-chain proposals, voting, treasury execution │
│                                                      │
│  Smart Contracts (Governance)                      │
│  └─ create_proposal, vote, execute_proposal        │
│                                                      │
│  Community Treasury (Deflationary)                 │
│  └─ 76.4% fees burned, community benefits          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

## Non-Extractive Fee Model

**The Problem**: Traditional governance bleeds fees
- Treasury pays for proposal submissions → Fund drain
- Users pay gas → Individual pain points
- Validators extract value → Misaligned incentives

**The GASdf Solution**:
```
Proposal Execution
        ↓
   $5,000 Fee
        ↓
   ┌────┴────┐
   ↓         ↓
76.4% Burn 23.6% Infra
Community  Operations
Treasury
Grows
```

**The Outcome**:
- Users submit free proposals (high participation)
- Community treasury absorbs costs (shared burden)
- Fees burn (deflationary, good for community token)
- Infrastructure maintains (sustainable)
- Governance improves (good verdicts = more burns = healthier community)

## Deployment Timeline

- **Week 1**: Deploy GASdf fee infrastructure
- **Week 2**: Test Discord → GASdf integration
- **Week 3**: Connect to NEAR for on-chain execution
- **Week 4**: Run pilot with test community
- **Week 5+**: Deploy to production memecoin communities

## Security Considerations

1. **Quote Expiration**: Quotes expire after 60 seconds (request new quote if needed)
2. **Token Account Verification**: Verify token accounts belong to user before submission
3. **Amount Validation**: Validate transaction amounts match proposal values
4. **Access Control**: Only governance contract can execute approved proposals
5. **Signature Verification**: All transactions must be properly signed
6. **Rate Limiting**: GASdf API has rate limits (100 req/min per key)

## Next Steps

1. Set up GASdf account and testnet credentials
2. Integrate executor into CYNIC action handler
3. Test end-to-end: Discord → CYNIC → GASdf → NEAR
4. Monitor burn statistics and community treasury health
5. Scale to production memecoin communities

## Resources

- [GASdf Documentation](https://docs.gasdf.io/)
- [GASdf API Reference](https://docs.gasdf.io/api)
- [Solana Developer Docs](https://docs.solana.com/)
- [SPL Token Program](https://spl.solana.com/token)
- [gasdf-py SDK](https://github.com/gasdf-io/gasdf-py)
