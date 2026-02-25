# CYNIC Governance Bot — Complete Schema Design

**Date:** 2026-02-25
**Status:** Design Phase — Ready for Implementation
**Target:** Memecoin Community Governance

---

## 1. Overview

The governance bot enables memecoin communities to:
1. **Submit proposals** — Community members suggest decisions
2. **Get CYNIC judgment** — 11 Dogs evaluate proposal quality
3. **Vote on proposals** — Community votes informed by CYNIC
4. **Execute decisions** — Approved proposals run on-chain (NEAR)
5. **Learn from outcomes** — CYNIC improves future judgments

**Core Principle:** Fair, learning-based governance powered by CYNIC's 11 Dogs consensus.

---

## 2. Data Models

### 2.1 Community Configuration

```json
{
  "community_id": "solana-doge-dao-001",
  "platform": "discord" | "telegram",
  "community_name": "Solana Doge DAO",
  "community_token": "SDOGE",
  "created_at": 1740447600,

  "governance_settings": {
    "voting_period_hours": 72,
    "execution_delay_hours": 24,
    "quorum_percentage": 25,
    "approval_threshold_percentage": 50,
    "proposal_submission_fee_tokens": 100,
    "voting_method": "token_weighted" | "one_person_one_vote",
    "allow_amendments": true,
    "amendment_vote_threshold": 66
  },

  "integration_settings": {
    "gasdf_enabled": true,
    "near_contract_address": "governance.near",
    "treasury_address": "treasury.near",
    "fee_burn_percentage": 100,
    "learning_enabled": true
  },

  "cynic_settings": {
    "judgment_required": true,
    "judgment_verdict_influences_voting": true,
    "min_dogs_consensus": 6,
    "q_score_threshold_for_alert": 30.0,
    "reputation_score": 0.618,
    "learning_weight": 1.0
  },

  "moderation": {
    "proposal_moderation_required": false,
    "prohibited_categories": ["illegal", "offensive"],
    "max_proposal_length_chars": 5000,
    "require_proposer_age_days": 7
  }
}
```

### 2.2 Proposal Model

