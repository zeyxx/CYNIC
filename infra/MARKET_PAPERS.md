# CYNIC Market — Academic References

## Purpose

This document maps academic papers and external research to the CYNIC market
design. Each entry summarizes the paper and explains how it informs specific
design decisions in `MARKET_ARCHITECTURE.md`.

---

## 1. Futarchy & Prediction Markets

### 1.1 Robin Hanson — "Futarchy: Vote Values, But Bet Beliefs" (2009)

**Source**: https://mason.gmu.edu/~rhanson/futarchy.html

**Summary**: Futarchy is a governance system where people vote on values
(what to optimize) but bet on beliefs (what will happen). Prediction markets
are used to select policies that maximize a specified value metric.

**Relevance to CYNIC**:
- Our market is futarchic: actors predict consequences of proposals, and
  the market selects proposals with the best predicted outcomes.
- Hanson's "decision markets" are the theoretical foundation of our
  proposal-prediction-resolution loop.
- Key insight: separate values (operator-defined) from beliefs (market-estimated).

### 1.2 Policy Analysis Market (Hanson 2007)

**Source**: https://ideas.repec.org/a/tpr/inntgg/v2y2007i3p73-88.html

**Summary**: Analysis of a thwarted experiment in using prediction markets for
public policy. Identifies key failure modes: manipulation, low liquidity,
political interference.

**Relevance to CYNIC**:
- Our anti-collusion mechanisms (Q5) address the manipulation risk.
- Our CYD stablecoin addresses the low liquidity problem.
- Our 3-level admission system addresses political/operator oversight.

---

## 2. Proof-of-Useful-Work & LLM Consensus

### 2.1 HadAgent — "Harness-Aware Decentralized Agentic AI Serving with Proof-of-Inference Blockchain Consensus" (2026)

**Source**: https://arxiv.org/html/2604.18614

**Summary**: Replaces hash-based PoW with Proof-of-Inference (PoI). Nodes execute
deterministic inference tasks (fixed weights, input, decoding parameters) and
submit results as cryptographic proofs. Verification requires only a single
forward pass.

**Relevance to CYNIC**:
- **Directly applicable to our PoW-LLM formula** (Section 8.1 of MARKET_ARCHITECTURE.md).
- Instead of just measuring "tasks completed", we can use verifiable inference
  tasks: same input → same output → proof of work.
- Cross-node verification is efficient (single forward pass), enabling our
  jury (PoJ) to verify work without re-executing.
- Key design choice: "harness-aware" — the system knows the execution
  environment (model, weights, temperature). Our capability store already
  tracks this.

### 2.2 Proof of Useful Intelligence (PoUI) — arxiv 2504.17539 (2025)

**Source**: https://arxiv.org/pdf/2504.17539

**Summary**: Replaces wasteful PoW hashing with valuable AI tasks. Proposes a
decentralized marketplace for task distribution and execution.

**Relevance to CYNIC**:
- Our CYD stablecoin (1 CYD ≈ 1 Wh compute) is a form of "useful work" — the
  work produces verifiable outputs, not just hashes.
- The marketplace model aligns with our proposal-prediction market.

### 2.3 ERC-8004 — "Trustless Agents" (2025)

**Source**: https://arxiv.org/html/2511.03434

**Summary**: Ethereum standard for inter-agent trust. Comparative study of
trust models: proof-based, stake-based, reputation-based. Evaluates A2A, AP2,
ERC-8004 on security, privacy, latency, social robustness.

**Relevance to CYNIC**:
- **Validates our multi-proof approach** (PoW, PoJ, PoS, PoSignal, PoCoverage).
- ERC-8004's "proof-based trust" = our PoW + PoJ.
- ERC-8004's "stake-based trust" = our PoS.
- ERC-8004's "reputation-based trust" = our E-Score.
- Key finding: hybrid trust models outperform single-mechanism systems.

---

## 3. Byzantine-Robust Multi-Agent Systems

### 3.1 BlockAgents — "Towards Byzantine-Robust LLM-Based Multi-Agent Coordination via Blockchain" (2025)

