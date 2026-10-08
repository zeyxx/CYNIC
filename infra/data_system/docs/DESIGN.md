# CYNIC Data System — Living Design Document (Causal Hypergraphs & Post-Training Pipelines)

**Document Status**: Living Architectural SSOT  
**Last Updated**: October 8, 2026  
**Primary Focus**: Non-linear Agentic Trajectory Modeling, Post-Training Data Systems, and Evals Infrastructure  

---

## 1. Problem Formulation & Anti-Linearity Axiom

Standard post-training data pipelines treat LLM chat sessions as linear Markov sequences:
$$\text{Session} = [m_0, m_1, m_2, \dots, m_n]$$

In modern autonomous agent frameworks (such as Hermes-Agent), **trajectories are inherently non-linear**. Flatenning them into linear chat sequences destroys vital causal context and introduces severe training biases:
1. **Parallel Execution**: Single assistant turns dispatch $K$ tool calls executed simultaneously. Linearization invents a false temporal sequence ($t_1 \rightarrow t_2$), forcing the model to learn nonexistent serial dependencies.
2. **Context Compaction ($N \rightarrow 1$)**: When context window limits are reached, $N$ historical messages are collapsed into an active summary. Linearization treats compacted messages as discarded or conflates the summary with raw history.
3. **Subagent Hierarchies ($1 \rightarrow M$)**: Sessions spawn child processes via asynchronous delegations. Linear pipelines treat child sessions in isolation, severing the causal link between parent intent and child execution.

**The Foundational Principle**:
An agent trajectory must be modeled as a **Directed Acyclic Hypergraph with a Lamport Partial Order**:
$$\mathcal{H} = (\mathcal{V}, \mathcal{E}, \prec)$$

---

## 2. Mathematical Formalism of the Causal Hypergraph

### A. Vertex Set $\mathcal{V}$ (State Entities)
* **$\text{TurnNode}(t)$**: Individual interaction turn with role, content, and native metadata.
* **$\text{CompactionNode}(c)$**: Dimensionality reduction node containing `_compressed_summary`.
* **$\text{SubagentRootNode}(s)$**: Root origin of an asynchronous child session.
* **$\text{EnvironmentSnapshot}(e)$**: Environment state at turn $t$ (working directory, git branch, modified files).

### B. Hyper-Edge Set $\mathcal{E}$ (Causal Derivations)
* **Sequential Edge ($1:1$)**: $T_i \xrightarrow{\text{seq}} T_{i+1}$ (Direct causal progression within the same local context).
* **Concurrent Tool Fork ($1:K$)**: $T_{\text{assist}} \xrightarrow{\text{fork}} \{C_1, C_2, \dots, C_K\}$ (Simultaneous tool dispatch where $\forall i \neq j, C_i \parallel C_j$).
* **Tool Join ($K:1$)**: $\{R_1, R_2, \dots, R_K\} \xrightarrow{\text{join}} T_{\text{assist}}$ (Aggregated perception of parallel results).
* **Compaction Hyper-Edge ($N:1$)**: $\{T_{a_1}, \dots, T_{a_N}\} \xrightarrow{\text{absorb}} C_{\text{summary}}$ (Lossless trace of absorbed historical states).
* **Delegation Hyper-Edge ($1:M$)**: $T_{\text{parent}} \xrightarrow{\text{delegate}} \{S_{\text{child}_1}, \dots, S_{\text{child}_M}\}$ (Inter-session causal branching).

### C. Lamport Partial Order ($\prec$)
Given two events $e_1, e_2 \in \mathcal{V}$:
* $e_1 \prec e_2 \iff e_1$ is in the causal past (light cone) of $e_2$.
* $e_1 \parallel e_2 \iff e_1 \not\prec e_2 \land e_2 \not\prec e_1$ (Events are concurrent and mutually independent).

---

## 3. The 8 Pillars of the Data Pipeline Architecture

```
                                  [1. Ingestion & Capture]
                                             │
                                             ▼
                                   [2. Profiling & EDA]
                                             │
                                             ▼
                                   [3. Hygiene & Quality]
                                   (Deduplication, Scrubbing)
                                             │
                                             ▼
                                [4. Causal Hypergraph Model]
                                (DAG, Poset, Subagent Trees)
                                             │
                                             ▼
                                 [5. Semantic Grounding]
                                (Task Success / Step Credit)
                                             │
                                             ▼
                                  [6. Leakage-Free Splits]
                                 (Temporal & Graph Disjoint)
                                             │
                                             ▼
                                  [7. Target Projections]
                           ┌─────────────────┴─────────────────┐
                           ▼                                   ▼
                  [SFT Training Datasets]             [DPO Contrastive Pairs]
                  (Loss-masked, Success only)         (Chosen vs Rejected)
                                             │
                                             ▼
                                [8. DataOps & Observability]
                                 (Lineage, JSON Event Logs)
```