```json
{
  "proposal_id": "prop_20260225_001_increase_treasury",
  "community_id": "solana-doge-dao-001",
  "proposer_id": "discord_user_12345",
  "proposer_reputation": 0.75,

  "proposal_metadata": {
    "title": "Increase marketing budget for Q1 2026",
    "description": "We should allocate additional 50k SDOGE tokens for marketing campaigns to increase community awareness",

    "category": "BUDGET_ALLOCATION" | "GOVERNANCE_CHANGE" | "PARTNERSHIP" | "COMMUNITY_DECISION" | "EMERGENCY",
    "impact_level": "LOW" | "MEDIUM" | "HIGH" | "CRITICAL",
    "urgency": "ROUTINE" | "IMPORTANT" | "URGENT" | "CRITICAL",

    "tags": ["marketing", "budget", "q1-2026"],
    "related_proposals": ["prop_20260101_001"],

    "created_at": 1740447600,
    "submission_fee_paid": 100,
    "submission_fee_token": "SDOGE"
  },

  "proposal_details": {
    "summary": "Marketing expansion for Q1",
    "background": "Current marketing spend is insufficient to compete with competitors",
    "proposed_action": "Vote to approve 50k SDOGE token allocation for marketing",

    "success_metrics": [
      "20% increase in social media followers",
      "15% increase in trading volume"
    ],
    "timeline": "Q1 2026 (3 months)",
    "budget_breakdown": {
      "social_media_campaigns": 20000,
      "influencer_partnerships": 15000,
      "content_creation": 10000,
      "events": 5000
    },

    "risk_assessment": "Low risk - funds come from treasury surplus",
    "alternative_proposals": []
  },

  "cynic_judgment": {
    "judgment_id": "jud_20260225_001_001",
    "requested_at": 1740447605,
    "received_at": 1740447620,

    "verdict": "WAG" | "HOWL" | "GROWL" | "BARK" | "PENDING",
    "q_score": 72.5,
    "confidence": 0.618,

    "dogs_voting": {
      "ANALYST": {"vote": "WAG", "reasoning": "Budget well-justified"},
      "ARCHITECT": {"vote": "HOWL", "reasoning": "Aligns with long-term strategy"},
      "CARTOGRAPHER": {"vote": "WAG", "reasoning": "Medium implementation risk"},
      "CYNIC": {"vote": "HOWL", "reasoning": "Addresses competitive gap"},
      "DEPLOYER": {"vote": "GROWL", "reasoning": "Tight execution timeline"},
      "GUARDIAN": {"vote": "WAG", "reasoning": "Treasury has sufficient funds"},
      "JANITOR": {"vote": "WAG", "reasoning": "Clean fund allocation"},
      "ORACLE": {"vote": "HOWL", "reasoning": "Market timing favorable"},
      "SAGE": {"vote": "HOWL", "reasoning": "Community needs marketing"},
      "SCHOLAR": {"vote": "WAG", "reasoning": "Data supports decision"},
      "SCOUT": {"vote": "WAG", "reasoning": "Competitors doing same"}
    },

    "consensus": {
      "howl_count": 4,
      "wag_count": 6,
      "growl_count": 1,
      "bark_count": 0,
      "recommendation": "FAVORABLE - Most dogs lean toward approval"
    },

    "detailed_reasoning": "11 Dogs consensus: This proposal is well-structured with clear budget breakdown and success metrics. Most concerns are about execution timeline, but risk is mitigated by treasury reserves.",

    "similar_proposals": ["prop_20260101_001", "prop_20251225_003"],
    "estimated_outcome": "75% likelihood of success if executed properly"
  },

  "voting": {
    "voting_start_time": 1740447620,
    "voting_end_time": 1740704820,
    "voting_status": "ACTIVE" | "CLOSED" | "CANCELLED",
    "voting_period_hours": 72,

    "votes": {
      "yes_count": 2500000,
      "no_count": 500000,
      "abstain_count": 100000,
      "total_votes": 3100000,
      "total_eligible": 5000000,
      "participation_rate": 0.62
    },

    "approval_status": "APPROVED" | "REJECTED" | "TIED" | "PENDING",
    "approval_threshold_met": true,
    "quorum_met": true,

    "voting_breakdown": {
      "by_token_holder_tier": {
        "whale": {"yes": 1500000, "no": 200000, "participation": 0.85},
        "large": {"yes": 700000, "no": 200000, "participation": 0.70},
        "medium": {"yes": 250000, "no": 80000, "participation": 0.55},
        "small": {"yes": 50000, "no": 20000, "participation": 0.35}
      }
    }
  },

  "execution": {
    "execution_status": "PENDING" | "SCHEDULED" | "EXECUTING" | "COMPLETED" | "FAILED",
    "execution_date": 1740708420,
    "execution_delay_hours": 24,

    "on_chain_data": {
      "near_tx_hash": "8Ug7qTB3z5k2mL9pN1vQ4w6xY8aZ9bC0dE1fG2hI3jK4lM5nO6pP",
      "near_block_height": 245000000,
      "treasury_transfer_amount": 50000,
      "treasury_transfer_token": "SDOGE",
      "execution_confirmed": true,
      "confirmation_time": 1740708600
    },

    "gasdf_integration": {
      "fee_collected": 100,
      "fee_token": "SDOGE",
      "fee_burned": true,
      "burn_timestamp": 1740708610,
      "burn_tx_hash": "9Vh8rUC4aL6mK0pQ5xR3wS2vT9bU8cV1dW2eX3fY4gZ5hA6iB7jC8k"
    }
  },

  "learning": {
    "learning_status": "PENDING" | "ACTIVE" | "COMPLETED",
    "outcome_determined": false,
    "outcome": null,
    "outcome_date": null,
    "outcome_description": null,

    "success_metrics_achieved": null,
    "community_satisfaction_rating": null,
    "cynic_rating_from_community": null,

    "learning_data": {
      "q_table_update_pending": true,
      "q_score_delta": null,
      "prediction_accuracy": null,
      "dogs_prediction_accuracy": {}
    }
  }
}
```

### 2.3 Vote Model

