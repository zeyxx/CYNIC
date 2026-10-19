# CYNIC Epistemic Data Triad: Sovereign Architecture Charter

**Document Status**: Living Constitution (v1.0)
**Authority**: Human Operator (Constituent Power)
**Language Invariant**: 100% English (code, schemas, docstrings, commits)

---

## 0. Preamble: The Cybernetic Imperative

CYNIC is not a data pipeline. It is a **sovereign cybernetic system** that observes, falsifies, learns, and acts. The triad of Data Center, Epistemic Engine, and Associative RAG forms a closed-loop recursive architecture that mechanically enforces scientific rigor, economic accountability, and ecological sustainability.

**Core Law**: *No fact enters the system without a machine-verifiable receipt. No hypothesis survives without falsification. No token is spent without accounting.*

---

## 1. The Triad: Three Pillars of Sovereign Intelligence

```
                        ┌─────────────────────────────────────────────────────────┐
                        │              1. THE SOVEREIGN DATA CENTER              │
                        │                 (Matter / Physical Reality)              │
                        │   Capture ──> Canonical ──> Model (DAG/Poset) ──> Proj │
                        └───────────────────────────┬─────────────────────────────┘
                                                    │
                                   Raw traces       │   Physical facts
                                   & trajectories   │   (refuted / admitted)
                                                    ▼
┌─────────────────────────────────────────┐                   ┌─────────────────────────────────────────┐
│     2. THE EPISTEMIC ENGINE             │                   │     3. THE ASSOCIATIVE RAG &            │
│        (Method / Popper & Ralph)        │◄─────────────────►│        MULTI-SCALE MEMORY               │
│ • Hypothesis generation $H_i$           │   Context         │  (Living Sovereign Knowledge)           │
│ • Physical falsification probes         │   History         │ • Indexing of winning subgraphs         │
│ • Popperian verdicts (REFUTED/UNREFUTED)│   & Precedents    │ • Analogy detection & anti-patterns    │
│ • Retroactive truth propagation          │                   │ • Procedural memory (SKILL.md)          │
└─────────────────────────────────────────┘                   └─────────────────────────────────────────┘
                                                    ▲
                                                    │
                                   HITL Arbitration │   Learning directives
                                   & Consultation   │   & SFT/DPO trajectories
                                                    ▼
                        ┌─────────────────────────────────────────────────────────┐
                        │             ACTORS & CONSUMERS OF THE SYSTEM           │
                        │  • Human Operator (Constituent Power, HITL)            │
                        │  • Coder Agents (SFT consumers & RLVR oracles)         │
                        │  • Epistemic Judge / Reviewer (Evals & Audits)        │
                        │  • Autonomous Workers (Ralph loops / Autoresearch)     │
                        └─────────────────────────────────────────────────────────┘
```

### 1.1 Pillar 1: The Sovereign Data Center

**Nature**: Physical & Topological
**Fundamental Question**: *"What exactly happened on the machine?"*
**Produces**: Deterministic causal graphs, `NULL != ""` invariants, leak-proof partitions, Parquet projections.

**Mandatory Invariants** ($\alpha_1 \rightarrow \alpha_{14}$):
- $\alpha_1$: Strict causal acyclicity (DAG)
- $\alpha_2$: $N \rightarrow 1$ compaction conservation
- $\alpha_3$: Transitive subagent tree resolution
- $\alpha_4$: Tool call concurrency as Lamport Poset ($A \parallel B$)
- $\alpha_5$: Integral CoT phenotype conservation
- $\alpha_6$: `NULL != ""` (absolute rejection of defensive coercion)
- $\alpha_7$: Deterministic normalization (bit-for-bit reproducibility)
- $\alpha_8$: Fail-closed schema drift (`SourceSchemaDrift`)
- $\alpha_9$: Frozen capture reads (never the active DB)
- $\alpha_{10}$: Acyclic causal strictness
- $\alpha_{11}$: $N \rightarrow 1$ compaction n-ary conservation
- $\alpha_{12}$: Transitive subagent resolution
- $\alpha_{13}$: Tool concurrency Poset
- $\alpha_{14}$: CoT phenotype conservation

