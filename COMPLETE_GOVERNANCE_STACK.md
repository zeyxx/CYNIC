# Complete Governance Stack: Discord → CYNIC → GASdf → NEAR

## Overview

The complete CYNIC governance system is a four-layer stack that enables decentralized, fair, and learning-based governance for memecoin communities:

```
┌──────────────────────────────────────────────────────────────┐
│                    Governance Stack                          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Layer 0: User Interface (Discord)                          │
│  ├─ /ask_cynic — Ask CYNIC to judge a question             │
│  ├─ /proposal — Propose governance action                   │
│  ├─ /vote — Vote on proposals (reactions → on-chain)        │
│  └─ /cynic_status — Check CYNIC and treasury health         │
│                      ↓                                       │
│  Layer 1: Intelligence (CYNIC API)                          │
│  ├─ 11 Dogs + 5 Axioms (FIDELITY/PHI/VERIFY/CULTURE/BURN) │
│  ├─ Q-Learning with φ-bounded Q-Scores (max 61.8%)         │
│  ├─ Verdict generation (HOWL/WAG/GROWL/BARK)               │
│  └─ Learning loop feedback integration                      │
│                      ↓                                       │
│  Layer 2: Economics (GASdf)                                 │
│  ├─ Gasless transaction abstraction                         │
│  ├─ Non-extractive fee model (76.4% burn to community)      │
│  ├─ Quote generation and fee deduction                      │
│  └─ Treasury health monitoring (ECOSYSTEM_EVENT sensor)     │
│                      ↓                                       │
│  Layer 3: Execution (NEAR Protocol)                         │
│  ├─ On-chain proposal storage                               │
│  ├─ Community voting with CYNIC verdict attached            │
│  ├─ Treasury transactions                                   │
│  └─ Immutable audit trail for learning feedback             │
│                                                              │
│  Output: Community treasury grows (deflationary)            │
│          CYNIC learns what good governance looks like       │
│          Network effects compound across communities        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Complete Workflow

### Phase 1: Discord Interaction (Layer 0)

User opens Discord and proposes an action:

```
User: /proposal "Increase liquidity provision to DEX by 10%"
```

Discord bot captures the proposal and sends to CYNIC API:

```python
# From cynic/discord/bot.py
@bot.tree.command(name="proposal")
async def proposal(interaction: discord.Interaction, title: str):
    await interaction.response.defer()

    payload = {
        "question": title,
        "context": f"Community proposal from {interaction.user}",
    }

    async with bot.cynic_session.post(
        f"{CYNIC_API_URL}/judge",
        json=payload
    ) as resp:
        judgment = await resp.json()
        # verdict, q_score, reasoning attached to proposal
```

### Phase 2: CYNIC Intelligence (Layer 1)

CYNIC receives the proposal and evaluates it through 11 Dogs + 5 Axioms:

```python
# From cynic/api/routers/core.py
@router.post("/judge")
async def judge(request: JudgeRequest) -> JudgmentResponse:
    # 11 Dogs evaluate independently
    evaluations = await cynic_organism.judge(
        question=request.question,
        context=request.context,
    )

    # Consensus: 5 Axioms + Q-Learning
    verdict, q_score, reasoning = await consensus_engine.decide(evaluations)
    # verdict ∈ {HOWL, WAG, GROWL, BARK}
    # q_score ∈ [0, 0.618] (φ-bounded)
    # reasoning: full explanation

    return JudgmentResponse(
        verdict=verdict,
        q_score=q_score,
        reasoning=reasoning,
        proposal_id=str(uuid4()),
    )
```

The verdict comes back to Discord with color-coded embed:

```
CYNIC Verdict: WAG ✓

Confidence: 72.5% (Q-Score: 0.725)

Reasoning:
- Liquidity provision increases trading volume (PHI ✓)
- Aligns with protocol incentives (FIDELITY ✓)
- Verified against market conditions (VERIFY ✓)
- Community culture supports DEX activity (CULTURE ✓)
- Non-extractive, benefits ecosystem (BURN ✓)

