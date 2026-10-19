# CYNIC Data System — Governance & Operational Policy

**Document Status**: Canonical Normative Contract  
**Scope**: `infra/data_system`  
**Scope**: `apps/cynic-data`  
**Governing Authority**: CYNIC Constitutional & Data-Centric Architecture  

---

## 1. Constitutional Principles (First Principles)

1. **Principle of the Black Box (Total Reproducibility & Reversibility)**:
   Every data transformation from raw capture to model projection must be deterministic, pure, and falsifiable. Given identical input bytes, any independent observer must produce byte-identical canonical representations.

2. **The Truth Hierarchy (Separation of Truth from Projection)**:
   - The frozen capture is the immutable historical truth.
   - The canonical layer is the typed, lossless, harness-agnostic normalization of that truth.
   - The model layer is the causal hypergraph topology of dependencies and states.
   - The projection layer is the task-specific representation (SFT, DPO, RLVR, Evals). Projections may transform or filter, but they never overwrite or pollute upstream truth.

3. **Strict Conservation (`NULL != ""`)**:
   Absence of a value (`NULL` / `None`) and presence of an empty value (`""`) represent fundamentally distinct computational states. No pipeline step may coalesce, coerce, or normalize these states into a single representation.

4. **Fail-Closed on Schema Drift**:
   If a producer schema mutates, or if an in-scope table/column cannot be resolved, the pipeline must abort loudly with typed domain errors (`SourceSchemaDrift`). It is forbidden to degrade to silent empty values or partial ingestion.

---

## 2. Redaction & PII Governance Boundary

1. **Upstream Verbatim Invariant**:
   Capture and Canonical layers **must never redact, mask, or scrub** any payload, secret, or token. Redaction in upstream layers destroys auditability and mathematical conservation.
2. **Downstream Scrubbing Mandate**:
   Redaction (entropy scans, regex scrubbing for `sk-...`, API keys, private keys, authorization tokens) is strictly enforced at the **Projection Layer**. No unscrubbed artifact may be exported to external storage, published to Hugging Face, or passed to training runners.

---

## 3. Dataset Curation & Training Policy

1. **SFT Admission Rule (Strict Success Filtering)**:
   Supervised Fine-Tuning (SFT) is imitation learning. Trajectories containing unrecovered failures, infinite repetition loops, or non-zero exit codes must be strictly excluded from SFT datasets. A model must never be trained to imitate failed tool attempts.
2. **DPO & Contrastive Rule (Paired Optimization)**:
   Direct Preference Optimization (DPO) and RLHF require paired contrastive instances $\{x, y_{\text{chosen}}, y_{\text{rejected}}\}$. Rejected trajectories must originate from identical or causally comparable task contexts as their chosen counterparts.
3. **Anti-Leakage Splitting Mandate**:
   - Splitting by random session index is **strictly forbidden**.
   - Splits (Train / Validation / Test) must be partitioned by **Connected Hypergraph Components** (an entire subagent tree must belong entirely to one split).
   - Splits must enforce **Temporal Ordering** (training on past timestamps, evaluating on strictly future timestamps) and **Project/Repository Disjointness**.

---

## 4. Empirical Falsification Rule (Popperian Standard)

No claim of completeness, lossless conservation, or boundary isolation is accepted as prose. Every policy claim must be guarded by an automated regression test in `tests/` that is proved **non-vacuous** (i.e. demonstrably fails when the invariant is broken).

---

## 5. The 7 Imperative Precautions for LLM-Agent-Assisted Engineering

When building data pipelines via autonomous LLM agents, specific cognitive and mechanical failure modes emerge. The following 7 operational rules are binding on all agent sessions:

1. **Rejection of the Self-Report Fallacy**:
   An agent's narrative claim ("Tests pass", "No duplicates found") is zero evidence. Execution success is acknowledged ONLY through physical, machine-readable artifacts: exit code 0, written JSON report re-read from disk, or committed Git SHA.
2. **Mandatory Mutation & Non-Vacuity Verification**:
   A green test is guilty of vacuity until proven innocent. Every regression test must be subjected to a mutation test (breaking the implementation or running against pre-fix commit `HEAD~1`) to verify that it reliably turns red.