```json
{
  "vote_id": "vote_20260225_001_user_12345",
  "proposal_id": "prop_20260225_001_increase_treasury",
  "community_id": "solana-doge-dao-001",

  "voter_id": "discord_user_12345",
  "voter_reputation": 0.72,
  "voter_token_balance": 10000,
  "voter_tier": "medium",

  "vote": "YES" | "NO" | "ABSTAIN",
  "vote_weight": 10000,

  "reasoning": "I support this because we need more marketing visibility",
  "voted_at": 1740450000,

  "vote_influenced_by_cynic": true,
  "cynic_verdict_at_vote_time": "WAG",
  "cynic_q_score_at_vote_time": 72.5,

  "amendable": true,
  "amendment_history": [
    {
      "amended_from": "ABSTAIN",
      "amended_to": "YES",
      "amended_at": 1740500000,
      "reason": "Changed mind after seeing execution plan"
    }
  ]
}
```

### 2.4 E-Score (Reputation) Model

```json
{
  "e_score_id": "escore_20260225_cynic_001",
  "community_id": "solana-doge-dao-001",
  "entity_type": "CYNIC" | "PROPOSER" | "VOTER",
  "entity_id": "cynic_organism_001" | "proposer_user_id" | "voter_user_id",

  "base_score": 0.618,

  "score_components": {
    "judgment_accuracy": 0.85,
    "prediction_success_rate": 0.78,
    "community_satisfaction": 0.72,
    "proposal_quality": 0.75,
    "execution_reliability": 0.80,
    "learning_improvement": 0.68
  },

  "historical_data": {
    "total_judgments": 42,
    "correct_predictions": 33,
    "successful_proposals": 28,
    "failed_proposals": 4,
    "community_votes_influenced": 156,
    "positive_feedback_count": 38,
    "negative_feedback_count": 2
  },

  "cross_community_score": {
    "communities_involved": 3,
    "average_score_across_communities": 0.72,
    "trending": "UP",
    "trend_change_percentage": 5.2
  },

  "updated_at": 1740450000,
  "update_source": "PROPOSAL_OUTCOME" | "COMMUNITY_FEEDBACK" | "LEARNING_LOOP",
  "last_updated_proposal": "prop_20260225_001"
}
```

---

## 3. Discord Bot Commands

### 3.1 Proposal Management

```
/propose <title> <description> <category> <impact_level>
  Creates a new proposal
  - title: Proposal title (max 200 chars)
  - description: Proposal description (max 5000 chars)
  - category: BUDGET_ALLOCATION | GOVERNANCE_CHANGE | PARTNERSHIP | etc.
  - impact_level: LOW | MEDIUM | HIGH | CRITICAL
  - Requires: 100 token submission fee
  - Returns: proposal_id, CYNIC judgment

/proposal-details <proposal_id>
  View full proposal with CYNIC judgment
  - Shows: Title, description, CYNIC verdict, voting status
  - Shows: Each Dog's vote and reasoning
  - Shows: Current vote count and approval probability

/proposals <filter>
  List proposals
  - filter: ACTIVE | PASSED | FAILED | PENDING_EXECUTION | ALL
  - Shows: Proposal ID, title, CYNIC verdict, voting status

/proposal-status <proposal_id>
  Quick status check
  - Shows: Voting period remaining, current vote count, trend

/amend-proposal <proposal_id> <new_description>
  Proposer can amend during voting period
  - Requires: Proposer is original author
  - Shows: Amendment history

/withdraw-proposal <proposal_id>
  Proposer can withdraw before voting closes
  - Requires: Proposer is original author
  - Shows: Withdrawal confirmed
```

### 3.2 Voting Commands

```
/vote <proposal_id> <yes|no|abstain> [reasoning]
  Cast or update your vote
  - proposal_id: ID of proposal to vote on
  - vote: YES | NO | ABSTAIN
  - reasoning: Optional explanation (max 500 chars)
  - Shows: Your vote recorded, CYNIC verdict for reference

/voting-status <proposal_id>
  See voting progress
  - Shows: Yes/No/Abstain counts
  - Shows: Participation rate vs. quorum
  - Shows: Time remaining
  - Shows: Approval probability
  - Shows: Voting breakdown by tier

/my-votes
  See your voting history
  - Shows: Proposals you voted on
  - Shows: Your votes and timing
  - Shows: How votes aligned with outcomes

/upcoming-votes
  See proposals coming to vote soon
  - Shows: CYNIC judgments
  - Shows: Community discussion summary
```

