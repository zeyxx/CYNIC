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

### 8.2 The 5-Class Session Typology & Routing Matrix
Sessions exhibit starkly different computational dynamics and must be routed conditionally:

| Cohort | Characteristics | Deduplication / Filtering | Target Projections |
|---|---|---|---|
| **Type A: Crons & Monitors** | Periodic, high-frequency, bash checks, low variance | MinHash aggressive dedup (keep 1 canonical instance) | Forensics, Anomaly detection. (Excluded from SFT). |
| **Type B: Coding & Debugging** | Multi-file edits, test runs, error-correction cycles | High-fidelity causal cone extraction | SFT (chosen), DPO (pre/post fix), RLVR Gyms. |
| **Type C: Research & Doctrine** | Dense dialogue, architectural thinking, zero edits | Extraction of decisions, principles, and trade-offs | Skill Mining (`SKILL.md`), Associative RAG. |
| **Type D: Multi-Agent Swarms** | Asynchronous subagent trees ($1 \rightarrow M$, 603 sessions) | Causal DAG unification across session boundaries | Multi-Agent Coordination, Delegation training. |
| **Type E: Pathologies & Crashs** | Ctrl+C, infinite loops, rate limits, SIGSEGV | Explicit failure typing (`USER_INTERRUPTED`, `LOOP`) | DPO (rejected pairs), SOC Behavioral Alarms. |