Status: Ready for community vote
React with 👍/👎/🤷 to vote
```

### Phase 3: Community Voting & Fee Abstraction (Layer 2)

Community votes via Discord reactions:

```
User reacts 👍 to proposal

Discord Bot → CYNIC API:
  {
    "proposal_id": "prop_1234",
    "voter_id": "user.near",
    "vote": "for",  // 👍
    "weight": 1,
  }

CYNIC records the vote and checks threshold
If vote.threshold_met:
  1. Send to GASdf for fee quote
  2. Create transaction
  3. Submit via GASdf
```

GASdf handles the fee abstraction:

```python
# From cynic/integrations/gasdf/executor.py
executor = GASdfExecutor(gasdf_client)

result = await executor.execute_verdict(
    proposal_id="prop_1234",
    verdict="WAG",
    community_id="memecoin_xyz",
    payment_token="token_mint",
    user_pubkey=user_pubkey,
    signed_transaction=signed_tx,
    payment_token_account=token_account,
    q_score=0.725,
)

# GASdf handles:
# 1. Get fee quote
# 2. Deduct from community token account
# 3. Burn 76.4% to community treasury
# 4. Pay 23.6% for infrastructure
```

### Phase 4: On-Chain Execution (Layer 3)

NEAR Protocol records the proposal on-chain with full context:

```python
# From cynic/integrations/near/executor.py
near_executor = NEARExecutor(near_config)

on_chain_result = await near_executor.submit_proposal(
    proposal_id="prop_1234",
    title="Increase liquidity provision to DEX by 10%",
    description="Will increase trading volume and community engagement",
    cynic_verdict="WAG",
    q_score=0.725,
    signer_id="governance.near",
    expires_at=int(time.time()) + 7 * 24 * 3600,
)
```

On-chain state shows complete governance record:

```
NEAR Blockchain
├── Governance Contract
│   └── Proposal #1234
│       ├── Title: "Increase liquidity provision..."
│       ├── Description: "Will increase trading volume..."
│       ├── CYNIC Verdict: WAG
│       ├── Q-Score: 0.725
│       ├── Status: voted
│       ├── Votes:
│       │   ├── For: 42 votes
│       │   ├── Against: 3 votes
│       │   └── Abstain: 2 votes
│       ├── Execution State:
│       │   ├── GASdf Fee: 5,000 COIN
│       │   ├── Burned (76.4%): 3,820 COIN
│       │   ├── Infrastructure (23.6%): 1,180 COIN
│       │   └── Status: executed
│       └── Timeline:
│           ├── Created: block 98765432
│           ├── Voted: block 98765500
│           └── Executed: block 98765550
│
└── Community Treasury
    ├── Previous balance: 1,000,000 COIN
    ├── Burn from proposal: -3,820 COIN
    ├── Execution txn: +5,000 COIN (transferred)
    └── Current balance: 1,001,180 COIN
        (net +1,180 from infrastructure fees)
```

## Data Flow Across Layers

### Request Flow (Discord → NEAR)

```
Discord User Input
        ↓
bot.py (Layer 0)
  - Capture proposal/vote
  - Format JSON
        ↓
CYNIC API (Layer 1)
  - 11 Dogs evaluate
  - Generate verdict + q_score
        ↓
GASdf Executor (Layer 2)
  - Check verdict (execute if HOWL/WAG)
  - Request fee quote
  - Deduct from treasury
        ↓
NEAR Executor (Layer 3)
  - Submit signed transaction
  - Record on-chain
  - Return signature
        ↓
Discord Bot
  - Show confirmation embed
  - Update proposal status
```

### Feedback Flow (NEAR → CYNIC Learning Loop)

```
NEAR Blockchain
  (on-chain proposal results)
        ↓
GASdfBurnSensor (Layer 2 → Layer 1)
  - Poll /v1/stats
  - Emit ECOSYSTEM_EVENT observation
  - Track treasury_burned
        ↓