**Scale Modeling: Working Hypothesis $H_{\text{scales}}$ (Pending Empirical Measurement)**:
*Epistemic Guard*: The 4-scale taxonomy is a conceptual hypothesis derived from operator memory and literature, **not an empirically observed law**. The true scale distribution, tree branching factor, delegation depth, and community clusters must be measured physically on the frozen corpus before promotion to architectural invariants.
Candidate scales under Popperian falsification:
1. *Candidate Scale 0 (Micro — Turn / Tool Call)*: Atomic action-perception.
2. *Candidate Scale 1 (Meso — Session Thread)*: Continuous linear execution.
3. *Candidate Scale 2 (Macro — Subagent Swarm)*: Asynchronous $1 \rightarrow M$ trees.
4. *Candidate Scale 3 (Meta — Cross-Session Saga)*: Multi-day workflows.

**Empirical Temporal Regimes** (from 310,507 transitions):
- **Regime 1**: Continuous synchronous stream ($\Delta t \le 10\text{ min}$) — 99.48% of transitions
- **Regime 2**: Transitional reflection ($10\text{ min} < \Delta t \le 30\text{ min}$) — 0.28%
- **Regime 3**: Cognitive episode cut ($\Delta t > 30\text{ min}$) — 0.15% (inserts `EpisodeBoundary`)
- **Regime 4**: Inter-day resumption ($8\text{h} \le \Delta t \le 16\text{h}$) — 143 occurrences

**Concurrency Algebra**:
- Causal Adjacency $\mathbf{A}$ & Transitive Closure: $\mathbf{C} = (\mathbf{I} - \mathbf{A})^{-1} - \mathbf{I}$
- Resource Collision: $\mathbf{M}_{\text{collision}} = \mathbf{W} \mathbf{R}^T$
- Lamport Vector Clocks: $\vec{V} = (t_1, \dots, t_M)$

### 1.2 Pillar 2: The Epistemic Engine

**Nature**: Methodological & Critical
**Fundamental Question**: *"Is this apparent result a verified fact or an illusion?"*
**Produces**: Popperian falsification verdicts, false positive elimination, autonomous recursive investigation.

**The Recursive Falsification Loop (Ralph / Autoresearch)**:
```
┌─────────────────────────────────────────────────────────────────┐
│                    EPISTEMIC ENGINE LOOP                        │
│                                                                 │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│   │ 1. HYPOTHESIS │───►│ 2. NULL HYP. │───►│ 3. DESTRUCTIVE│     │
│   │  GENERATION   │    │  FORMULATION │    │  PROBE DESIGN │     │
│   │  (LLM L2)     │    │  $H_0$       │    │  (SQL/Python) │     │
│   └──────────────┘    └──────────────┘    └──────┬───────┘     │
│                                                  │              │
│   ┌──────────────┐    ┌──────────────┐    ┌──────▼───────┐     │
│   │ 6. FACT      │◄───│ 5. VERDICT   │◄───│ 4. EXECUTION │     │
│   │  REGISTRY    │    │  (Popper)    │    │  (Sandbox)   │     │
│   │  UPDATE      │    │              │    │              │     │
│   └──────────────┘    └──────────────┘    └──────────────┘     │
│          │                                                      │
│          ▼                                                      │
│   ┌──────────────┐                                             │
│   │ 7. RETROACTIVE│                                            │
│   │  PROPAGATION  │                                            │
│   │  (RAG update) │                                            │
│   └──────────────┘                                             │
└─────────────────────────────────────────────────────────────────┘
```

**Mandatory Protocol**:
1. **Anti-Confirmation Bias**: The engine is FORBIDDEN from seeking confirming examples. It must design the most aggressive test to destroy its own hypothesis.
2. **Physical Receipt Requirement**: No fact enters the registry without an immutable machine receipt (SHA-1, SQLite hash, exit code).
3. **Retroactive Truth Propagation**: A truth discovered at turn 50 or session $N+1$ must retroactively correct labels at turn 0 of session $N$.
4. **Echo Chamber Prevention**: Every fact in the registry must carry its verification pointer. Recursive loops must anchor to physical receipts, not to prior LLM summaries.

### 1.3 Pillar 3: The Associative RAG & Multi-Scale Memory

**Nature**: Cognitive & Memorial
**Fundamental Question**: *"Where have we already solved a structurally identical problem?"*
**Produces**: Winning subgraph retrieval, procedural memory, cross-session contextualization.