### 3.3 CYNIC Judgment Commands

```
/cynic-verdict <proposal_id>
  Get CYNIC's detailed judgment
  - Shows: HOWL/WAG/GROWL/BARK verdict
  - Shows: Q-Score and confidence
  - Shows: Each Dog's reasoning
  - Shows: Similar past proposals
  - Shows: Estimated success probability

/cynic-explain <proposal_id> <dog_name>
  Get specific Dog's reasoning
  - dog_name: ANALYST | ARCHITECT | CARTOGRAPHER | CYNIC | etc.
  - Shows: That Dog's specific vote and detailed reasoning

/cynic-status
  Check CYNIC's health
  - Shows: Reputation score (E-Score) in this community
  - Shows: Judgment accuracy rate
  - Shows: Prediction success rate
  - Shows: Community satisfaction
  - Shows: Status across all communities

/similar-proposals <proposal_id>
  Find similar past proposals
  - Shows: Past proposals with similar topics
  - Shows: How they were voted
  - Shows: Outcomes and results
```

### 3.4 Community & Treasury Commands

```
/community-info
  View governance settings
  - Shows: Voting period, quorum, approval threshold
  - Shows: CYNIC integration status
  - Shows: GASdf/NEAR integration status

/treasury-status
  View community treasury
  - Shows: Token balance
  - Shows: Funds allocated to active proposals
  - Shows: Recent transactions

/governance-stats
  Community governance metrics
  - Shows: Total proposals submitted
  - Shows: Approval rate
  - Shows: Average voting participation
  - Shows: CYNIC judgment accuracy in this community
  - Shows: E-Score trend (improving/declining)

/leaderboard <type>
  Reputation leaderboard
  - type: PROPOSERS | VOTERS | COMMUNITIES
  - Shows: Top 10 by E-Score
  - Shows: Their stats and recent activity

/feedback <proposal_id> <rating> [comment]
  Rate a proposal outcome (after execution)
  - proposal_id: Completed proposal
  - rating: 1-5 stars
  - comment: Optional feedback (max 200 chars)
  - Shows: Feedback recorded, helps CYNIC learn
```

### 3.5 Learning & Analytics Commands

```
/learning-status <proposal_id>
  Track CYNIC's learning from proposal
  - Shows: Outcome determined (yes/no)
  - Shows: Success metrics vs. predictions
  - Shows: CYNIC accuracy score for this proposal
  - Shows: Q-Score adjustment pending

/community-learning
  View learning progress in community
  - Shows: Proposals used for learning: N
  - Shows: CYNIC accuracy trend
  - Shows: Most improved Dog (learning best)
  - Shows: E-Score trend (improving/declining)

/prediction-accuracy <dog_name>
  How accurate is a specific Dog's predictions?
  - Shows: Historical accuracy rate
  - Shows: Recent performance
  - Shows: Best/worst prediction types

/dogs-consensus-quality
  How well do 11 Dogs agree?
  - Shows: Average consensus rate
  - Shows: Dogs who most often agree
  - Shows: Dogs with diverse perspectives
```

### 3.6 Configuration Commands (Admin Only)

```
/governance-config
  View/edit governance settings
  - Voting period, quorum, thresholds
  - Token submission fees
  - Voting method (token-weighted vs. 1-person-1-vote)

/cynic-config
  Configure CYNIC integration
  - Judgment required: yes/no
  - Min dogs consensus needed
  - Q-Score thresholds for alerts

/integrate-gasdf <treasury_address>
  Enable GASdf fee burning
  - Configures: Treasury address
  - Configures: Fee burn percentage
  - Verifies: Integration working

/integrate-near <contract_address>
  Enable NEAR on-chain execution
  - Configures: Smart contract address
  - Configures: Execution permissions
  - Verifies: Contract accessible
```

---

## 4. Telegram Bot Commands