CYNIC Q-Learning Loop
  - Receive burn statistics
  - Calculate reward signal
    reward = burn_amount / time_to_execution
  - Update Q-Table:
    Q[state, verdict] += α * (reward + γ * max(Q[next_state]))
        ↓
Better verdicts over time
  (CYNIC learns what good governance looks like)
```

## Key Integration Points

### 1. Discord Bot ↔ CYNIC API

**File**: `cynic/discord/bot.py`

```python
async def ask_cynic(interaction: discord.Interaction, question: str):
    payload = {
        "question": question,
        "context": f"From {interaction.user} in {interaction.channel}",
    }

    async with bot.cynic_session.post(
        f"{CYNIC_API_URL}/judge",
        json=payload,
        timeout=aiohttp.ClientTimeout(total=CYNIC_API_TIMEOUT)
    ) as resp:
        judgment = await resp.json()
        # Use judgment.verdict and judgment.q_score
```

### 2. CYNIC API ↔ GASdf Executor

**File**: `cynic/api/handlers/judgment_executor.py`

```python
# After CYNIC makes a judgment
if should_execute(verdict):
    gasdf_executor = GASdfExecutor(gasdf_client)

    result = await gasdf_executor.execute_verdict(
        proposal_id=proposal_id,
        verdict=verdict,
        community_id=community_id,
        payment_token=GASDF_PAYMENT_TOKEN,
        user_pubkey=user_pubkey,
        signed_transaction=signed_tx,
        payment_token_account=token_account,
        q_score=q_score,
    )

    # result.fee_amount and result.signature for logging
```

### 3. GASdf Executor ↔ NEAR Executor

**File**: `cynic/api/handlers/governance_executor.py`

```python
# After GASdf execution succeeds
if gasdf_result.status == "confirmed":
    near_executor = NEARExecutor(near_config)

    on_chain_result = await near_executor.submit_proposal(
        proposal_id=proposal_id,
        title=title,
        description=description,
        cynic_verdict=verdict,
        q_score=q_score,
        signer_id="governance.near",
        expires_at=vote_deadline,
    )

    # on_chain_result.transaction_hash for audit trail
```

### 4. NEAR Results ↔ CYNIC Learning Loop

**File**: `cynic/protocol/lnsp/layer1.py`

```python
# GASdfBurnSensor polls NEAR treasury
sensor = GASdfBurnSensor(
    sensor_id="gasdf_burn_1",
    client=gasdf_client,
)

observation = await sensor.observe()
# Returns: LNSPMessage with ECOSYSTEM_EVENT
# Contains: total_burned, treasury_health, burn_rate

# LNSP Layer 1 aggregates observation
# Layer 2-3 use for judgment refinement
# Q-Table updates with burn-derived reward signal
```

## Complete Example: End-to-End Governance

### Step 1: User Proposes via Discord

```
User: /proposal "Increase treasury allocation to marketing by 10%"
```

### Step 2: CYNIC Evaluates

```
CYNIC internally:
- Dog1: "Marketing increases awareness (positive for ecosystem)"
- Dog2: "10% is reasonable (not excessive)"
- Dog3: "Treasury has capacity (sustainable)"
- Dog4: "Community sentiment is positive (from past votes)"
- Dog5-11: Additional evaluations...

Consensus verdict: WAG (yes, with medium confidence)
Q-Score: 0.65 (65% confidence, φ-bounded)
```

### Step 3: Discord Shows Verdict

```
Embed in Discord:
┌─────────────────────────────────┐
│ CYNIC Verdict: WAG ✓            │
│ Confidence: 65%                 │
│                                 │
│ "Good governance decision that  │
│  aligns with community growth"  │
│                                 │
│ React to vote:                  │
│ 👍 = Support (for)              │
│ 👎 = Oppose (against)           │
│ 🤷 = Abstain                    │
└─────────────────────────────────┘
```

### Step 4: Community Votes

```
5 users react 👍 (for)
1 user reacts 👎 (against)
2 users react 🤷 (abstain)

