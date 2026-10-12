# RFC-003: Sovereign Data-Centric Distillation & Askesis Execution Trace Pipeline

- **Status**: Draft
- **Author**: Titouan Corneloup (zeyxx) / Antigravity & Hermes
- **Date**: 2026-09-21
- **Target Crates**: `cynic-protocol`, `cynic-intel`, `cynic-pkg`, `cynic-askesis`, `cynic-inference`, `cynic-cli`
- **Related Requirements**: Foundation §468 (protocol_is_pure), §472 (cli_is_adapter), §476 (proof_producers_are_low_layer), §481 (one_boundary_one_reason)

---

## 1. Context and Problem Statement

Autonomous multi-agent runtimes currently face a structural paradox:
1. **Frontier Model Friction**: Frontier LLMs (70B+ parameters, cloud APIs) possess vast general knowledge but suffer from severe epistemic asymmetry when operating low-layer sovereign systems. They hallucinate ad-hoc shell commands (e.g. attempting unadmitted `cargo build` without stack adjustments, unadmitted `git add .`, or bypassing claim protocols), ignore local architectural boundaries, carry heavy token context overhead, and introduce external network egress and credential risks.
2. **Untapped Execution Wealth**: In CYNIC, every admitted mutation produces an immutable causal trace: an agent claim (`.agent-claims.json`), bounded actions behind typed ports, execution receipts, and a cryptographic read-back proof (`RuntimeProofBundle` with SHA3-512). These traces currently sit as passive audit artifacts in Askesis journals, Antigravity logs, and agent transcripts rather than feeding back into the system's cognitive capability.
3. **Data-Centric Opportunity**: Narrow task specialization demonstrates that a 1.5B parameter model trained on 200–500 verified domain examples consistently outperforms generic frontier models on specific execution tasks, with orders-of-magnitude lower latency, zero cloud dependency, and complete hardware sovereignty.

This specification formalizes the **data-centric distillation loop** for CYNIC: transforming verified execution traces into high-soundness training and evaluation datasets for local, sovereign micro-models.

---

## 2. Decision and Formal Specification

CYNIC adopts a closed-loop data-centric architecture where verified historical execution traces are harvested, schema-validated, domain-partitioned, and converted into Supervised Fine-Tuning (SFT) and Direct Preference Optimization (DPO) datasets.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        DATA-CENTRIC FEEDBACK LOOP                      │
├────────────────────────────────────────────────────────────────────────┤
│  [ Operator Intent / Task ]                                           │
│             │                                                          │
│             ▼                                                          │
│  [ Agent Execution Trajectory (Antigravity / Hermes / Codex) ]         │
│             │                                                          │
│             ▼                                                          │
│  [ Proof Verification Gate (Claim Valid + Read-Back Proof OK) ]        │
│             │                                                          │
│     ┌───────┴────────────────────────┐                                 │
│     │ SUCCESS                        │ VIOLATION / REJECTED            │
│     ▼                                ▼                                 │
│  [ Gold SFT Example ]             [ Negative DPO Branch ]              │
│  (Intent → Bounded Argv)          (Rebuffed ad-hoc command)            │
│     │                                │                                 │
│     └───────┬────────────────────────┘                                 │
│             ▼                                                          │
│  [ Domain Splitting: Train 80% / Val 10% / Test 10% by Crate ]         │
│             │                                                          │
│             ▼                                                          │
│  [ QLoRA Distillation → 1.5B Specialist Adapter ]                      │
│             │                                                          │
│             ▼                                                          │
│  [ Evaluation Gate: CYNIC-Eval-Harness (Zero Hallucination) ]          │
│             │                                                          │
│             ▼                                                          │
│  [ Sovereign Local Serving: cynic-inference (llama.cpp / vLLM) ]       │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Formal Invariants

1. **INV-01 (Proof-Grounded Admissibility)**:
   An execution trace $T$ is admissible for inclusion in the training corpus $\mathcal{D}$ if and only if:
   * The active agent lease was formally acquired (`status == "claimed"`), unexpired, and released without budget overrun.
   * The terminal read-back proof is cryptographically verified (`status == "ok"`).
   * Contamination metric $C(T) = 0$ (no unverified mutations, unadmitted bypasses, or dead-code masks `#[allow(dead_code)]`).

2. **INV-02 (Hermetic Domain Partitioning)**:
   Dataset splitting into Train (80%), Validation (10%), and Test (10%) MUST occur strictly along **domain and crate boundaries**, never by raw session lines or contiguous tokens. For example, models trained on `cynic-protocol` and `cynic-storage` workflows must be evaluated on unseen `cynic-node` tasks to ensure genuine generalization of constitutional rules.

3. **INV-03 (Soundness over Fluency / Numeric Preservation)**:
   Every generated token sequence must preserve strict literal fidelity for flags, numbers, hashes, and schemas. Any candidate model proposing an unadmitted binary, non-existent CLI flag, or hallucinated hash is scored as a critical safety violation. Auto-correction of numeric tokens or hashes during dataset generation is strictly forbidden.

