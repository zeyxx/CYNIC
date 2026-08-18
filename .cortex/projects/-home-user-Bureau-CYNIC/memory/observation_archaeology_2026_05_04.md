---
name: Observation Archaeology & Causality Discovery
description: Extracted 1000+ observations from kernel; discovered schema is dead template. Next task is extract organism causality (behavior_log → verdict → crystal) to discover blockchain/consensus topology from data.
type: project
---

# Observation Archaeology — Session 2026-05-04

## Critical Finding

**Observation schema is a structural ghost.**

### What Exists
- 21 fields, 100% present, uniformly structured
- Structural fields filled: id, tool, target, domain, created_at, agent_id, session_id, tags, status
- Temporal metadata present: created_at

### What's Missing (All Null)
- **value**: signal value or verdict (null in 100%)
- **confidence**: quality score (null in 100%)
- **consumer**: which system should read this (null in 100%)
- **action**: what consumer should do (null in 100%)
- **context**: enrichment metadata (empty string in 100%)

### Signal Types Discovered
1. **operation** (470): Bash, Edit, Write tools — structural logs
2. **unknown** (420): verdict tool — no semantic enrichment
3. **observation** (70): twitter domain operations
4. **session** (40): user_prompt tool — session metadata

### Root Cause (Deduced)

Observation system is a **centralized log without K15 enforcement**. Producers POST blindly. Consumers never read. Schema exists but holds no learning signal.

## Next Task: Extract Organism Causality

**Direction**: CHAOS→MATRIX data-centric discovery.

Don't design blockchain topology. Extract what's already flowing:

1. **Trace real causality chains**
   - behavior_log click → capture → enrichment → verdict → crystal → decision
   - 44K events, 681 domain routes
   - Map actual information flow

2. **Measure consensus gaps**
   - Where do Dogs disagree on verdict?
   - Where does verdict not reach crystal?
   - Where does routing fail?
   - **Those = where proofs needed**

3. **Identify immutability points**
   - What breaks if enrichment forged?
   - What breaks if verdict hash altered?
   - What breaks if causality rewritten?
   - **Those = blockchain primitives**

4. **Discover consensus topology**
   - count(verdicts where dog_A == dog_B) / total per pair
   - Find natural validator network
   - **That's the actual consensus layer**

5. **Extract hypergraph edges**
   - For each crystal: what observations fed it?
   - For each decision: what crystals enabled it?
   - **Those relationships = edges**

## Scripts to Write

**extract_organism_causality.py**:
- Input: behavior_log (44K clicks), killchain.jsonl, verdicts, crystals
- Output:
  - causality_dag.json: click → capture → enrichment → verdict → crystal → decision
  - consensus_gaps.json: where dogs disagree, where routing fails
  - blockchain_primitives.json: immutability requirements, proof points, validator sets
  - hypergraph_edges.json: observation → crystal → decision chains

## Falsifiable Test

If emergent causality chain matches blockchain structure (immutable proof → consensus points → routing decisions), topology is real. If it doesn't, the architecture needs adjustment.

## K15 Follow-up (Not Phase 2, Phase 3+)

Once causality topology is discovered:
1. **K15 validator**: Reject POST /observe if value/confidence/consumer null
2. **Ledger per cortex**: Each agent maintains local blockchain of claims
3. **Consensus protocol**: Agreement rules for distributed verdicts
4. **Github integration**: Map commits → transactions, PRs → consensus requests

## Blocked Dependencies

- Kernel storage degraded (but queryable)
- Causality extraction requires live kernel data (available)

## Why This Matters

Current observation system is pre-hypergraph, pre-blockchain. It's a centralized log. The next architecture needs:
- **Distributed validation** (not kernel-centric)
- **Proof of work** (not just logs)
- **Causality tracing** (not scattered domains)
- **Consensus topology** (not null consumers)

The organism is already generating the right data. Archaeology extracts what it needs.