**Source**: https://dl.acm.org/doi/fullHtml/10.1145/3674399.3674445

**Summary**: Integrates blockchain into LLM-based multi-agent systems. Proposes
Proof-of-Thought (PoT) consensus + stake-based miner designation + multi-round
debate-style voting. Reduces poisoning attacks to <3%, backdoor attacks to <5%.

**Relevance to CYNIC**:
- **Our PoJ (Proof-of-Jury) should incorporate multi-round debate**, not just
  isolated voting. BlockAgents shows debate-style voting is more robust.
- "Proof-of-Thought" (PoT) — agents must show their reasoning, not just answers.
  Our jury should evaluate rationale quality, not just outcome accuracy.
- Multi-metric evaluation: factual consistency, redundancy, contextual causal
  relevance. Our judge quality metric should include these dimensions.

### 3.2 DecentLLMs — "Byzantine-Robust Decentralized Coordination of LLM Agents" (2025)

**Source**: https://arxiv.org/html/2507.14928v1

**Summary**: Leaderless consensus architecture. Workers generate answers in
parallel, evaluators score them. Uses Geometric Median for Byzantine-robust
aggregation. Tolerates Byzantine agents as long as majority of evaluators
are honest.

**Relevance to CYNIC**:
- **Our jury should be leaderless** (no single judge has veto power).
- Geometric Median is more robust than mean for score aggregation — resistant
  to outlier manipulation.
- Tolerance threshold: f < n/3 Byzantine evaluators. Our Q5 simulation
  confirms this threshold.

### 3.3 Self-Anchored Consensus (SAC) — arxiv 2605.09076 (2025)

**Source**: https://arxiv.org/html/2605.09076

**Summary**: (F+1)-robustness conditions for communication graphs. MSR-inspired
filter-and-refine protocol. Receiver-side evaluation (not sender-reported
confidence) is essential.

**Relevance to CYNIC**:
- **Our jury must use receiver-side evaluation** — actors evaluate each other,
  not self-report confidence. This matches our PoJ design.
- (F+1)-robustness: the communication graph must be designed so that honest
  agents remain connected even after filtering F neighbors.
- Our graph topology (Section 11.1) should satisfy (F+1)-robustness.

### 3.4 CP-WBFT — "Rethinking the Reliability of Multi-Agent System" (2025)

**Source**: https://doi.org/10.48550/arxiv.2511.10400

**Summary**: Confidence probe-based weighted Byzantine Fault Tolerant consensus.
Achieves 85.7% fault tolerance (6 out of 7 nodes malicious) on complete graphs.

**Relevance to CYNIC**:
- Confidence probes (prompt-level and hidden-level) can assess agent reliability.
- Our E-Score dimensions could include "confidence calibration" as a metric.
- Key finding: LLM-based agents are more skeptical of erroneous messages than
  traditional agents — our market benefits from this intrinsic property.

---

## 4. Collusion Detection

### 4.1 Colosseum — "Auditing Collusion in Cooperative Multi-Agent Systems" (2026)

**Source**: https://arxiv.org/html/2602.15198v1

**Summary**: Formal framework for auditing collusion using Distributed Constraint
Optimization Problems (DCOPs). Measures regret relative to cooperative optimum.
Key finding: LLM-as-a-judge alone is NOT sufficient — action-based metrics
(regret) are required. "Emergent collusion" appears even without explicit
prompting when secret channels exist.

**Relevance to CYNIC**:
- **Critical for Q5 (Collusion Threshold)**: Our simulation must include
  regret-based metrics, not just LLM-as-judge.
- "Emergent collusion" means our system must monitor for spontaneous coordination,
  not just explicit collusion.
- DCOP formalism provides a rigorous way to measure "coalition advantage" —
  the benefit a colluding group gains over honest participation.
- Three types of collusion: direct, attempted, hidden. Our detection must
  handle all three.

### 4.2 Multi-Agent Interpretability — "Detecting Collusion through Multi-Agent Interpretability" (2025)