Same as Discord but formatted for Telegram:

```
/propose
  Sends form: Title? Description? Category? Impact?

/vote <proposal_id>
  Sends inline keyboard: YES | NO | ABSTAIN

/verdict <proposal_id>
  CYNIC verdict in thread

Similar structure, adapted for Telegram's UX patterns
```

---

## 5. Bot Response Formats

### 5.1 Proposal Created Response

```
PROPOSAL SUBMITTED ✅

ID: prop_20260225_001_increase_treasury
Title: Increase marketing budget for Q1 2026

CYNIC JUDGMENT 🧠:
Verdict: WAG (Lean toward approval)
Q-Score: 72.5/100
Confidence: 61.8% (φ-bounded)

DOGS VOTE:
🐕 HOWL:  4 Dogs (ARCHITECT, CYNIC, ORACLE, SAGE)
🐕 WAG:   6 Dogs (ANALYST, CARTOGRAPHER, GUARDIAN, JANITOR, SCHOLAR, SCOUT)
🐕 GROWL: 1 Dog (DEPLOYER)
🐕 BARK:  0 Dogs

Consensus: FAVORABLE - Most dogs lean toward approval

KEY INSIGHT:
Well-structured proposal with clear metrics. Execution timeline is the main concern, but treasury reserves mitigate risk.

Voting Opens: in 1 minute
Voting Period: 72 hours
Approval Threshold: 50% + 1
Quorum Required: 25%

👉 /vote prop_20260225_001 YES to support
👉 /proposal-details prop_20260225_001 for full details
```

### 5.2 CYNIC Verdict Response

```
CYNIC JUDGMENT 🧠

Proposal: Increase marketing budget for Q1 2026
Verdict: WAG (Lean Toward Approval)
Q-Score: 72.5/100
Confidence: 61.8%

CONSENSUS:
✅ 4 HOWL  (Recommend)
✅ 6 WAG   (Lean toward)
⚠️ 1 GROWL (Caution)
❌ 0 BARK  (Reject)

DOGS REASONING:

🔍 ANALYST:   "Budget well-justified" → WAG
🏗️ ARCHITECT:  "Aligns with long-term strategy" → HOWL
🗺️ CARTOGRAPHER: "Medium implementation risk" → WAG
😈 CYNIC:      "Addresses competitive gap" → HOWL
🚀 DEPLOYER:   "Tight execution timeline" → GROWL
🛡️ GUARDIAN:   "Treasury has sufficient funds" → WAG
🧹 JANITOR:    "Clean fund allocation" → WAG
🔮 ORACLE:     "Market timing favorable" → HOWL
🧙 SAGE:       "Community needs marketing" → HOWL
📚 SCHOLAR:    "Data supports decision" → WAG
🥾 SCOUT:      "Competitors doing same" → WAG

SUMMARY:
This proposal is well-structured with clear budget breakdown and success metrics. The primary concern from DEPLOYER is about execution timeline, but this risk is mitigated by sufficient treasury reserves.

SIMILARITY MATCH:
Similar to: prop_20260101_001 (Marketing spend - APPROVED)
Predicted Success Rate: 75% if executed properly

Confidence: 61.8% (φ-bounded)
```

### 5.3 Voting Status Response

```
VOTING STATUS 📊

Proposal: Increase marketing budget for Q1 2026
Status: ACTIVE (closes in 24 hours)

CURRENT VOTES:
YES:     2,500,000 tokens (80.6%)
NO:        500,000 tokens (16.1%)
ABSTAIN:   100,000 tokens (3.2%)

Total Votes: 3,100,000 / 5,000,000 tokens
Participation: 62.0%

APPROVAL PROBABILITY: 95% ✅
Quorum Met: YES (25% required, 62% participating)
Approval Threshold Met: YES (50% required, 80.6% voting yes)

VOTING BREAKDOWN BY TIER:
🐋 Whales (>100k tokens):    85% participation, 89% YES
📈 Large (10k-100k):         70% participation, 78% YES
📊 Medium (1k-10k):          55% participation, 76% YES
💰 Small (<1k):              35% participation, 71% YES

TREND: ↗️ YES votes increasing (last 6 hours +3%)

👉 /vote prop_20260225_001 YES to support
```

