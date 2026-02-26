# Phase 1 & 2 Implementation Plan
**Goal:** Production-ready governance bot + NEAR integration
**Timeline:** 12-17 days
**Status:** Starting execution

---

## Phase 1: Discord Bot Hardening (7-10 days)

### Task 1.1: Error Handling & Recovery
**Files to modify:** governance_bot/bot.py, governance_bot/views.py

```python
# Current: Bot crashes on error
# Needed: Graceful error handling

# 1. Add try-catch around all command handlers
# 2. Add error logging to file + Discord notifications
# 3. Add recovery procedures (restart bot safely)
# 4. Add circuit breaker (if CYNIC down, pause governance)
```

**Subtasks:**
- [ ] Add comprehensive error handling to ProposalModal.on_submit()
- [ ] Add error handling to VotingView._handle_vote()
- [ ] Add error handling to OutcomeRatingView._handle_rating()
- [ ] Add error logging system (file + Discord channel)
- [ ] Add circuit breaker for CYNIC unavailability
- [ ] Add graceful shutdown handlers

**Tests needed:**
- [ ] Test error recovery (bot survives exception)
- [ ] Test error logging (messages appear in log)
- [ ] Test circuit breaker (governance pauses when CYNIC down)

**Effort:** 2-3 days

---

### Task 1.2: Database Reliability
**Files to modify:** governance_bot/database.py

```python
# Current: Single SQLite connection
# Needed: Connection pooling + transaction management

# 1. Add SQLAlchemy session pooling
# 2. Add transaction management (atomicity)
# 3. Add data consistency checks
# 4. Add backup strategy
```

**Subtasks:**
- [ ] Implement connection pool (max 5 concurrent connections)
- [ ] Add transaction management (begin/commit/rollback)
- [ ] Add data consistency verification on startup
- [ ] Add automatic backup to timestamped file
- [ ] Add schema migration system (alembic)
- [ ] Add database health check command

**Tests needed:**
- [ ] Test concurrent access (multiple votes simultaneously)
- [ ] Test transaction rollback on error
- [ ] Test data consistency after crash
- [ ] Test backup/restore

**Effort:** 2-3 days

---

### Task 1.3: Configuration Management
**Files to create:** .env.template, governance_bot/config.py (enhanced)

```python
# Current: Ad-hoc environment variables
# Needed: Validated configuration system

# 1. Create .env.template with all required variables
# 2. Add configuration validation on startup
# 3. Add environment-specific configs (testnet/mainnet)
# 4. Add secrets rotation capability
```

**Subtasks:**
- [ ] Create .env.template with defaults
- [ ] Add pydantic ConfigSettings class
- [ ] Validate all required variables on startup
- [ ] Add testnet/mainnet environment selector
- [ ] Add configuration hot-reload capability
- [ ] Document all configuration options

**Tests needed:**
- [ ] Test invalid configuration detected
- [ ] Test missing variables caught
- [ ] Test environment switching

**Effort:** 1-2 days

---