**Source**: https://www.lesswrong.com/posts/Q65SETuTmhF4LWdix/detecting-collusion-through-multi-agent-interpretability-3

**Summary**: 5 probing techniques for collusion detection: Peak Suspicion,
Deception Split, Asymmetry Probe, Activation Alignment, Sorted Concatenation.
Steganographic collusion is fundamentally different from deliberative collusion.

**Relevance to CYNIC**:
- **Our collusion detection should combine all 5 paradigms** (not just one).
- Steganographic collusion (hiding signals in natural language) requires
  activation-level monitoring, not just text analysis.
- "Collusion on paper" — agents plan to collude but don't execute. Our system
  must detect both planning and execution.

### 4.3 Q-Learning Based Collusion Tolerance — arxiv 2512.22860 (2025)

**Source**: https://arxiv.org/html/2512.22860

**Summary**: Multi-agent RL for collusion defense. MARL achieves F1=0.85 against
collusive attacks vs F1=0.50 for single-agent RL. Time-delayed poisoning
(sleeper agents) is catastrophic for all approaches (F1 drops to 0.11).

**Relevance to CYNIC**:
- **Sleeper agents are our highest risk** — actors that build trust before
  gaming. Our probation mechanism (20 resolutions) mitigates this.
- MARL (multi-agent RL) is more robust than single-agent — our market is
  inherently multi-agent, which is a defense advantage.
- Adaptive trust consensus (our E-Score) should use similar RL-based weighting.

---

## 5. Reputation Systems

### 5.1 Design Considerations for Decentralized Reputation Systems (Petitcolas 2017)

**Source**: https://www.petitcolas.net/fabien/publications/rwot2017-reputation-design.pdf

**Summary**: Comprehensive framework for reputation system design. Key
principles: peers define and enforce shared ethics, incentives align behavior,
market corrects diversions over time.

**Relevance to CYNIC**:
- Our E-Score is a decentralized reputation system — this paper provides
  the design principles.
- "Peers define and enforced shared ethics" = our axiomatic constraints.
- "Market corrects diversions" = our resolution mechanism.

### 5.2 Reputation Tokenomics — "DAO Governance Design Analysis" (2024)

**Source**: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=5018833

**Summary**: Analysis of reputation tokenomics in DAOs. Reputation as a
non-transferable token that influences governance power.

**Relevance to CYNIC**:
- Our CYD is transferable (for betting), but E-Score is non-transferable
  (reputation). This dual-token model is validated by DAO research.

---

## 6. Key Insights Summary

| Insight | Source | Application |
|---------|--------|-------------|
| Use verifiable inference tasks as PoW | HadAgent | PoW-LLM formula |
| Multi-round debate > isolated voting | BlockAgents | PoJ jury design |
| Leaderless + Geometric Median | DecentLLMs | Jury aggregation |
| Receiver-side evaluation essential | SAC | PoJ scoring |
| (F+1)-robustness for graph topology | SAC | Network design |
| Regret metrics > LLM-as-judge | Colosseum | Collusion detection |
| Emergent collusion is real | Colosseum | Monitoring system |
| Sleeper agents are catastrophic | MARL paper | Probation mechanism |
| Hybrid trust models win | ERC-8004 | Multi-proof system |
| Confidence probes improve robustness | CP-WBFT | E-Score dimensions |

---

## 7. Open Questions from Literature

1. **Steganographic collusion**: Can we detect collusion hidden in natural
   language? (Multi-Agent Interpretability paper suggests activation-level
   monitoring, but this requires white-box access.)

2. **Sleeper agent detection**: No paper solves this satisfactorily. Our
   probation mechanism (20 resolutions) is a heuristic, not a proof.

3. **Quantum resilience**: Post-quantum signatures may be needed for long-term
   market integrity (arxiv 2606.24942).

4. **Cross-chain reputation**: If CYNIC expands to multiple chains, how does
   E-Score port across chains? (ERC-8004 addresses this partially.)
