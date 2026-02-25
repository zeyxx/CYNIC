# Week 2: Voting & Execution — Implementation Plan

**Status:** Planning complete. Ready to implement.

---

## ⚠️ First: Test the Bot (Week 1 still untested!)

Before building Week 2, you need to invite the bot to a Discord server and test Week 1.

### Step A: Get OAuth2 Invite URL

1. Go to https://discord.com/developers/applications
2. Select **CYNIC Governance Bot**
3. Left sidebar → **OAuth2** → **URL Generator**
4. Scopes: `bot` + `applications.commands`
5. Permissions: `Send Messages`, `Embed Links`, `Read Message History`
6. Copy the generated URL → open it → select your test server → Authorize

### Step B: Register Slash Commands

When the bot starts, Discord slash commands are **not auto-registered**.
You need to sync them once. Restart the bot with this one-time command sync added to `on_ready()`:

```python
@bot.event
async def on_ready():
    logger.info(f"Bot logged in as {bot.user}")
    await init_db()
    # Sync slash commands to Discord
    synced = await bot.tree.sync()
    logger.info(f"Synced {len(synced)} slash commands")
    check_voting_status.start()
```

Add this to `governance_bot/bot.py` then restart the bot.

### Step C: Test Week 1 Commands

In your Discord server, type `/` and look for CYNIC commands:

```
/propose title:"First proposal" description:"Should we test this?" COMMUNITY_DECISION MEDIUM
→ Should show CYNIC verdict (HOWL/WAG/GROWL/BARK) + Q-Score

/voting-status <proposal_id>
→ Should show vote counts and time remaining

/vote <proposal_id> YES
→ Should confirm vote recorded

/cynic-verdict <proposal_id>
→ Should show detailed judgment

/cynic-status
→ Should show CYNIC kernel is online
```

---

## Week 2 Implementation Plan

### What We're Building

```
Voting period ends (auto)
        ↓
Bot closes proposal → determines APPROVED/REJECTED
        ↓
If APPROVED → 24h delay → NEAR execute_proposal()
        ↓
GASdf burn stats reported
        ↓
Discord announcement: "Executed! TX: abc123" + nearblocks.io link
        ↓
/feedback 4 "worked great" → CYNIC learns
```

---

### Step 1: Install py-near (NEAR signing library)

```bash
pip install py-near
```

Add to `governance_bot/requirements.txt`:
```
py-near>=1.1.0
```

**Critical prerequisite:** Check contract owner before wiring signing:
```bash
near view governance.testnet get_owner ''
```
Must return `cynic-gov.testnet`. If not, bot can't call `create_proposal` (contract is owner-only).

---

### Step 2: Fix Crash Bug in formatting.py (line 83)

**File:** `governance_bot/formatting.py:83`

Bug: `project.approval_status` (undefined variable) + missing `f` on f-string.

Fix:
```python
# Change FROM:
"**Approval Status:** {project.approval_status if proposal.approval_status != \"PENDING\" else \"Vote in progress...\"}"

# Change TO:
f"**Approval Status:** {proposal.approval_status if proposal.approval_status != 'PENDING' else 'Vote in progress...'}"
```

---

### Step 3: Add NEAR Environment Variables

**File:** `governance_bot/config.py` — add after `ENABLE_LEARNING_LOOP`:
```python
# NEAR Integration
NEAR_ACCOUNT_ID = os.getenv("NEAR_ACCOUNT_ID", "cynic-gov.testnet")
NEAR_PRIVATE_KEY = os.getenv("NEAR_PRIVATE_KEY", "")
NEAR_CONTRACT_ID = os.getenv("NEAR_CONTRACT_ID", "governance.testnet")
NEAR_NETWORK = os.getenv("NEAR_NETWORK", "testnet")
NEAR_RPC_URL = os.getenv("NEAR_RPC_URL", "https://rpc.testnet.near.org")
ENABLE_NEAR_EXECUTION = os.getenv("ENABLE_NEAR_EXECUTION", "False").lower() == "true"
```

**File:** `governance_bot/.env` — add:
```
NEAR_ACCOUNT_ID=cynic-gov.testnet
NEAR_PRIVATE_KEY=ed25519:YOUR_PRIVATE_KEY_HERE
NEAR_CONTRACT_ID=governance.testnet
NEAR_NETWORK=testnet
NEAR_RPC_URL=https://rpc.testnet.near.org
ENABLE_NEAR_EXECUTION=False
```
(Keep `ENABLE_NEAR_EXECUTION=False` until full testing is done)

---

### Step 4: Implement NEAR Transaction Signing

**File:** `cynic/integrations/near/executor.py`

The method `_execute_contract_call` (line 244) is a TODO stub. Replace its body with real py-near signing.