### Task 1.4: Deployment Readiness
**Files to create:** docker/Dockerfile, docker-compose.yml, kubernetes/*.yaml

```python
# Current: Manual startup (python governance_bot/bot.py)
# Needed: Container-based deployment

# 1. Create Dockerfile for bot
# 2. Create docker-compose for local testing
# 3. Create K8s manifests for production
# 4. Add health check endpoints
```

**Subtasks:**
- [ ] Create Dockerfile (Python 3.13, dependencies)
- [ ] Create docker-compose.yml (bot + sqlite)
- [ ] Create K8s deployment manifest
- [ ] Add /health endpoint to bot
- [ ] Add liveness/readiness probes
- [ ] Document deployment process

**Tests needed:**
- [ ] Test Docker build succeeds
- [ ] Test docker-compose startup
- [ ] Test health endpoint responds

**Effort:** 1-2 days

---

### Task 1.5: Integration Testing
**Files to create:** tests/test_bot_integration.py

```python
# Current: Unit tests only
# Needed: Bot integration tests

# 1. Test proposal creation end-to-end
# 2. Test vote recording
# 3. Test CYNIC judgment integration
# 4. Test outcome processing
```

**Subtasks:**
- [ ] Create test Discord server (use discord.py TestBot)
- [ ] Test ProposalModal submission flow
- [ ] Test VotingView button interactions
- [ ] Test error scenarios (CYNIC timeout, DB failure)
- [ ] Test concurrent operations
- [ ] Test recovery from crashes

**Tests needed:**
- [ ] 10+ bot integration tests
- [ ] Error scenario coverage

**Effort:** 2-3 days

---

## Phase 2: NEAR Integration Completion (5-7 days)

### Task 2.1: Transaction Signing
**Files to modify:** cynic/integrations/near/executor.py, cynic/integrations/near/rpc_client.py

```python
# Current: Transaction creation only (no signing)
# Needed: Full transaction signing + submission

# 1. Implement ed25519 signing
# 2. Implement transaction serialization
# 3. Add nonce management
# 4. Add error recovery for failed transactions
```

**Subtasks:**
- [ ] Install near-api-py dependency
- [ ] Implement KeyStore for key management
- [ ] Implement transaction signing (ed25519)
- [ ] Implement transaction serialization (Borsh)
- [ ] Add nonce tracking per account
- [ ] Add transaction retry logic
- [ ] Add signature verification

**Tests needed:**
- [ ] Test transaction signing produces valid signature
- [ ] Test nonce increments correctly
- [ ] Test serialization/deserialization
- [ ] Test signature verification

**Effort:** 2-3 days

---

### Task 2.2: Contract Deployment
**Files to create:** contracts/governance.near, scripts/deploy_contract.py

```python
# Current: Contract methods defined, not deployed
# Needed: Governance contract on testnet

# 1. Create governance contract (Rust/AssemblyScript)
# 2. Implement create_proposal, vote, execute_proposal methods
# 3. Deploy to NEAR testnet
# 4. Initialize contract state
```

**Subtasks:**
- [ ] Write governance contract in Rust (or AssemblyScript)
- [ ] Implement create_proposal method
- [ ] Implement vote method
- [ ] Implement execute_proposal method
- [ ] Implement get_proposal query method
- [ ] Build contract (wasm)
- [ ] Deploy to testnet
- [ ] Initialize contract state
- [ ] Document contract ABI

**Tests needed:**
- [ ] Test contract compiles to wasm
- [ ] Test contract deploys to testnet
- [ ] Test contract methods callable via RPC

**Effort:** 2-3 days

---

### Task 2.3: Transaction Submission & Confirmation
**Files to modify:** cynic/integrations/near/executor.py

```python
# Current: _execute_contract_call returns PENDING status
# Needed: Actually submit to NEAR + wait for confirmation

# 1. Implement transaction submission to NEAR RPC
# 2. Implement confirmation polling
# 3. Implement timeout handling
# 4. Implement error recovery
```

**Subtasks:**
- [ ] Implement send_transaction() to NEAR RPC
- [ ] Implement poll_transaction_status() with exponential backoff
- [ ] Add confirmation timeout (30-60 seconds)
- [ ] Add transaction receipt parsing
- [ ] Add failure reason detection
- [ ] Add retry logic for transient failures
- [ ] Add timeout recovery

**Tests needed:**
- [ ] Test transaction submits successfully
- [ ] Test confirmation polling works
- [ ] Test timeout triggers properly
- [ ] Test failure detection

**Effort:** 2 days

---

### Task 2.4: NEAR Integration Testing
**Files to create:** tests/test_near_integration_live.py

```python
# Current: NEAR types and contract calls mocked
# Needed: Live NEAR testnet integration tests

# 1. Test actual proposal submission to testnet
# 2. Test vote recording on testnet
# 3. Test proposal execution on testnet
# 4. Test confirmation and outcomes
```

**Subtasks:**
- [ ] Create test NEAR testnet account
- [ ] Test submit_proposal to live testnet
- [ ] Test record_vote to live testnet
- [ ] Test execute_proposal to live testnet
- [ ] Test query_proposal from testnet
- [ ] Test transaction confirmation
- [ ] Test error scenarios (insufficient balance, etc.)

**Tests needed:**
- [ ] 5+ live NEAR testnet integration tests
- [ ] All transaction types tested
- [ ] Error scenarios covered

**Effort:** 2 days

---

## Implementation Order

**Week 1:**
- Day 1-2: Task 1.1 (Error handling)
- Day 2-3: Task 1.2 (Database reliability)
- Day 3-4: Task 1.3 (Configuration)
- Day 4-5: Task 1.4 (Deployment)

**Week 2:**
- Day 6-7: Task 1.5 (Bot integration tests)
- Day 8-9: Task 2.1 (Transaction signing)
- Day 10-11: Task 2.2 (Contract deployment)
- Day 12-13: Task 2.3 (Submission & confirmation)
- Day 14-15: Task 2.4 (NEAR integration tests)

**Total: 15 days (12-17 expected range)**

---

## Success Criteria

### Phase 1 Complete
- ✅ Discord bot handles errors gracefully
- ✅ No unhandled exceptions crash the bot
- ✅ Database survives concurrent access
- ✅ Configuration validated on startup
- ✅ Bot runs in Docker
- ✅ 10+ integration tests passing
- ✅ Bot can run 24/7 without manual intervention

### Phase 2 Complete
- ✅ Transactions signed and submitted to NEAR
- ✅ Governance contract deployed to testnet
- ✅ Proposals created on-chain
- ✅ Votes recorded on-chain
- ✅ Proposals executed on-chain
- ✅ Transaction confirmation working
- ✅ 5+ NEAR integration tests passing

### End-to-End Flow
```
User: /propose "Increase rewards"
  ↓
Discord bot receives command
  ↓
CYNIC judges (11 Dogs + PBFT)
  ↓
Proposal created on NEAR testnet ✅
  ↓
Users vote (YES/NO/ABSTAIN)
  ↓
Votes recorded on NEAR testnet ✅
  ↓
Voting closes
  ↓
Proposal executed on NEAR ✅
  ↓
Community rates satisfaction
  ↓
Q-Table learns ✅
  ↓
Next proposal uses improved confidence ✅
```

---

## Critical Dependencies

- **discord.py** — Already installed
- **near-api-py** — Need to install (for transaction signing)
- **SQLAlchemy** — Already installed
- **pydantic** — Already installed (for config validation)
- **Docker** — Need to install locally

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| NEAR contract won't compile | Use existing contract ABI, or use AssemblyScript (easier) |
| Testnet rate limiting | Implement request throttling |
| Database contention | Use connection pool + async sessions |
| Discord API limits | Implement backoff strategy |
| Transaction signing bugs | Extensive unit tests before live submission |

---

## What Gets Deployed

After Phase 1 & 2, we have:
- ✅ Production-hardened Discord bot
- ✅ Working NEAR integration
- ✅ Actual on-chain governance
- ✅ Error recovery
- ✅ Monitoring/logging
- ❌ Agent members (Phase 4)
- ❌ Multi-community scaling (Phase 3)

**But we can run real governance with humans voting and learning.**

Ready to start Phase 1 Task 1.1?