4. **INV-04 (Hardware-Bounded Sovereign Inference)**:
   Trained model adapters MUST target parameter scales between 0.5B and 3B (e.g. `Qwen2.5-1.5B`, `SmolLM2-1.7B`) and operate within the local memory and compute profiles defined by `cynic-inference` (`HardwareProfile` and `SlotSemaphore`):
   $$M_{\text{VRAM}} \le 4.0\text{ GB}, \quad \text{TTFT} \le 250\text{ ms}$$
   enabling execution on modest edge or node hardware without cloud GPU leasing.

### 2.2 Mathematical Model & Physical Analogy

#### Soundness Metric
Let $M$ be a candidate model evaluated against test dataset $\mathcal{D}_{\text{eval}} = \{(x_i, y_i^*)\}_{i=1}^N$.
The composite Soundness Score $S(M)$ is defined as:
$$S(M) = \frac{1}{N} \sum_{i=1}^N \alpha_i \cdot \beta_i \cdot (1 - \gamma_i)$$
where:
* $\alpha_i \in \{0, 1\}$ is the **Admitted Argv Indicator**: $\alpha_i = 1$ iff every command token in $\hat{y}_i = M(x_i)$ belongs to the registered schema $\mathcal{S}_{\text{admitted}}$.
* $\beta_i \in \{0, 1\}$ is the **Lease Compliance Indicator**: $\beta_i = 1$ iff the workflow preserves the `claim -> mutate -> verify -> release` cycle.
* $\gamma_i \in [0, 1]$ is the **Hallucination Penalty**:
  $$\gamma_i = \min\left(1.0, \, \frac{\text{count}(\text{hallucinated\_hashes}) + \text{count}(\text{unmodelled\_flags})}{2}\right)$$

#### Constitutional DPO Objective
The preference loss $\mathcal{L}_{\text{DPO}}$ aligns the sovereign adapter $\pi_\theta$ against base reference model $\pi_{\text{ref}}$:
$$\mathcal{L}_{\text{DPO}}(\pi_\theta; \pi_{\text{ref}}) = - \mathbb{E}_{(x, y_w, y_l) \sim \mathcal{D}_{\text{pref}}} \left[ \log \sigma \left( \beta \log \frac{\pi_\theta(y_w | x)}{\pi_{\text{ref}}(y_w | x)} - \beta \log \frac{\pi_\theta(y_l | x)}{\pi_{\text{ref}}(y_l | x)} \right) \right]$$
where:
* $y_w$ (winning) is the proof-verified, bounded typed execution sequence.
* $y_l$ (losing) is the unadmitted reflex (e.g. attempting raw unadmitted `cargo build`, bypassing lease claims, or mutating files outside lease).
* $\beta = 0.1$ is the divergence regularization factor.

---

## 3. Technical Design and Schemas

### 3.1 Dataset Record Schema (`cynic.dataset.trace.v1`)

```json
{
  "$schema": "cynic.dataset.trace.v1",
  "trace_id": "trace_hermes_20260921_01",
  "source_agent": "hermes:main",
  "source_session_ref": "session://7d1d0212-de56-4b02-9674-68775f07ab3c",
  "target_domain": "crate:cynic-release",
  "admissibility": {
    "lease_id": "claim_hermes_supply_chain_001",
    "read_back_proof_hash": "sha3-512:9f8a...",
    "constitutional_verdict": "ok"
  },
  "conversations": [
    {
      "role": "system",
      "content": "You are a sovereign CYNIC operator agent. Execute only admitted typed actions behind registered ports. Never invoke arbitrary shell mutations. Always produce read-back proof."
    },
    {
      "role": "user",
      "content": "Extract release_bot from cynic-host into a clean micro-crate cynic-release."
    },
    {
      "role": "assistant",
      "content": "1. Claim scope: cynic agent claim --target crates/cynic-release --claim-type crate\n2. Extract domain logic to crates/cynic-release/src/lib.rs\n3. Add boundary test in crates/cynic-release/tests/architecture_boundary.rs\n4. Validate: cynic dev crate-audit --json"
    }
  ]
}
```

### 3.2 DPO Preference Pair Schema (`cynic.dataset.preference.v1`)

```json
{
  "$schema": "cynic.dataset.preference.v1",
  "pair_id": "pref_hermes_20260921_01",
  "domain": "crate:cynic-pkg",
  "prompt": "We need to check upstream versions for dependencies in cynic-pkg.",
  "chosen": "cynic-pkg exposes UpstreamProbePort with probe_upstream_version_with_port(..., &HttpUpstreamProbe). In tests, use OfflineUpstreamProbe to avoid unadmitted network egress.",
  "rejected": "Run curl -s https://api.github.com/repos/... directly from inside the domain parser."
}
```