**Architecture**:
1. **Semantic LLM-Mining over Dense Clusters**: Small local evaluators audit anomaly clusters for unmodeled edge cases.
2. **Associative RAG Cross-Correlator**: Indexes past failure signatures, queries historical sessions for recurring blind spots.
3. **Procedural Memory (`SKILL.md`)**: Distilled operational knowledge from validated subgraphs.
4. **Quarantine of Purity**: Only subgraphs validated by L0 execution oracle (code compiles + test passes) are admitted into the vector index.

**Hybrid Retrieval Mandate**:
- **FTS5 / BM25** (lexical exact): Symbol-level, identifier-level, error-code-level search.
- **Dense Embeddings**: Intent and dialogue-level semantic search.
- **Forbidden**: Dense-only RAG on code and terminal logs (semantic collapse on `mut` vs `&mut`).

---

## 2. The Sovereign Agentic Epistemic Harness

CYNIC requires a **3-component harness** that mechanically enforces scientific rigor:

```
                  ┌────────────────────────────────────────────────────────┐
                  │          CYNIC SOVEREIGN AGENTIC HARNESS               │
                  └──────────────────────────┬─────────────────────────────┘
                                             │
      ┌──────────────────────────────────────┼──────────────────────────────────────┐
      │                                      │                                      │
      ▼                                      ▼                                      ▼
1. EXECUTION HARNESS                 2. TOPOLOGICAL HARNESS               3. FALSIFICATION HARNESS
   (Physical / Sandboxed)              (Structural / AST)                  (Epistemic / Popper)
• Hermetic Linux sandbox             • Causal DAG conservation            • Null hypothesis $H_0$
• Bit-for-bit deterministic replay    • Rust/Python AST verification      • Destructive probe design
• Memory/CPU quota isolation          • Strict type conservation          • Popperian verdict
• Real exit code capture              (`NULL != ""`)                      • Retroactive correction
```

### 2.1 Execution Harness (L0 Physical Oracle)
- Replays actions in hermetic containers.
- Measures ground truth: does the code compile? does the test pass?
- **Asymmetry Exploitation**: Verification is $O(1)$ CPU, generation is $O(n)$ tokens. Always verify first.

### 2.2 Topological Harness (Structural Verification)
- Verifies $\alpha_1 \rightarrow \alpha_{14}$ without code execution.
- AST-level analysis for Rust/Python.
- Type conservation: `NULL != ""` enforced at every boundary.

### 2.3 Falsification Harness (Epistemic Verification)
- Drives hypothesis testing with destructive intent.
- Produces `REFUTED` or `UNREFUTED` verdicts.
- Updates the Fact Registry with machine receipts.

---

## 3. The Popperian LLM Tribunal

### 3.1 What the Tribunal PERMITS
- **Semantic ambiguity detection**: Is a response off-topic, aggressive, or doctrinally misaligned?
- **Diverse perspective ensemble**: Security-oriented vs performance-oriented model confrontation.
- **Qualitative explainability**: Produces reasoned arguments for human operators.

### 3.2 What the Tribunal FORBIDS
- **Physical truth voting**: 5 LLMs unanimously claiming "this code compiles" has **ZERO** proof value if `cargo check` fails.
- **Family collusion**: Two models from the same training lineage share blind spots and mutually approve.
- **Eloquence bias**: The most verbose and sycophantic model wins over a laconic correct answer.

### 3.3 The New Tribunal Protocol
```
               POPPERIAN EPISTEMIC TRIBUNAL (Refutative Regime)
               
      [Accusation] ──> Must bring executable material proof
                       (Exit code != 0, regression trace, broken diff).
                       
      [Defense]    ──> Must bring a green unit-test counter-proof.
      
      [LLM Judges] ──> CANNOT vote on material facts.
                       STRICTLY limited to:
                       1. Human intent interpretation.
                       2. Doctrinal contract alignment.
                       3. Statistical agreement (κ Cohen > 0.75).
```

### 3.4 Judge Calibration Drift Prevention
- Every LLM judge must be anchored to a **Golden Calibration Set** of 50 immutable cases.
- Re-evaluated every session to detect silent API model changes.
- If $\kappa$ drops below 0.75, the judge is suspended pending recalibration.

---

