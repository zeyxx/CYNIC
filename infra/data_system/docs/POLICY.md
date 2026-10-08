# CYNIC Data System — Governance & Operational Policy

**Document Status**: Canonical Normative Contract  
**Scope**: `infra/data_system`  
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

## 4. Empirical Falsification Rule

No claim of completeness, lossless conservation, or boundary isolation is accepted as prose. Every policy claim must be guarded by an automated regression test in `tests/` that is proved **non-vacuous** (i.e. demonstrably fails when the invariant is broken).