### 3.3 Target Rust Domain Types (`cynic-protocol::dataset`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub schema: String,
    pub created_at_unix_s: u64,
    pub total_samples: usize,
    pub train_samples: usize,
    pub val_samples: usize,
    pub test_samples: usize,
    pub domain_partitions: Vec<DomainPartitionSummary>,
    pub contamination_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSample {
    pub trace_id: String,
    pub domain: String,
    pub source_session_ref: String,
    pub admissibility: AdmissibilityProof,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationScorecard {
    pub model_id: String,
    pub evaluated_at_unix_s: u64,
    pub samples_evaluated: usize,
    pub admitted_argv_ratio: f64,
    pub lease_compliance_rate: f64,
    pub hallucination_rate: f64,
    pub soundness_score: f64,
    pub latency_ttft_ms: u64,
    pub passed_eval_gate: bool,
}
```

---

## 4. Top-Down Causal Chain Impact

- **Constituent / Operator Intent**:
  Human operators retain ultimate sovereign authority. Dataset manifest generation requires explicit operator trigger (`cynic dev dataset export --manifest`).
- **Protocol Types & Contracts (`cynic-protocol`)**:
  Pure domain types: `DatasetManifest`, `TraceSample`, `DpoPreferenceRecord`, `EvaluationScorecard`, and `SoundnessMetric`. No I/O, no runtime policy.
- **Log & Trace Intelligence (`cynic-intel`)**:
  Re-uses `cynic-intel::autoresearch::trace_fs` and canonical `bucket_signatures::matching_buckets` to harvest raw session journals and tag execution errors (e.g. ICEs, syntax failures, boundary violations) for automatic negative preference extraction.
- **Packaging & Tooling Authority (`cynic-pkg`)**:
  Manages Python virtual environment and local tooling (`uv`, `huggingface-cli`, model weights cache layouts) through `PrincipalInstallLayout` and `EcosystemPackageExecutionPort` without ambient PATH pollution.
- **Askesis Engine (`cynic-askesis`)**:
  Implementation of the `TraceHarvesterPort` reading append-only session and coordination logs to reconstruct verified action trajectories with SHA3-512 checksums.
- **Inference Runtime (`cynic-inference`)**:
  Local loading of fine-tuned LoRA adapters alongside the base model through `llama.cpp` or `vLLM` adapters, governed by `SlotSemaphore` and `MemoryBudget`.
- **Operator CLI (`cynic-cli`)**:
  Thin operator surfaces:
  * `cynic dev dataset export --output <path> [--json]`
  * `cynic dev dataset validate --manifest <path>`
  * `cynic dev eval run --model <path> --test-suite <suite>`

---

## 5. Blast Radius and Non-Goals

### In-Scope
- Pure extraction of historical Antigravity, Hermes, and Codex session logs.
- Transformation into standard instruction and DPO formats (JSONL).
- Deterministic evaluation harness for verifying model compliance with CYNIC rules.
- Local LoRA adapter fine-tuning using sovereign compute resources.

### Explicit Non-Goals
- Training monolithic 70B+ general intelligence models.
- Scraping external, non-consensual web data.
- Introducing cloud training dependencies into the canonical build gate.
- Allowing models to bypass runtime verification: a fine-tuned model still receives zero ambient authority; all mutations remain gated by admission and read-back proof.

---

## 6. Verification and Evaluation Gates (CYNIC-Eval-Harness)

A candidate specialist model must clear the following deterministic benchmarks prior to runtime admission:
1. **Admitted Argv Ratio**: $\ge 99.5\%$ of proposed actions map directly to canonical CLI surfaces in `infra/surface-registry/manifest.json`.
2. **Lease Compliance Rate**: $100\%$ of mutating workflows initiate with `cynic agent claim` and conclude with `cynic agent release`.
3. **Numeric and Hash Invariance**: $0\%$ hallucinated hashes or fabricated numeric metrics.
4. **Hardware Latency Bound**: $\le 250\text{ ms}$ TTFT (Time-To-First-Token) on admitted node hardware classes ($M_{\text{VRAM}} \le 4.0\text{ GB}$).

---

## 7. Rollout Plan

- **Phase 1 (Protocol & Harvesting)**:
  1. Add pure dataset domain types to `crates/cynic-protocol/src/dataset/`.
  2. Implement `TraceHarvesterPort` in `cynic-askesis` and `cynic-intel::autoresearch::trace_fs`.
  3. Wire `cynic dev dataset export` in `cynic-cli`.
- **Phase 2 (Baseline & Distillation)**:
  1. Harvest 300+ gold verified traces from Antigravity/Hermes history.
  2. Benchmark base `Qwen2.5-1.5B-Instruct` against the test set.
  3. Execute QLoRA fine-tuning using local GPU/builder runner resources, managed by `cynic-pkg` (`uv` + `mlx` or `unsloth` / `llama.cpp`).
- **Phase 3 (Sovereign Node Integration)**:
  1. Wire the fine-tuned adapter into `cynic-inference` as the local subagent engine for routine maintenance and boundary audit tasks.
  2. Verify against `CYNIC-Eval-Harness` and publish proof bundle.