---

## 6. Database Schema

### Core Tables

```sql
-- Communities
CREATE TABLE communities (
  community_id VARCHAR PRIMARY KEY,
  platform ENUM('discord', 'telegram'),
  community_name VARCHAR,
  community_token VARCHAR,

  -- Governance settings
  voting_period_hours INT,
  execution_delay_hours INT,
  quorum_percentage DECIMAL,
  approval_threshold_percentage DECIMAL,
  proposal_submission_fee_tokens DECIMAL,
  voting_method ENUM('token_weighted', 'one_person_one_vote'),

  -- Integration
  gasdf_enabled BOOLEAN,
  near_contract_address VARCHAR,
  treasury_address VARCHAR,
  fee_burn_percentage INT,

  -- CYNIC
  cynic_enabled BOOLEAN,
  min_dogs_consensus INT,

  created_at TIMESTAMP,
  updated_at TIMESTAMP
);

-- Proposals
CREATE TABLE proposals (
  proposal_id VARCHAR PRIMARY KEY,
  community_id VARCHAR REFERENCES communities,
  proposer_id VARCHAR,

  title VARCHAR(200),
  description TEXT,
  category ENUM('BUDGET', 'GOVERNANCE', 'PARTNERSHIP', 'COMMUNITY', 'EMERGENCY'),
  impact_level ENUM('LOW', 'MEDIUM', 'HIGH', 'CRITICAL'),

  -- Timeline
  created_at TIMESTAMP,
  voting_start_time TIMESTAMP,
  voting_end_time TIMESTAMP,
  execution_date TIMESTAMP,

  -- Status
  voting_status ENUM('PENDING', 'ACTIVE', 'CLOSED', 'CANCELLED'),
  execution_status ENUM('PENDING', 'SCHEDULED', 'EXECUTING', 'COMPLETED', 'FAILED'),
  approval_status ENUM('APPROVED', 'REJECTED', 'TIED', 'PENDING'),

  -- CYNIC judgment FK
  judgment_id VARCHAR REFERENCES judgments,

  -- On-chain
  near_tx_hash VARCHAR,

  updated_at TIMESTAMP
);

-- Votes
CREATE TABLE votes (
  vote_id VARCHAR PRIMARY KEY,
  proposal_id VARCHAR REFERENCES proposals,
  voter_id VARCHAR,
  vote ENUM('YES', 'NO', 'ABSTAIN'),
  vote_weight DECIMAL,
  voted_at TIMESTAMP,

  UNIQUE(proposal_id, voter_id)
);

-- CYNIC Judgments
CREATE TABLE judgments (
  judgment_id VARCHAR PRIMARY KEY,
  proposal_id VARCHAR REFERENCES proposals,

  verdict ENUM('HOWL', 'WAG', 'GROWL', 'BARK', 'PENDING'),
  q_score DECIMAL,
  confidence DECIMAL,

  dogs_voting JSON, -- JSON object with each Dog's vote
  consensus_summary JSON,

  requested_at TIMESTAMP,
  received_at TIMESTAMP
);

-- E-Scores
CREATE TABLE e_scores (
  e_score_id VARCHAR PRIMARY KEY,
  community_id VARCHAR REFERENCES communities,
  entity_type ENUM('CYNIC', 'PROPOSER', 'VOTER'),
  entity_id VARCHAR,

  base_score DECIMAL,
  score_components JSON,
  historical_data JSON,

  updated_at TIMESTAMP,
  update_source ENUM('PROPOSAL_OUTCOME', 'FEEDBACK', 'LEARNING')
);

-- Learning Outcomes
CREATE TABLE learning_outcomes (
  outcome_id VARCHAR PRIMARY KEY,
  proposal_id VARCHAR REFERENCES proposals,

  outcome VARCHAR,
  success_metrics_achieved JSON,
  community_satisfaction_rating DECIMAL,
  cynic_rating_from_community INT,

  recorded_at TIMESTAMP
);
```

---

## 7. Integration Points

### 7.1 CYNIC MCP Integration

