# CYNIC Data System — Technical Specifications & Requirements (Cahier des Charges)

**Document Status**: Specification & Requirements Baseline  
**Scope**: `infra/data_system`  

---

## 1. Architectural Architecture Layers

```
Layer 0: CAPTURE
  └── Input: Active producer state stores (e.g., Hermes SQLite state.db)
  └── Output: Frozen, content-addressed captures (/var/lib/cynic/vault/corpus/captures/*.sqlite)
  └── SLA: Read-only access (`mode=ro`, `immutable=1`), SHA-256 fingerprinting.

Layer 1: CANONICAL
  └── Input: Frozen capture SQLite databases
  └── Output: Normalized sessions, turns, tools, human signals, native metadata
  └── Storage: Canonical SQLite / Rust memory representations
  └── SLA: 100% column conservation, lossless roundtrip, determinism across instances.

Layer 2: MODEL (Causal Hypergraph)
  └── Input: Normalized canonical sessions
  └── Output: Directed Acyclic Hypergraph (DAG), Lamport poset orderings, subagent trees
  └── Topology: Multi-session causal links, N-ary compactions, tool concurrency ($A \parallel B$)

Layer 3: PROJECTION
  └── Input: Causal hypergraphs and state snapshots
  └── Output: Parquet datasets for SFT (ChatML/loss-masked), DPO pairs, Eval rubrics
  └── SLA: Zero unscrubbed secrets, schema-validated Parquet, full lineage back to raw rows.
```

---

## 2. Register of Invariants ($\alpha$)

### Stabilized & Automated Invariants (Layers 0 & 1)
* **$\alpha_1$ (Content Immutability)**: Captures are frozen and verified by SHA-256 hash.
* **$\alpha_2$ (DDL Closure)**: Every table in the producer schema is either covered by column fate contracts or formally declared in `out_of_scope`.
* **$\alpha_3$ (Strict Conservation)**: `NULL != ""` across all turns and tool raw outputs.
* **$\alpha_4$ (Determinism)**: $f(\text{capture}) \rightarrow \text{session}$ is a pure function. Repeated calls yield byte-identical sessions.
* **$\alpha_5$ (Storage Idempotence)**: Writing a session twice produces identical state via transactional upsert.
* **$\alpha_6$ (Fail-Closed Drift Guard)**: Missing in-scope tables raise `SourceSchemaDrift`.
* **$\alpha_7$ (Lossless Round-Trip)**: `read(write(normalize(x))) == normalize(x)` without information drift.
* **$\alpha_8$ (Cross-Oracle Telemetry)**: Token and cost telemetry verified against independent model usage tables.
* **$\alpha_9$ (Source Isolation)**: Pipeline tests execute exclusively against frozen captures, never live stores.

### Stabilized Topological Invariants (Layer 2)
* **$\alpha_{10}$ (Causal DAG Acyclicity)**: All inter-session delegation relationships form a strictly acyclic directed graph (tested on 603 child sessions).
* **$\alpha_{11}$ (Compaction Conservation)**: $N$-ary absorptions map distinct message UIDs to their summary node without loss across multi-pass compactions.
* **$\alpha_{12}$ (Subagent Resolution)**: 100% of declared parent sessions resolve to existing capture sessions (0 orphaned children).
* **$\alpha_{13}$ (Tool Concurrency Poset)**: $K > 1$ parallel tool calls within a single assistant turn form concurrent, incomparable elements ($A \parallel B$) in Lamport's partial order.
* **$\alpha_{14}$ (CoT Phenotype Conservation)**: Thinking fields (`reasoning_content`, `codex_reasoning_items`, `reasoning_details`) remain distinctly preserved.

---

## 3. Performance & Resource SLAs

1. **Throughput SLA**:
   Normalization of single large sessions ($>18,000$ messages) must complete in $< 5.0$ seconds in Python, and $< 0.1$ seconds in native Rust.
2. **Memory Safety**:
   Cursor streaming must be strictly enforced on SQLite connections. Loading unbounded record collections into contiguous memory arrays is prohibited to prevent host segmentation faults.
3. **Auditability**:
   Every normalization and projection run must emit structured, machine-parseable JSON events conforming to `cynic.canonical.event.v1`.

---

## 4. Target Projections Specification (The 7 Consumer Outputs)

The Causal Hypergraph serves 7 distinct downstream artifacts:

1. **Projection 1: Supervised Fine-Tuning (SFT)**
   - Format: `.parquet` / `.jsonl` with ChatML / Jinja templates.
   - Admission: 100% verified successful trajectories (`exit_code == 0`, no infinite loops).
   - Loss-Masking: $\text{Loss} = 1.0$ on assistant turns/thoughts; $\text{Loss} = 0.0$ on user messages and tool results.
2. **Projection 2: Direct Preference Optimization (DPO / RLHF)**
   - Format: `.parquet` triplets $\{x, y_{\text{chosen}}, y_{\text{rejected}}\}$.
   - Source: Paired task contexts with successful vs failed/interrupted trajectory completions.
3. **Projection 3: Process Reward Model (PRM / Step-level Supervision)**
   - Format: Stepwise trajectories with per-turn scalar rewards $r_t \in \{-1.0, 0.0, +1.0\}$.
   - Purpose: MCTS / Tree-of-Thought / Best-of-N inference-time compute guidance.
4. **Projection 4: Reinforcement Learning with Verifiable Rewards (RLVR Gyms)**
   - Format: Standalone reproducible execution scenarios (initial files, instructions, verifiable test oracle).
   - Purpose: Autonomous self-play policy training (PPO / GRPO).
5. **Projection 5: Sovereign Evaluation Benchmarks (Golden Evals)**
   - Format: Curated hard-problem slices with automated pass/fail grading suites (anti-contamination).
6. **Projection 6: Procedural Skill Mining & Few-Shot RAG**
   - Format: Markdown skill specs (`SKILL.md`) and dense vector embeddings of successful tool interventions.
7. **Projection 7: Agent Behavioral Forensics & Economic Routing**
   - Format: Telemetry matrices and anomaly signatures to detect infinite loops, context fatigue, and optimize model cost-to-capability routing.

---

## 5. Temporal Evolution & Session Typology Specification

### 5.1 Temporal Non-Stationarity Management
1. **Deployment Epochs ($\mathcal{E}_k$)**: Slicing the corpus by historical architectural eras to prevent anachronistic training contradictions.
2. **Git Commit State Anchoring**: Every session trajectory is bound to its exact repository commit SHA-1.
3. **Append-Only Idempotent Captures**: New time windows land in immutable, content-addressed files without mutating historical captures.

### 5.2 Session Typology Taxonomy (Substance-Driven, Not Prefix-Driven)
* **Periodic & Monitor Tasks**: Repetitive checks without mutations $\rightarrow$ MinHash deduplicated, routed to Forensics.
* **Autonomous Engineering & Bug Resolution (Including Active Crons)**: Sessions performing code modifications (`patch`, `write_file`) and multi-turn debugging (even if named `cron_*`) $\rightarrow$ Primary source for SFT, DPO, and RLVR.
* **Research & Conceptual Doctrine**: Dense dialogue, architectural design $\rightarrow$ Routed to Skill Mining and RAG.
* **Multi-Agent Delegations & Swarms**: Asynchronous forks ($1 \rightarrow M$) $\rightarrow$ Routed to hypergraph coordination training.
* **Interrupted & Pathological Sessions**: Timeouts, loops, Ctrl+C $\rightarrow$ Primary source for DPO rejected pairs and SOC detection.

---

## 6. Scale Modeling: Working Hypothesis $H_{\text{scales}}$ (Pending Empirical Measurement)

The multi-scale architecture is treated strictly as an unverified working hypothesis $H_{\text{scales}}$ awaiting topological measurement on the corpus:
1. *Candidate Scale 0 (Micro — Turn / Tool Call)*: Atomic action-perception steps. Validation: syntax, tool schema conformance, exit code.
2. *Candidate Scale 1 (Meso — Single Session Thread)*: Continuous linear thread within one agent execution. Validation: intra-session task objective fulfillment.
3. *Candidate Scale 2 (Macro — Subagent Delegation Tree)*: Asynchronous swarms spanning parent and child sessions ($1 \rightarrow M$, e.g. 603 child sessions). Validation: join resolution back to parent context.
4. *Candidate Scale 3 (Meta — Inter-Session Campaign / Saga)*: Multi-day workflows spanning consecutive sessions on the same repository. Validation: long-term test stability and repository convergence.

---

## 7. Cross-Outcome Disambiguation & Retroactive Oracle Protocol

To eliminate false positives through systemic skepticism, no outcome may self-validate. Every candidate label must be confirmed by an independent orthogonal oracle:

1. **Agent Assertion $\leftrightarrow$ Git Diff & Compiler Oracle**:
   Claims of completion ("I fixed the bug") are rejected as false positives unless confirmed by a non-empty `git diff` and compiler/test exit code 0 (`cargo test` / `pytest`).
2. **Local Turn Success $\leftrightarrow$ Next Turn Human Oracle ($t + 1$)**:
   An apparently successful tool execution (`exit_code: 0`) is retroactively demoted if the subsequent human turn signals redirection, dissatisfaction, or correction.
3. **Session Outcome $\leftrightarrow$ Retroactive Session Oracle ($S + 1$)**:
   The ultimate truth of session $N$ is decoded from the opening prompt of session $N+1$. An operator reporting regressions retroactively flips session $N$ into a negative demonstration ($y_{\text{rejected}}$) and establishes session $N+1$ as its contrastive repair ($y_{\text{chosen}}$).
4. **Subagent Return $\leftrightarrow$ Parent Decision Oracle**:
   Subagent executions are verified by checking whether the parent session adopted the child output or was forced to discard and re-execute it.

---

## 8. Empirical Temporal Regimes & Dynamic Discontinuity Specification

The arbitrary 4-hour threshold is rejected. Inter-turn delays ($\Delta t$) follow an empirical distribution across 310,507 transitions:
1. **Regime 1: Continuous Synchronous Stream ($\Delta t \le 10\text{ min}$)**:
   Accounts for **99.48%** of all transitions (62.34% in $<5\text{s}$, 29.16% in $5\text{s}-30\text{s}$, 5.31% in $30\text{s}-2\text{min}$, 2.67% in $2\text{min}-10\text{min}$). Modeled as continuous attention.
2. **Regime 2: Transitional Reflection / Short Pause ($10\text{ min} < \Delta t \le 30\text{ min}$)**:
   Accounts for **0.28%** of transitions. Short pause without context loss.
3. **Regime 3: Cognitive Episode Cut ($\Delta t > 30\text{ min}$)**:
   The statistical inflection point (**0.15%** of transitions). Any gap $>30\text{ min}$ marks a $>99.8\%$ probability of operator disengagement, inserting an explicit `EpisodeBoundary`.
4. **Regime 4: Inter-Day Resumption ($8\text{h} \le \Delta t \le 16\text{h}$)**:
   Distinct empirical mode (143 transitions) representing sleep / next-day resumption. Distinct project context re-evaluation required.

---

## 9. Concurrent Multi-Session Tensor & Matrix Formulation Specification

Simultaneous sessions operating in parallel over shared workspaces are governed by linear algebraic formulations:
1. **Causal Adjacency Matrix $\mathbf{A}$ & Transitive Closure**:
   $$\mathbf{C} = (\mathbf{I} - \mathbf{A})^{-1} - \mathbf{I} = \sum_{k=1}^\infty \mathbf{A}^k$$
   Two concurrent sessions $A$ and $B$ are strictly independent if and only if $C_{AB} = 0 \land C_{BA} = 0$.
2. **Resource Collision Matrix $\mathbf{M}_{\text{collision}}$**:
   Given write-incidence matrix $\mathbf{W}$ and read-incidence matrix $\mathbf{R}$ over modified file paths:
   $$\mathbf{M}_{\text{collision}} = \mathbf{W} \mathbf{R}^T$$
   $(\mathbf{M}_{\text{collision}})_{ij} \neq 0$ signals shared resource contention (e.g. concurrent git worktree writes), explaining transient tool failures as environmental race conditions rather than model errors.
3. **Lamport Vector Clocks**:
   Logical timestamps $\vec{V} = (t_1, t_2, \dots, t_M)$ decouple causal order from jittery host physical clocks.

---

## 10. Human Entropy & Mandatory HITL Quality Gates Specification

1. **Paranoid Skepticism Mandate**:
   Surface-level cleanliness in session traces is non-evidence. Every candidate finding or clean classification must be treated as a refutable hypothesis.
2. **Mandatory HITL Arbitration**:
   No ambiguous, edge-case, or multi-faceted session cluster may be committed to downstream SFT/DPO projections without explicit Human-in-the-Loop review and arbitration.
3. **Trace Grounding against External Mutations**:
   When tool outputs or compiler checks indicate file states inconsistent with prior turns, the pipeline must flag potential out-of-band operator edits (IDE mutations) rather than automatically asserting model hallucination.