---

## 4. Empirical Ground-Truth Findings & Solved Pitfalls

This section records empirical physical discoveries made on the real frozen corpus (`/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite` — 2,796 sessions, 314,194 messages):

### Pitfall 1: Non-Uniqueness of `message_uid` across Compaction Passes
* *Discovery*: A single `message_uid` appears multiple times in SQLite when Hermes performs iterative compactions (once active, then duplicated across subsequent compaction passes with `compacted=1`).
* *Resolution*: Mathematical assertion for compaction conservation must be $\text{count}(\text{DISTINCT } \text{message\_uid}) == \text{len}(\text{set}(\text{uids}))$, not a naive row count.

### Pitfall 2: Nested JSON in Command Arguments Masquerading as Multi-Tool Calls
* *Discovery*: Filtering by text patterns like `tool_calls LIKE '%},{%'` hits bash command arguments containing nested JSON strings, falsely classifying single-tool invocations as parallel calls.
* *Resolution*: Strict structural JSON parsing of `tool_calls` arrays to verify genuine array length $K > 1$.

### Pitfall 3: Subagent Tree Completeness
* *Discovery*: 603 out of 2,796 sessions declare a `parent_session_id`.
* *Resolution*: 100.0% (603/603) resolve to existing parent sessions within the capture. The inter-session delegation hierarchy forms a verified DAG with 0 cycles and 0 dangling nodes ($\alpha_{10}, \alpha_{12}$).

### Pitfall 4: CoT Phenotype Fragmentation
* *Discovery*: Model thinking is split across three distinct producer fields: `reasoning_content` (56,053), `codex_reasoning_items` (17,558), and `reasoning_details` (2,481).
* *Resolution*: Preserved as distinctly typed reasoning homes in canonical and model layers; never flattened into standard user/assistant text.

---

## 5. The Canonical 6-Stage Production Workflow & Audit Gates

The post-training data pipeline is executed strictly through 6 sequential stages, guarded by 5 non-negotiable Quality Gates:

```
[STAGE 0: INGESTION & CONSERVATION]  -> [GATE 0: DDL Closure & Non-Vacuity (α1-α9)]
                 │
                 ▼
[STAGE 1: PROFILING & EDA]           -> [GATE 1: Population Audit & Cohort Mapping]
                 │
                 ▼
[STAGE 2: HYGIENE & QUALITY]         -> [GATE 2: Deduplication & Attrition Waterfall]
                 │
                 ▼
[STAGE 3: CAUSAL HYPERGRAPH MODEL]   -> [GATE 3: Topological Audit & Acyclicity (α10-α14)]
                 │
                 ▼
[STAGE 4: SEMANTIC GROUNDING]        -> [GATE 4: Inter-Rater Reliability (Cohen's κ > 0.80)]
                 │
                 ▼
[STAGE 5: LEAKAGE-FREE SPLITS]       -> [GATE 5: Split Certification & Disjointness Proof]
                 │
                 ▼
[STAGE 6: PROJECTION & EXPORT]       -> [GATE FINAL: Artifact Lineage & Zero-Secret Proof]
```

---

## 6. Living Maintenance & Update Protocol

Whenever a new empirical fact or mathematical property is discovered:
1. Formulate a failing test first in `tests/test_model_topology_invariants.py`.
2. Implement the resolution in domain code.
3. Update this document under **Section 4 (Empirical Ground-Truth Findings)** with proof metrics.
4. Commit documentation changes alongside code changes.

---

## 7. The Full Spectrum of 7 Target Projections

The pipeline does not terminate at basic SFT/DPO datasets; it services 7 distinct AI and software engineering disciplines:

```
                       ┌─── [1. SFT] : Pure imitation of verified successful trajectories
                       ├─── [2. DPO / RLHF] : Contrastive preference alignment ({chosen, rejected})
                       ├─── [3. PRM / Step-Reward] : Step-by-step value models for MCTS & Search
                       │
CAUSAL HYPERGRAPH ─────┼─── [4. RLVR Gyms] : Autonomous self-play environments with test oracles
     (The SSOT)        │
                       ├─── [5. Golden Evals] : Sovereign benchmarks against external contamination
                       ├─── [6. Skill Mining & RAG] : Automatic procedural memory extraction
                       └─── [7. Forensics & Routing] : Cost-to-capability routing & drift detection
```