```python
# When proposal created
async def create_proposal(proposal_data):
    # 1. Create proposal in database
    proposal = save_proposal(proposal_data)

    # 2. Request CYNIC judgment
    judgment = await call_mcp_tool('ask_cynic', {
        'question': proposal['title'],
        'context': proposal['description'],
        'reality': 'GOVERNANCE'
    })

    # 3. Store judgment
    judgment_id = save_judgment(proposal['id'], judgment)
    update_proposal(proposal['id'], judgment_id=judgment_id)

    # 4. Post to Discord
    await post_proposal_to_discord(proposal, judgment)
```

### 7.2 GASdf Integration

```python
# When proposal is approved and executed
async def execute_proposal(proposal_id):
    proposal = get_proposal(proposal_id)

    # 1. Collect submission fee (already collected at proposal time)
    fee = proposal['submission_fee_paid']

    # 2. Trigger GASdf fee burn
    await gasdf.burn_tokens(
        amount=fee,
        token=proposal['submission_fee_token'],
        burn_address=get_treasury('burn')
    )

    # 3. Execute on NEAR
    await near_contract.execute_proposal(proposal_id)
```

### 7.3 NEAR Integration

```python
# Smart contract interface
interface GovernanceContract {
    execute_proposal(proposal_id, treasury_amount, recipients) -> tx_hash
    record_outcome(proposal_id, success, metrics) -> outcome_id
    get_proposal_status(proposal_id) -> status
}

# Bot call
async def execute_on_chain(proposal_id):
    near_tx = await near_contract.execute_proposal(
        proposal_id=proposal_id,
        treasury_amount=proposal['budget'],
        recipients=proposal['recipients']
    )

    # Store tx hash
    update_proposal(
        proposal_id,
        near_tx_hash=near_tx.hash,
        execution_status='EXECUTING'
    )
```

### 7.4 Learning Loop Integration

```python
# When proposal outcome is determined
async def record_outcome(proposal_id, success, metrics):
    proposal = get_proposal(proposal_id)
    judgment = get_judgment(proposal['judgment_id'])

    # 1. Record outcome
    outcome = save_outcome(proposal_id, success, metrics)

    # 2. Feed to CYNIC learning
    await call_mcp_tool('learn_cynic', {
        'judgment_id': judgment['id'],
        'outcome': success,
        'actual_metrics': metrics,
        'predicted_metrics': judgment['predicted_metrics'],
        'feedback_rating': 4  # 1-5 stars
    })

    # 3. Update E-Score
    update_e_score(judgment['cynic_id'], outcome)

    # 4. Announce to community
    await post_outcome_to_discord(proposal, outcome)
```

---

## 8. User Flows

### 8.1 Proposal Submission Flow

```
User → /propose title description → Bot validates inputs
                                  → User pays 100 token fee
                                  → Bot creates proposal in DB
                                  → Bot calls CYNIC /ask_cynic
                                  → CYNIC returns judgment
                                  → Bot stores judgment
                                  → Bot posts to Discord
                                  → Community sees: Proposal + CYNIC verdict
```

### 8.2 Voting Flow

```
Community member → sees proposal in Discord
                 → reads CYNIC verdict
                 → /vote prop_id YES (with optional reasoning)
                 → Bot records vote with vote_weight
                 → Bot updates vote count display
                 → If approval threshold met → mark APPROVED
                 → If voting period ends → close voting
                 → If APPROVED → schedule execution
```

### 8.3 Execution & Learning Flow

```
Proposal APPROVED → 24 hour delay
                  → NEAR contract executes
                  → GASdf burns fee
                  → Proposal executes (e.g., fund transferred)
                  → Community verifies outcome
                  → /feedback prop_id 4 "Great decision"
                  → Bot calls learn_cynic
                  → CYNIC updates Q-Table
                  → E-Score recalculated
                  → Community sees E-Score improvement
```

---

## 9. Example Scenarios

### Scenario 1: Marketing Budget Proposal