## 4. The L0 → L3 Computational Cascade

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    COMPUTATIONAL CASCADE                                │
├─────────┬───────────────┬───────────────────┬───────────────────────────┤
│  Tier   │  Compute      │  Cost             │  Role                     │
├─────────┼───────────────┼───────────────────┼───────────────────────────┤
│  L0     │  CPU only     │  $0.00            │  SQLite queries, AST      │
│  Zero   │  0 tokens     │  0 GPU            │  parsers, FTS5, exit      │
│  GPU    │               │                   │  code checks              │
├─────────┼───────────────┼───────────────────┼───────────────────────────┤
│  L1     │  Local SLM    │  ~$0.01/query     │  Hypothesis generation,   │
│  Fast   │  7B-14B       │  Low GPU          │  candidate filtering,     │
│  Local  │  (Qwen/Llama) │                   │  anomaly detection        │
├─────────┼───────────────┼───────────────────┼───────────────────────────┤
│  L2     │  Frontier     │  ~$0.10-1.00/     │  Final synthesis, high-   │
│  Large  │  Model        │  query            │  entropy cases, HITL      │
│  API    │  (Claude/GPT) │  High GPU         │  escalation               │
├─────────┼───────────────┼───────────────────┼───────────────────────────┤
│  L3     │  Human        │  ~$50-200/hour    │  Sovereign arbitration,   │
│  Human  │  Operator     │  Irreplaceable    │  doctrinal decisions,     │
│  HITL   │               │                   │  constitutional changes   │
└─────────┴───────────────┴───────────────────┴───────────────────────────┘
```

**Mandatory Routing Rule**: L0 verification MUST complete before L1/L2 inference is invoked. Physical truth is always cheaper and more reliable than cognitive inference.

---

## 5. The CYNIC Memory System

### 5.1 Memory Architecture (Emerging from the Epistemic Engine)

The memory system is NOT a passive store. It is an **active, self-correcting knowledge graph** that emerges from the recursive falsification loop:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CYNIC MEMORY SYSTEM                                 │
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  WORKING    │  │  EPISODIC   │  │  PROCEDURAL │  │  DOCTRINAL  │    │
│  │  MEMORY     │  │  MEMORY     │  │  MEMORY     │  │  MEMORY     │    │
│  │             │  │             │  │             │  │             │    │
│  │ Current     │  │ Session     │  │ SKILL.md    │  │ POLICY.md   │    │
│  │ session     │  │ narratives  │  │ patterns    │  │ SPEC.md     │    │
│  │ context     │  │ & outcomes  │  │ & recipes   │  │ DESIGN.md   │    │
│  │             │  │             │  │             │  │             │    │
│  │ TTL: 1 hour │  │ TTL: 7 days │  │ TTL: 30 days│  │ TTL: forever│    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    FACT REGISTRY (Immutable)                     │   │
│  │  • Every fact carries: SHA-1, source query, exit code, timestamp │   │
│  │  • Retroactive correction: new facts update old labels           │   │
│  │  • Echo chamber prevention: no fact without physical receipt     │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Memory Invariants
1. **No fact without receipt**: Every memory entry must carry an immutable machine receipt.
2. **Retroactive correction**: New truths must propagate backward to update old labels.
3. **Quarantine of purity**: Only L0-validated subgraphs enter procedural memory.
4. **TTL-based decay**: Working memory (1h) → Episodic (7d) → Procedural (30d) → Doctrinal (forever).

---

## 6. Recursive Architecture: The LLM Play

### 6.1 Why Recursion is the Next Frontier

The recursive architecture is not a luxury; it is a **mathematical necessity** for scaling epistemic rigor:

1. **Hypothesis → Test → Refutation → New Hypothesis**: Each cycle tightens the boundary of known truth.
2. **Retroactive propagation**: Truths discovered at depth $N$ correct labels at depth $0$.
3. **Autonomous operation**: The system investigates without saturating human attention.

### 6.2 The Recursive Loop Contract

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RECURSIVE INVESTIGATION CONTRACT                     │
│                                                                         │
│  INPUT:  Hypothesis $H_i$ (natural language or formal logic)           │
│                                                                         │
│  STEP 1: Formulate null hypothesis $H_0$                                │
│  STEP 2: Design destructive probe (SQL/Python/AST)                     │
│  STEP 3: Execute in hermetic sandbox (L0)                               │
│  STEP 4: Measure result (exit code, hash, diff)                         │
│  STEP 5: Popperian verdict:                                            │
│          • REFUTED → Generate new hypothesis $H_{i+1}$                  │
│          • UNREFUTED → Record fact with receipt                         │
│  STEP 6: Retroactive propagation (update RAG, memory, labels)          │
│  STEP 7: If confidence < threshold → escalate to L3 (Human HITL)       │
│                                                                         │
│  OUTPUT: Fact Registry entry + updated memory + verdict                │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.3 Echo Chamber Prevention in Recursive Loops
- **Physical anchor requirement**: Every recursive iteration must reference at least one L0 physical receipt.
- **No prose-only facts**: A fact stated in natural language without a machine pointer is rejected.
- **Diversity enforcement**: If 3 consecutive iterations produce the same verdict, the loop must inject a perturbation or escalate to L3.

---

## 7. Token & Compute Economy

### 7.1 Accounting Ledger

Every LLM invocation must be recorded in an immutable ledger:

```json
{
  "schema": "cynic.token_ledger.v1",
  "timestamp": "2026-10-08T18:30:00Z",
  "tier": "L1",
  "model": "qwen-2.5-coder-7b-local",
  "input_tokens": 1200,
  "output_tokens": 350,
  "cost_usd": 0.0035,
  "gpu_seconds": 2.1,
  "purpose": "hypothesis_generation",
  "hypothesis_id": "H_20261008_001",
  "verdict": "UNREFUTED",
  "receipt_sha1": "a1b2c3d4..."
}
```

### 7.2 Ecological Constraints
1. **L0 first**: No L1/L2 invocation until L0 verification is complete.
2. **Budget caps**: Per-session token budgets with hard stops.
3. **Carbon tracking**: GPU-seconds logged per query.
4. **Cost-benefit gating**: If L0 can answer the question, L1/L2 is forbidden.

### 7.3 The Asymmetry Principle
- **Verification**: $O(1)$ CPU, 0 tokens, ~50ms
- **Generation**: $O(n)$ tokens, ~10min, ~50k tokens
- **Rule**: Always verify physically before inferring cognitively.

---

## 8. Strict Journaling & Auditability

### 8.1 Event Schema
All pipeline events must conform to `cynic.canonical.event.v1`:

```json
{
  "schema": "cynic.canonical.event.v1",
  "event_id": "evt_20261008_001",
  "timestamp": "2026-10-08T18:30:00Z",
  "source": "epistemic_engine",
  "type": "hypothesis_tested",
  "hypothesis_id": "H_20261008_001",
  "verdict": "UNREFUTED",
  "receipt": {
    "sha1": "a1b2c3d4...",
    "exit_code": 0,
    "query_hash": "e5f6g7h8..."
  },
  "token_cost": {
    "input": 1200,
    "output": 350,
    "usd": 0.0035
  }
}
```

### 8.2 Audit Trail Requirements
1. **Append-only**: Events are never deleted or modified.
2. **Content-addressed**: Each event carries its own hash.
3. **Causal ordering**: Lamport timestamps for concurrent events.
4. **Full reproducibility**: Given the same inputs, the same event sequence must be produced.

---

## 9. The 12 Blind Spots & Their Invariants

| # | Blind Spot | Invariant / Rule |
|---|---|---|
| 1 | **LLM Confirmation Bias** | Forbidden to seek confirming examples; must design destructive probes |
| 2 | **LLM-as-Judge Pathologies** | Position permutation, κ Cohen > 0.75, L0 pre-filter |
| 3 | **Recursive Echo Chamber** | Physical anchor required; no prose-only facts |
| 4 | **Embedding Collapse on Code** | Hybrid FTS5 + dense; dense-only forbidden on code |
| 5 | **Indirect Prompt Injection** | `<untrusted_historical_trace>` sanitization |
| 6 | **Token Cost Explosion** | L0 → L1 → L2 cascade; budget caps |
| 7 | **Operator Cognitive Saturation** | Batched HITL; compact decision sheets |
| 8 | **Human Entropy (Out-of-band mutations)** | Flag inconsistent file states; HITL arbitration |
| 9 | **Surface Cleanliness Illusion** | Paranoid skepticism; clean = unverified hypothesis |
| 10 | **Judge Calibration Drift** | Golden Calibration Set; κ re-evaluation per session |
| 11 | **RAG Contamination** | Quarantine of purity; only L0-validated subgraphs indexed |
| 12 | **Serialization Overhead** | Apache Arrow in-memory; Parquet on disk |

---

## 10. Serialization & Data Formats

### 10.1 In-Memory: Apache Arrow
- Zero-copy sharing between Data Center, Epistemic Engine, and RAG.
- Columnar format for analytical queries.
- Language-agnostic (Rust, Python, C++).

### 10.2 On-Disk: Apache Parquet
- Compressed columnar storage for training projections.
- Schema evolution support.
- Predicate pushdown for efficient filtering.

### 10.3 Event Log: JSONL with Content Addressing
- Append-only, immutable.
- Each line carries its own SHA-1.
- Lamport timestamps for causal ordering.

---

## 11. Actor & Consumer Matrix

| Actor | Role | Consumes | Produces |
|---|---|---|---|
| **Human Operator** | Constituent Power, HITL | Decision sheets, anomaly reports | Arbitrations, doctrinal changes |
| **Coder Agents** | SFT consumers, RLVR oracles | Few-shot RAG, SFT/DPO data | New session traces |
| **Epistemic Judge** | Evals & Audits | Calibration sets, test cases | Verdicts, κ scores |
| **Autonomous Workers** | Ralph loops, Autoresearch | Hypotheses, corpus | Facts, refuted hypotheses |

---

## 12. The 7 Target Projections (Outcomes)

| # | Projection | Definition | Role |
|---|---|---|---|
| 1 | **SFT** | Successful trajectories with strict loss-masking | Direct imitation learning |
| 2 | **DPO/RLHF** | Contrastive pairs $\{x, y_{\text{chosen}}, y_{\text{rejected}}\}$ | Negative preference alignment |
| 3 | **PRM** | Step-by-step scalar rewards $r_t \in \{-1, 0, +1\}$ | Inference-time search guidance (MCTS) |
| 4 | **RLVR Gyms** | Autonomous training scenarios with oracle verification | Self-play RL with automatic validation |
| 5 | **Golden Evals** | 50 hardest bugs with automatic verifiers | Sovereign benchmark, contamination-immune |
| 6 | **Skill Mining & RAG** | `SKILL.md` extraction, vector index of winning subgraphs | Zero-GPU procedural memory |
| 7 | **Forensics & Routing** | Behavioral signatures, cost/token matrices | Security detection, economic model selection |

---

## 13. Governance & Sovereignty

### 13.1 Authority Hierarchy
1. **Observed runtime state** (physical truth)
2. **`contracts/foundation.json`** (constitutional law)
3. **`infra/runtime-authority.json`** (operational authority)
4. **`infra/registry.json`** (placement registry)
5. **`Cargo.toml` workspace** (dependency graph)
6. **Canonical Rust crates** (implementation)
7. **`infra/agent-exposure.json`** (surface exposure)

### 13.2 Human Sovereignty
- The human operator is the **constituent power** above all models, scores, and automated heuristics.
- Automated metrics NEVER override human empirical domain experience.
- All ambiguous cases require explicit HITL arbitration.

### 13.3 Agent Identity
- Every agent commit carries the agent's own identity (`-c user.name=<agent-id>`).
- The default Git identity belongs to the human operator alone.
- Agent work signed with the default identity is indistinguishable from operator work and untriagable.

---

## 14. Conclusion: The Cybernetic Closure

The CYNIC Epistemic Data Triad is not a tool; it is a **sovereign cybernetic organism** that:
1. **Observes** physical reality through the Data Center.
2. **Falsifies** illusions through the Epistemic Engine.
3. **Remembers** validated truths through Associative RAG.
4. **Acts** through the 7 target projections.
5. **Accounts** for every token, every computation, every fact.
6. **Corrects** itself recursively through retroactive propagation.

This architecture mechanically enforces:
- **Scientific rigor** (Popperian falsification, physical receipts)
- **Economic sustainability** (L0 → L3 cascade, token accounting)
- **Ecological responsibility** (carbon tracking, budget caps)
- **Human sovereignty** (HITL gating, constituent authority)

The system is designed to **scale without saturating human attention**, to **investigate without echo chambers**, and to **learn without self-poisoning**.

---

**End of Charter**
