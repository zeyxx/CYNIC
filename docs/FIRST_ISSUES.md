# First Issues for New Contributors

These are the first 5 issues to work on. Start with Issue #1 and progress sequentially.

---

## Issue #1: Explain the 11 Dogs Architecture

**Difficulty:** Easy
**Time:** 1-2 hours
**Hardware:** Mobile ✅
**Label:** `good-first-issue`, `documentation`

### Task

Read `cynic/cognition/neurons/` or the Dogs implementations and write a 200-300 word explanation:
- What are the 11 Dogs?
- What does each Dog decide/judge?
- How do they work together?

### Files to Read
- `cynic/cognition/neurons/` (Dog implementations)
- `docs/CYNIC_COMPLETE_SYSTEM_MAP.md` (architecture overview)
- Session 5 commits (gossip + federation context)

### Where to Put Your Answer
Create file: `docs/contributor-learning/explain-11-dogs.md`

### Example Structure
```markdown
# The 11 Dogs: CYNIC's Judgment Council

## Overview
[1 paragraph: what they do]

## The Dogs (brief list)
1. Dog1 (Governance) - decides...
2. Dog2 (Risk) - decides...
...

## How They Work Together
[1 paragraph: consensus mechanism]

## Why This Matters
[1 paragraph: impact on governance]
```

### Success Criteria
- ✅ All 11 Dogs mentioned (even if briefly)
- ✅ Clear explanation of each role
- ✅ Explains how they reach consensus
- ✅ No jargon or simple language used
- ✅ 200-300 words (not too long)

### Questions to Answer in Your Explanation
- What domain does each Dog judge? (governance, risk, value, etc)
- How many Dogs vote to reach consensus?
- What happens when Dogs disagree?
- Why 11 Dogs specifically?

---

## Issue #2: Document the Gossip Protocol

**Difficulty:** Easy
**Time:** 1-2 hours
**Hardware:** Mobile ✅
**Label:** `good-first-issue`, `documentation`

### Task

Read the Gossip Protocol implementation and explain how CYNIC instances share learning.

### Files to Read
- `cynic/federation/gossip.py` (code)
- `cynic/federation/merge.py` (merge algorithm)
- Session 5 memory notes

### Write in: `docs/contributor-learning/explain-gossip-protocol.md`

### Example Structure
```markdown
# Gossip Protocol: How CYNIC Instances Learn Together

## The Problem
[Why instances need to share learning]

## The Solution
[How Gossip Protocol works]

## Key Concepts
- k=3 peers
- batch_size=10 judgments
- weighted merging
- φ-bounded confidence

## Example Scenario
[Step by step: instance A judges → pushes to B → B pushes to C]

## Benefits
[Why this is better than alternatives]
```

### Success Criteria
- ✅ Explains what's being shared (Q-Table snapshots)
- ✅ Explains when it's triggered (every 10 judgments)
- ✅ Explains who receives it (k=3 peers)
- ✅ Explains why it helps (collective learning)
- ✅ Clear language, no jargon

### Questions to Answer
- How many peers does each instance connect to?
- What data is shared?
- Why only 10 judgments before sharing?
- What is φ-bounded confidence?

---

## Issue #3: Create Architecture Diagram (Text)

**Difficulty:** Medium
**Time:** 1-2 hours
**Hardware:** Mobile ✅
**Label:** `good-first-issue`, `documentation`

### Task

Create a text-based architecture diagram showing CYNIC's structure.

### Files to Read
- `docs/CYNIC_COMPLETE_SYSTEM_MAP.md`
- Memory: Phase 3 architecture section

### Deliverable

Create file: `docs/contributor-learning/cynic-architecture-diagram.txt`

### Example Format (ASCII art)
```
┌─────────────────────────────────────┐
│    CYNIC Governance Oracle          │
├─────────────────────────────────────┤
│                                     │
│  INPUT: Proposal                    │
│    ↓                                │
│  [11 Dogs Analyze in Parallel]      │
│    ├─ Dog1: Governance              │
│    ├─ Dog2: Risk                    │
│    └─ Dog3-11: Other domains        │
│    ↓                                │
│  [PBFT Byzantine Consensus]         │
│    ↓                                │
│  [Axiom Constraint Check]           │
│    ↓                                │
│  OUTPUT: Verdict (HOWL/WAG/etc)     │
│    ↓                                │
│  [Q-Table Learning + Feedback]      │
│                                     │
└─────────────────────────────────────┘
```