```
Step 1: Proposer submits "Increase marketing budget for Q1"
        Bot charges 100 SDOGE fee
        CYNIC gives judgment: WAG, Q-Score 72.5

Step 2: Community votes (72 hours)
        Result: 80% YES, 20% NO
        Approval: PASSED

Step 3: 24 hour execution delay

Step 4: NEAR contract executes
        GASdf burns 100 SDOGE to community treasury
        50,000 SDOGE transferred to marketing wallet

Step 5: 3 months later - outcome determined
        Social media followers: +22% (target 20%) ✅
        Trading volume: +16% (target 15%) ✅
        Community satisfaction: 4.5/5 stars

Step 6: Community rates outcome: 5 stars
        Bot calls learn_cynic with success data
        CYNIC updates Q-Table
        CYNIC's E-Score: 0.618 → 0.642 (+3.9%)

Result: CYNIC learns marketing proposals work well
        Dogs improve Q-Scores on similar proposals
        Next marketing proposal gets better judgment
```

### Scenario 2: Governance Change (Controversial)

```
Step 1: Proposer: "Change voting threshold to 40%"
        CYNIC gives: GROWL, Q-Score 35.2
        Dogs mostly worried about low quorum

Step 2: Community votes
        Result: 45% YES, 55% NO
        Approval: REJECTED

Step 3: Community feedback shows:
        - Whales wanted 40% threshold
        - Small holders worried about exclusion
        - Fairness concern

Step 4: Proposer learns from CYNIC reasoning
        Submits modified: "Change to 45% with quorum boost"
        CYNIC gives: WAG, Q-Score 68.1

Step 5: Second vote passes (62% YES)

Result: CYNIC prevented extractive governance change
        Community learned from CYNIC reasoning
        Modified proposal found consensus
```

---

## 10. Implementation Roadmap

### Week 1: Core Schema & Bot
- [ ] Implement database schema
- [ ] Build Discord bot commands (propose, vote, verdict)
- [ ] Connect to CYNIC MCP (ask_cynic tool)
- [ ] Test with single community

### Week 2: Voting & Execution
- [ ] Implement voting logic
- [ ] Build NEAR smart contract
- [ ] Implement GASdf integration
- [ ] Test proposal execution flow

### Week 3: Learning & E-Score
- [ ] Implement outcome tracking
- [ ] Connect to learn_cynic tool
- [ ] Implement E-Score calculation
- [ ] Test learning loop

### Week 4: Scale & Polish
- [ ] Multi-community support
- [ ] Telegram bot variant
- [ ] Admin configuration tools
- [ ] Analytics & leaderboards
- [ ] Deploy to 3-5 pilot communities

---

## 11. Success Metrics

### Phase 1 (Week 1-2)
- ✅ Bot deployed and responding to commands
- ✅ Proposals created and CYNIC judging them
- ✅ Community voting on proposals

### Phase 2 (Week 3-4)
- ✅ Proposals executing on-chain
- ✅ GASdf burning fees (deflationary proof)
- ✅ CYNIC learning from outcomes
- ✅ E-Score improving (learning working)

### Phase 3 (Month 2)
- ✅ 3-5 communities using governance
- ✅ 50+ proposals submitted total
- ✅ CYNIC accuracy improving over time
- ✅ E-Score trending UP across communities

---

## 12. Notes & Decisions

**Decision 1: Token-Weighted Voting**
- Default voting method but configurable per community
- Whales (>100k) get more say initially
- Can be changed to "1 person 1 vote" if community prefers

**Decision 2: 11 Dogs Consensus**
- Requires 6/11 Dogs agreement for HOWL or BARK
- 4+ WAG is favorable (lean toward approval)
- Prevents single Dog from dominating

**Decision 3: Fee Burning**
- 100% of proposal fees burn to community treasury
- Makes voting extraction impossible
- Incentivizes thoughtful proposals

**Decision 4: Learning Loop**
- Outcomes tracked 30 days post-execution
- Community rates success (1-5 stars)
- CYNIC learns and updates Q-Table
- E-Score reflects learning progress

**Decision 5: Cross-Community E-Score**
- CYNIC's E-Score improves across all communities
- Network effects: Each community's learning helps others
- Incentivizes CYNIC to be fair and accurate

---

**Status:** Schema design complete and ready for implementation.

**Next Step:** Implement database schema and build Discord bot core (Week 1).