Pattern:
```python
from py_near.account import Account as PyNearAccount

acc = PyNearAccount(account_id, private_key, rpc_url)
await acc.startup()  # fetches nonce from chain

result = await acc.function_call(
    contract_id,
    method_name,     # "execute_proposal", "create_proposal", etc.
    args,
    gas=contract_call.gas,
    amount=deposit_yocto,
)
tx_hash = result.transaction_outcome.get("id", "")
```

---

### Step 5: Add 5 New Database Functions

**File:** `governance_bot/database.py`

Also fix `check_voting_closed` — replace hardcoded `approval_threshold = 50.0` with:
```python
community = await get_community(session, proposal.community_id)
approval_threshold = community.approval_threshold_percentage if community else 50.0
```

New functions needed:
1. `list_expired_active_proposals(session)` — finds ACTIVE proposals past voting_end_time
2. `schedule_execution(session, proposal_id)` — sets execution_status=SCHEDULED, execution_date=now+24h
3. `list_scheduled_proposals_due(session)` — finds SCHEDULED proposals ready to execute
4. `update_proposal_execution(session, proposal_id, tx_hash, status)` — saves tx hash
5. `create_learning_outcome(session, proposal_id, outcome, rating, comment)` — for /feedback

---

### Step 6: Create execution_pipeline.py (new file)

**File:** `governance_bot/execution_pipeline.py`

Isolated module for: GASdf stats → NEAR execution → DB update.

Key design:
- If `ENABLE_NEAR_EXECUTION=False` → writes `"SIMULATED"` tx hash (all DB/Discord logic still runs)
- GASdf stats are non-blocking (failure doesn't abort execution)
- Returns `{success, near_tx_hash, gasdf_stats, error}`

---

### Step 7: Fill in Background Task (check_voting_status)

**File:** `governance_bot/bot.py:447`

Replace `logger.debug("Checking voting status...")` with real logic:

**Job 1 — Close expired proposals (every 5 min):**
```
list_expired_active_proposals()
  → check_voting_closed() for each
  → APPROVED: schedule_execution() + announce to Discord
  → REJECTED: announce to Discord
```

**Job 2 — Execute scheduled proposals (every 5 min):**
```
list_scheduled_proposals_due()
  → execute_approved_proposal() for each
  → announce TX hash to Discord
  → learn_cynic() (non-fatal if fails)
```

Add helper functions:
- `_announce_proposal_result(proposal, approved)` — posts result to #governance channel
- `_announce_execution_result(proposal, exec_result)` — posts tx hash + nearblocks.io link
- `_find_announcement_channel(guild)` — looks for: governance → announcements → general → first writable

---

### Step 8: Add /feedback Command

**File:** `governance_bot/bot.py`

```
/feedback <proposal_id> <1-5> [comment]
  → 4-5 stars = SUCCESS
  → 3 stars   = PARTIAL
  → 1-2 stars = FAILED
  → create_learning_outcome()
  → update_e_score() for proposer
  → learn_cynic() to improve future verdicts
```

---

## Testing Strategy

### Phase 1: Simulation (no blockchain)

Set `ENABLE_NEAR_EXECUTION=False` in .env, then:
```sql
-- Shorten voting period for testing:
UPDATE communities SET voting_period_hours=0, execution_delay_hours=0
WHERE community_id = 'discord_YOUR_GUILD_ID';
```

1. `/propose "Test" "testing Week 2"` → proposal created, CYNIC judges
2. `/vote <id> YES` → vote recorded
3. Wait 5 min (bg task cycle) → proposal closes → Discord announces APPROVED
4. Wait another 5 min → execution scheduled → Discord announces "SIMULATED" tx
5. `/feedback <id> 4 "works"` → learning loop fires

### Phase 2: Real NEAR testnet

1. Add `NEAR_PRIVATE_KEY` to `.env`
2. Run: `python scripts/verify_near_setup.py`
3. Set `ENABLE_NEAR_EXECUTION=True`
4. Repeat test sequence → real tx hash on nearblocks.io

---

## Files Changed

| File | Change |
|------|--------|
| `governance_bot/requirements.txt` | Add `py-near>=1.1.0` |
| `governance_bot/bot.py` | Sync commands in on_ready; fill bg task; add /feedback; add 3 announcement helpers; update imports |
| `governance_bot/formatting.py` | Fix line-83 crash bug; add /feedback to help text |
| `governance_bot/config.py` | Add 5 NEAR env vars |
| `governance_bot/.env` + `.env.example` | Add NEAR vars |
| `cynic/integrations/near/executor.py` | Implement _execute_contract_call with py-near |
| `governance_bot/database.py` | Fix hardcoded threshold; add 5 new functions; add uuid/LearningOutcome imports |
| `governance_bot/execution_pipeline.py` | **NEW** — execution module |

---

## Estimated Scope

- ~8 files modified, 1 new file
- ~400 lines of new code total
- All builds on existing patterns — no new architecture decisions

**Week 2 unlocks:** Real on-chain governance. Proposals aren't just Discord messages — they execute on NEAR Protocol.