### Success Criteria
- ✅ Shows all 11 Dogs
- ✅ Shows consensus mechanism
- ✅ Shows feedback loop
- ✅ Shows axiom checks
- ✅ Clear and readable

---

## Issue #4: Test Documentation Improvements

**Difficulty:** Medium
**Time:** 1-2 hours
**Hardware:** Mobile ✅
**Label:** `good-first-issue`, `testing`

### Task

Read the federation tests and add clear docstrings explaining what each test validates.

### Files to Read
- `cynic/tests/test_federation.py` (all 9 tests)
- `cynic/federation/merge.py` (merge algorithm)
- `cynic/federation/gossip.py` (gossip manager)

### What to Do

For each test, add or improve docstring:

```python
def test_merge_weighted_by_visits(self):
    """
    Verify that Q-Table merge correctly weights values by visit count.

    When a remote peer has more visits (experience) on a domain,
    its values should dominate in the weighted average.

    Example:
    - Local: 1 visit, q_score=60
    - Remote: 9 visits, q_score=90
    - Weighted: 0.1*60 + 0.9*90 = 87 (remote dominates)

    This ensures older/more experienced peers teach newer ones.
    """
```

### Success Criteria
- ✅ All 9 tests have clear docstrings
- ✅ Each docstring explains WHAT is tested
- ✅ Each docstring explains WHY it matters
- ✅ Example included where helpful
- ✅ No jargon or simple language

---

## Issue #5: Create Contributor Learning Index

**Difficulty:** Medium
**Time:** 1-2 hours
**Hardware:** Mobile ✅
**Label:** `good-first-issue`, `documentation`

### Task

Create a master index document linking all contributor learning materials.

### Create File: `docs/contributor-learning/README.md`

### Contents
```markdown
# Contributor Learning Center

Welcome! Use this guide to learn CYNIC from the ground up.

## Phase 1: Understand the Architecture

- [ ] Read: [The 11 Dogs](./explain-11-dogs.md) - CYNIC's decision council
- [ ] Read: [Gossip Protocol](./explain-gossip-protocol.md) - How instances learn
- [ ] Read: [Architecture Diagram](./cynic-architecture-diagram.txt) - Complete overview

## Phase 2: Understand the Code

- [ ] Read: `cynic/federation/` - Peer-to-peer learning
- [ ] Read: `cynic/cognition/neurons/` - The 11 Dogs
- [ ] Read: `cynic/tests/test_federation.py` - How it's tested

## Phase 3: Your First Contribution

- [ ] Document what you learned (create docs/contributor-learning/your-name.md)
- [ ] Improve test docstrings
- [ ] Create your first PR

## Questions?

Post in GitHub Issues with `help-wanted` label.

## Next Steps

After completing Phase 1-3:
- Join the contributor community
- Get assigned to a coding issue
- Build your portfolio

---

## Learning Paths by Interest

### I want to learn AI/Judgment Systems
Start with: [The 11 Dogs](./explain-11-dogs.md)

### I want to learn Distributed Systems
Start with: [Gossip Protocol](./explain-gossip-protocol.md)

### I want to learn Blockchain
Start with: `docs/CYNIC_COMPLETE_SYSTEM_MAP.md` (NEAR section)

### I want to learn Governance
Start with: `docs/PHASE3_UNIFIED_ARCHITECTURE.md`
```

### Success Criteria
- ✅ Links to all learning materials
- ✅ Clear progression (Phase 1 → 2 → 3)
- ✅ Multiple learning paths by interest
- ✅ Clear next steps

---

## How to Submit These Issues as PRs

After completing any issue:

```bash
git checkout -b docs/contribution-name
# (create your file)
git add docs/contributor-learning/
git commit -m "docs: Add contributor learning - [topic]"
git push origin docs/contribution-name
# (open PR on GitHub)
```

---

## Progression

1. **Issue #1** → Understand Dogs (foundation)
2. **Issue #2** → Understand Gossip (how they learn together)
3. **Issue #3** → See the whole system (diagram)
4. **Issue #4** → Read actual code tests (get technical)
5. **Issue #5** → Help others learn (become a teacher)

After these 5, you're ready for coding issues! 🚀