3. **Zero Defensive Coercion (Anti-Swallowing Rule)**:
   Defensive patterns (`x or ""`, `try/except Exception: pass`, `.strip()`) are strictly forbidden. Missing data or unmodeled variants must trigger typed domain exceptions (`SourceSchemaDrift`, `ContractViolation`), never silent defaults.
4. **Memory Isolation & Bounded Streaming**:
   Unbounded allocations (`fetchall()` on full tables, loading multi-hundred-MB JSON blobs into memory) are forbidden. All database and file readers must use streaming cursors or batch generators to prevent host kernel SIGSEGV or OOM-killer invocations.
5. **Living Context Anchoring (Anti-Amnesia)**:
   Decisions, DDL closures, and mathematical invariants must be committed into living documentation (`docs/POLICY.md`, `docs/SPEC.md`, `docs/DESIGN.md`) within the Git repository. Memory loss across context compaction windows must never alter architectural law.
6. **Strict Git Worktree & Claim Isolation**:
   Agents must never run blind `git add -A`. All work must be preceded by a formal lease claim (`cynic agent claim`), staged via dedicated temporary index files (`GIT_INDEX_FILE`), verified via `git diff --cached --stat`, and released with proof references.
7. **Source-Sink Contamination Firewall**:
   Under no circumstances may an agent data pipeline read from active execution stores (`~/.hermes/state.db`) or re-ingest active meta-dialogues about data engineering. The pipeline operates exclusively on frozen, content-addressed vault captures.

---

## 6. Scientific Protocol & Epistemological Framework

1. **Falsificationism (Karl Popper)**:
   Every data invariant $\alpha_k$ must be stated as a falsifiable empirical proposition. A proposition that cannot specify the exact condition of its empirical refutation is unscientific.
2. **Reproducibility Criterion**:
   Any result, metric, or dataset split must be independently reproducible from the raw capture using a single command, deterministic random seeds, and pinned dependencies.
3. **Audit Trail (Proof of Work)**:
   Every transformation step must emit structured metadata: input content hash, output content hash, runtime parameters, and execution duration.

---

## 7. Information & Data Engineering Standards

1. **DAMA-DMBOK Data Quality Dimensions**:
   Every layer must actively evaluate and report:
   - **Completeness**: $100\%$ table and column fate closure.
   - **Uniqueness**: Strict deduplication accounting via MinHash LSH and SHA-256.
   - **Consistency**: Exact cross-oracle coherence between raw events and independent telemetry.
   - **Validity**: Strict conformance to domain entity types.
   - **Accuracy**: Verbatim bit-level preservation ($NULL \neq ""$).
2. **Shannon Information Theory**:
   - Trajectory token streams must be evaluated for **Entropy Rate** $H(X)$ to detect repetitive loops (low-entropy degenerate loops).
   - Context window compression ($N \rightarrow 1$) must be evaluated as lossy channel encoding with bounded semantic distortion.
3. **Relational & Semi-Structured Integrity (Codd vs Graph)**:
   - Atomic entity records must preserve 3rd Normal Form (3NF) at the Canonical layer.
   - Relational joins must never destroy heterogeneous non-relational payloads (JSON objects, raw BLOBs).

---

## 8. Underlying Mathematical Framework

1. **Order Theory & Lamport Posets**:
   An agent execution trace is a partially ordered set $(S, \prec)$ where:
   - $a \prec b$ indicates a causal dependency ($a$ is in the backward light cone of $b$).
   - $a \parallel b \iff a \not\prec b \land b \not\prec a$ indicates concurrent, independent execution (e.g., parallel tool invocations).
2. **Directed Hypergraphs & Simplicial Complexes**:
   - $N \rightarrow 1$ context compactions are directed hyper-edges from a source set $|V| = N$ to a target summary $|V| = 1$.
   - $1 \rightarrow M$ subagent delegations are directed branching hyper-edges.
3. **Robust Multivariate Statistics & Measure Theory**:
   - Due to heavy-tailed power-law distributions (Pareto / Zipf) in trajectory length and token counts, Gaussian assumptions ($\mu, \sigma$) are rejected.
   - Location and scale estimators must use non-parametric statistics: **Median, Interquartile Range (IQR), Median Absolute Deviation (MAD)**.
   - Confidence bounds must be estimated via **Non-parametric Bootstrap** (1,000+ resamples).
