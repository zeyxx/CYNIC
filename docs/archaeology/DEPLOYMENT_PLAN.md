# CYNIC Governance MVP Deployment Plan
**Target:** First memecoin community (testnet pilot)
**Timeline:** Week 1 execution
**Status:** Planning phase

## Pre-Deployment Checklist

### Code Readiness
- [x] Governance bot implementation (governance_bot/)
- [x] NEAR integration (cynic/integrations/near/)
- [x] Unified orchestrator (cynic/organism/orchestrator.py)
- [x] 11 Dogs judges (cynic/judges/dog_implementations.py)
- [x] Q-Table learning (cynic/learning/unified_learning.py)
- [x] 289 tests passing (100%)
- [x] End-to-end wiring verified

### Infrastructure Needed
- [ ] NEAR testnet account setup (cynic.testnet, contract deployment)
- [ ] Discord server for pilot community
- [ ] Memecoin community contact/partner
- [ ] Database setup (governance_bot/database.py)
- [ ] Environment configuration (.env setup)
- [ ] Monitoring/logging system

### Community Requirements
- [ ] Memecoin identified (who wants to pilot?)
- [ ] Community size (minimum viable group)
- [ ] Governance culture fit (fair governance interest)
- [ ] Technical readiness (can they use Discord bot?)

## Deployment Phases

### Phase 1: Infrastructure Setup
- [ ] Deploy NEAR testnet account
- [ ] Configure governance contract on testnet
- [ ] Setup Discord test server
- [ ] Initialize governance_bot database
- [ ] Configure environment variables

### Phase 2: Bot Integration
- [ ] Start governance_bot locally
- [ ] Connect to Discord test server
- [ ] Test proposal submission
- [ ] Test CYNIC judgment flow
- [ ] Test vote recording
- [ ] Test NEAR execution

### Phase 3: Pilot Community Launch
- [ ] Contact memecoin community
- [ ] Invite to Discord governance server
- [ ] Run training session (how to use bot)
- [ ] Submit first governance proposal
- [ ] Monitor governance round

### Phase 4: Monitoring & Learning
- [ ] Track proposal outcomes
- [ ] Monitor Q-Table updates
- [ ] Document community feedback
- [ ] Measure learning improvement
- [ ] Collect metrics for Phase 2 scaling

## Questions to Answer

1. **Which memecoin community to target?**
   - Need community interested in fair governance
   - Ideally 50-500 members (critical mass for voting)
   - Active discord/telegram

2. **NEAR network choice?**
   - Testnet (recommended for MVP) = free to experiment
   - Mainnet (if we want real stakes) = requires real tokens

3. **Database setup?**
   - SQLite (simple, local)
   - Cloud Postgres (scalable)

4. **Monitoring approach?**
   - Simple logging to file
   - Grafana dashboard
   - Discord notifications of governance events

5. **Success metrics for pilot?**
   - N proposals submitted
   - N votes recorded
   - Q-Table improves 10%+ on repeated verdicts
   - Community satisfaction >4/5 stars