---

## 8. Multi-Week Temporal Evolution & Session Typology Architecture

### 8.1 Temporal Non-Stationarity Handling
Agent trajectories over multiple weeks suffer from environment drift, prompt updates, and shifting tool implementations. We govern temporal drift via:
1. **Deployment Epochs ($\mathcal{E}_1, \mathcal{E}_2, \dots$)**: Partitioning the corpus into distinct architectural phases (e.g., pre-refactor vs post-refactor) to prevent anachronistic training collisions.
2. **Git Commit Anchoring**: Every node in the hypergraph is indexed by the active repository commit SHA-1, decoupling historical truth from HEAD code changes.
3. **Idempotent Append-Only Storage**: New capture batches receive distinct SHA-256 digests and are ingested without rewriting historical canonical stores.

### 8.2 Beyond Monolithic Classification: Multidimensional Attribution

Reducing an entire session to a single rigid bucket (e.g., classifying a 50-turn session as "Type E: Pathology / DPO Rejected" because the last command was interrupted by Ctrl+C or a host SIGSEGV) is a critical methodological flaw. Sessions are collections of cognitive subgraphs and must be evaluated across 4 orthogonal dimensions:

```
                  ┌─── DIMENSION 1 : Physical Host Integrity (Host Layer)
                  │    (OOM kill, Python SIGSEGV, network timeout, disk full)
                  │
                  ├─── DIMENSION 2 : Model Cognition (Cognitive Layer)
TRAJECTORY ───────┤    (Tool syntax validity, plan coherence, logical deduction)
SUBGRAPH          │
                  ├─── DIMENSION 3 : Human Operator Steering (Interaction Layer)
                  │    (Natural approval, early Ctrl+C to redirect, out-of-band message)
                  │
                  └─── DIMENSION 4 : Temporal Prefix Locality (Decomposition Layer)
                       (Extracting autonomous valid prefixes 0..k from incomplete runs)
```

1. **Host Fault vs Model Fault Isolation**:
   - A host crash (e.g. host Linux SIGSEGV during pytest) is an environmental failure, not a model cognitive error. Training DPO against this unfairly penalizes valid model reasoning for physical hardware defects. Environmental faults are quarantined from preference alignment.
2. **Human Ctrl+C is Steering, Not Automatic Failure**:
   - Operator interruptions (`Ctrl+C`, `SIGINT`) frequently occur when a command takes too long, when the operator already has the answer, or to guide the agent to a better branch. The preceding turns $0 \dots k-1$ frequently constitute pristine SFT material.
3. **Prefix Slicing & Subgraph Harvesting**:
   - Incomplete or interrupted sessions are decomposed into their subgraphs. Every autonomous step sequence $[T_0 \rightarrow T_k]$ that achieved a verified intermediate goal (e.g. clean code exploration, successful uncorrupted patch, valid unit test) is harvested as a valid SFT demonstration.
4. **Target Routing by Subgraph, Not by Monolithic Session**:
   - A single session can yield SFT training prefixes from turns $0 \dots 20$, a DPO pair from a failed attempt at turn $22$ and its correction at turn $23$, and a refusal/steering alignment point at the final interruption at turn $25$.

---

## 9. Multi-Scale Trajectory Geometry & Cross-Outcome Disambiguation

### 9.1 The 4 Fractal Scales of Execution
Trajectories exhibit self-similarity across four scales:
* **Scale 0 (Micro — Step/Action)**: Atomic tool calls. Failure mode: schema error, crash, command typo.
* **Scale 1 (Meso — Thread/Session)**: Sequential conversation history. Failure mode: context saturation, plan drift.
* **Scale 2 (Macro — Subagent Swarm)**: Asynchronous tree of 1 parent and $M$ children. Failure mode: orphaned results, recursion loops.
* **Scale 3 (Meta — Cross-Session Saga)**: Cumulative development over days. Failure mode: regression of past contracts, architectural amnesia.

### 9.2 The Cross-Outcome Skepticism Matrix
A single green exit code or self-reported success claim is statistically unreliable. Disambiguation requires orthogonal cross-validation:

| Candidate Claim / Outcome | Primary Flaw / False Positive Mode | Orthogonal Disambiguating Oracle | Consequence |
|---|---|---|---|
| **Agent: "Bug fixed"** | Hallucination, premature claim | Non-empty `git diff` + `cargo test` exit 0 | Unverified claim rejected from SFT |
| **Tool exit code 0** | Tool reported success on wrong target | Next human turn ($t+1$) satisfaction/redirect | Silent failure demoted to failure label |
| **Session $N$ finished** | Broken build discovered next day | Next session ($N+1$) opening prompt | Session $N$ flipped to DPO $y_{\text{rejected}}$ |
| **Subagent reported Done** | Hallucinated side-quest | Parent session actually consumed return | Unused subagent marked dead branch |
| **Candidate SFT sample** | Plausible code that fails build | Hermetic RLVR sandbox compilation | Flawed code purged prior to training export |

---

## 10. Multi-Day Episode Segmentation & Concurrency Tensor Formulation

### 10.1 Empirical Discontinuity & Substance-Driven Segmentation
Long-running sessions spanning days or traversing unrelated features are segmented into autonomous episodes:
* **Empirical Temporal Regimes**: Rejecting arbitrary thresholds. 99.48% of work occurs in $\Delta t \le 10\text{ min}$. The statistical inflection occurs at $\Delta t > 30\text{ min}$ (inserting an `EpisodeBoundary`), while $\Delta t \in [8\text{h}, 16\text{h}]$ marks inter-day sleep/resumption.
* **Substance Over Naming for Crons**: Sessions named `cron_*` cannot be naively excluded. 320 code mutations (`patch`, `write_file`) and complex multi-turn debugging occur in crons; classification depends on action substance, not id prefix.
* **Topic Shift Isolation**: Subtree transitions (e.g. moving from `crates/cynic-host` to `crates/cynic-node`) form distinct episode clusters.
* **Compaction Folds as Epochs**: $N \rightarrow 1$ summary generation represents a natural phase transition.

### 10.2 Algebraic Concurrency & Resource Collision Formulation
Simultaneous sessions over shared worktrees are evaluated via linear algebra:
* **Causal Adjacency $\mathbf{A}$ & Resolvent**: Transitive dependency cone $\mathbf{C} = (\mathbf{I} - \mathbf{A})^{-1} - \mathbf{I}$. Disjoint sessions exhibit $C_{AB} = 0 \land C_{BA} = 0$.
* **State Collision Matrix**: $\mathbf{M}_{\text{collision}} = \mathbf{W} \mathbf{R}^T$ over touched file paths. Non-zero entries identify cross-session race conditions, isolating environment collisions from model reasoning errors.
* **Lamport Vector Clocks**: Logical timestamps $\vec{V} = (t_1, \dots, t_M)$ replace unstable physical clocks.

---

## 11. Semantic Discovery: LLM-Mining & RAG Blind-Spot Harvester

Static SQL and regex queries are blind to semantic subtleties (strange prompt wording, subtle reasoning deadlocks, implicit operator frustration). The pipeline deploys active semantic discovery:
1. **Semantic LLM-Mining over Dense Clusters**: Small local evaluators audit anomaly clusters to detect unmodeled edge cases.
2. **Associative RAG Cross-Correlator**: Indexes past failure signatures and queries historical sessions to surface recurring operational blind spots across months of work.

---

## 12. Human Entropy Modeling & Mandatory HITL Gating Protocol

### 12.1 The Dimensions of Real-World Human Entropy
Human software development is not a synthetic test harness; it is saturated with human entropy:
1. **Out-of-Band Displaced Mutations**: The operator edits files directly in their IDE (VSCode/Vim) while the agent is running or paused. The agent's subsequent tool executions see an updated reality with no corresponding turn in the capture log.
2. **Intentional Stress & Failure Probing**: The operator submits prompts designed to test the agent's boundaries or tool limits, which naive classifiers falsely label as user confusion or model failure.
3. **Laconic & Sarcastic Steering**: Human feedback is often terse ("no", "redo", "ok") and requires context from the developer's historical habits.

### 12.2 Paranoid Skepticism & HITL Arbitration
1. **Rejection of Surface Cleanliness**: Any metric, cluster, or outcome that looks cleanly sorted is treated with paranoid skepticism. Cleanliness in human interaction logs is often evidence of over-simplification or silent loss.
2. **Mandatory Human-in-the-Loop (HITL) Gate**: Ambiguous cases, boundary anomalies, and classification thresholds must never be settled autonomously by an agent. The pipeline surfaces the raw evidence, statistical quantiles, and candidate hypotheses, leaving the authoritative verdict to the operator.