4. **Psychometrics & Evaluator Calibration**:
   - Inter-judge agreement (human-to-LLM or LLM-to-LLM) must be evaluated via **Cohen’s $\kappa$** (nominal classification) and **Krippendorff’s $\alpha$** (multi-rater reliability).
   - Uncalibrated evaluators ($\kappa < 0.70$) are inadmissible as automated judges.

---

## 9. Real-World Environmental Robustness, Security, and Human Operational Invariants

To prevent silent failures in production-grade agent trajectories, the following 7 operational boundaries are enforced:

1. **Terminal Output Canonicalization (Physical Layer)**:
   Raw tool executions capture ANSI terminal escape sequences (`\x1b[...]`), cursor returns (`\r`), and dynamic progress bars.
   - *Rule*: Canonical layer preserves these verbatim. Projection layer must deterministically strip non-semantic ANSI escape codes and normalize `\r\n \rightarrow \n` to prevent context poisoning and corrupt tokenization.
2. **Judge Bias & Condorcet Cycle Elimination (Epistemic Layer)**:
   LLM-as-judge systems are susceptible to position bias ($A > B$ purely because $A$ precedes $B$), verbosity bias, and cyclic preferences ($A > B > C > A$).
   - *Rule*: All preference evaluations must execute bidirectional positional swaps $(A, B)$ and $(B, A)$. Pairs failing invariance under positional swap or violating transitivity are marked unstable and discarded.
3. **Loss-Masking Boundary (Tensors & Optimization Layer)**:
   - *Rule*: Assistant decision thoughts and tool call syntax/arguments have $\text{Loss} = 1.0$. User messages and tool execution outputs have $\text{Loss} = 0.0$ (masked). Tool outputs representing transient network errors (HTTP 429, timeouts) must never have gradients backpropagated through them.
4. **Indirect Prompt Injection & Data Poisoning Firewall (Security Layer)**:
   Web extractions and ingested files may contain adversarial prompt injection payloads.
   - *Rule*: A signature and heuristic scanner must audit raw external inputs prior to projection admission to prevent adversarial backdoor injection into model weights.
5. **Decoupling Observed History from Sandbox Replay (Temporal Layer)**:
   - *Rule*: Observed historical reality (what actually happened at timestamp $T$ in the capture) is distinct from reproducible execution replay. Replay requires pinning exact repository commit SHAs and containerized environments.
6. **Typing of Human Out-of-Band Interruptions (Interaction Layer)**:
   Mid-turn human steering (`[OUT-OF-BAND USER MESSAGE]`, `Ctrl+C`, `SIGINT`) must not be treated as natural model completions.
   - *Rule*: Termination states must be explicitly typed: `COMPLETED_NATURAL`, `USER_INTERRUPTED`, `AGENT_TIMEOUT`, `OOM_CRASH`. Truncated turns are excluded from SFT imitation but preserved for refusal/interruption alignment.
7. **Intellectual Property & Licensing Firewall (Legal Layer)**:
   - *Rule*: File paths referenced by `read_file` or `git` must be reconciled against repository `LICENSE` metadata to prevent proprietary or incompatible copyleft code from leaking into public training projections.

---

## 10. Human Operator Constituent Authority & Human Entropy Law

1. **Sovereign Constituent Authority**:
   The human operator is the sovereign constituent authority above all models, scores, and automated heuristics. When automated metrics conflict with human empirical domain experience (e.g., knowledge of past operational cycles, development habits, or real production incidents), the human operator's verdict overrides automated classifications.
2. **The Law of Human Entropy**:
   Real-world human interaction introduces fundamental entropy that cannot be reduced to clean algorithmic abstractions:
   - External, unrecorded mutations: Code edits made directly in IDEs/editors while the agent was idle.
   - Communicative entropy: Laconic steering, intentional failure testing (probing tools with deliberate bad inputs), and unstated vocal/mental context.
   - Shifting priorities: Sudden topic changes mid-stream.
3. **Paranoid Skepticism & Mandatory HITL Gating**:
   Every claim, metric, or classification that appears "clean" or "theoretically valid" on the surface is suspect by default. No open, ambiguous, or multi-faceted finding may be committed to downstream datasets without explicit Human-in-the-Loop (HITL) review and contextual arbitration by the operator.