Total: 5 yes, 1 no, 2 abstain
Threshold: 60% approval needed → 5/6 = 83% ✓ PASS
```

### Step 5: GASdf Handles Fees

```
GASdf executor.execute_verdict():
- Verdict WAG + q_score 0.65 → Execute? YES
- Request fee quote for 100,000 token transfer
- Quote response: fee 5,000 tokens
- Burn calculation: 5,000 × 0.764 = 3,820 tokens
- Deduct from community treasury token account
- Submit transaction with quote
```

### Step 6: NEAR Records Everything

```
NEAR submit_proposal():
- Store proposal with verdict attached
- Record community vote counts
- Record GASdf fee and burn amount
- Store transaction signature
- Immutable on-chain proof
```

### Step 7: Execution Completes

```
Discord shows:
✅ Proposal #1234 executed successfully

Status:
- CYNIC Verdict: WAG (65% confidence)
- Community Vote: 83% approval (5 yes, 1 no, 2 abstain)
- Fee (GASdf): 5,000 tokens
- Burned to treasury: 3,820 tokens
- On-chain signature: 5Yk2m...

Treasury update:
- Before: 1,000,000 tokens
- Transfer: +100,000 tokens (marketing budget)
- Burn: -3,820 tokens (GASdf fee)
- After: 1,096,180 tokens (net +96,180)
```

### Step 8: Learning Loop Updates

```
GASdfBurnSensor observes:
- Total burned this month: 45,600 tokens
- Total transactions: 12
- Average burn per tx: 3,800 tokens
- Treasury health: GOOD

CYNIC Q-Learning:
- Reward = burn_amount / execution_time
- Update Q-Table[state=marketing_proposal, action=WAG]
- Q-value increases because verdict correlated with treasury growth
- Next similar proposal: CYNIC more confident in WAG verdict
```

## Security & Sustainability

### Non-Extractive Model Ensures:

1. **Community Owns Treasury**
   - No founder extraction
   - No VC fees
   - No centralized control

2. **Governance Quality Improves Over Time**
   - CYNIC learns which verdicts lead to treasury growth
   - Better verdicts → more executions → more burns → healthier community
   - Learning loop is self-reinforcing

3. **Transparent Audit Trail**
   - Every proposal on NEAR blockchain
   - Every verdict visible on Discord
   - Every fee burn tracked in GASdf stats
   - Community can verify everything

### Deployment Readiness Checklist

- [ ] Discord bot running with all commands
- [ ] CYNIC API healthy and responsive
- [ ] GASdf account created and funded
- [ ] NEAR governance contract deployed to testnet
- [ ] Environment variables configured (.env)
- [ ] Integration tests passing
- [ ] Monitoring/logging configured
- [ ] Runbook created for operational issues

## Performance Targets

| Layer | Latency | Throughput | Notes |
|-------|---------|-----------|-------|
| Discord → API | <100ms | N/A | In-process |
| CYNIC evaluation | 500ms - 2s | 1-10 judgments/min | 11 Dogs in parallel |
| GASdf fee quote | <500ms | 100+ quotes/min | Stateless |
| NEAR submit | 1-2s | 100+ txns/min | 1-2s finality |
| Learning feedback | 5-10s | Per execution | Async batch updates |

## Next Steps

1. **Deploy to testnet** with test community (Week 1)
2. **Run pilot** with real memecoin community (Week 2-3)
3. **Monitor metrics** — treasury growth, governance quality (Week 4)
4. **Measure learning** — verify Q-Table improvements (Week 5)
5. **Scale** — deploy to 5-10 additional communities (Month 2-3)

## Resources

- [Discord Bot Integration](./DISCORD_BOT_INTEGRATION.md)
- [GASdf Integration](./GASDF_INTEGRATION.md)
- [NEAR Integration](./NEAR_INTEGRATION.md)
- [CYNIC Architecture](./cynic/protocol/lnsp/README.md)
