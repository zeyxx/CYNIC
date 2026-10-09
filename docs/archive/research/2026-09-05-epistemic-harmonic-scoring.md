# Epistemic Harmonic Scoring & The Kenosis Hypothesis

**Mathematical Foundations for Adversarial Agent Evaluation, Weak-Link Sensitivity, and Confidence Ceilings**

**Date:** 2026-09-05  
**Author:** zeyxx · κυνικός Research  
**Status:** Working Paper / Research Protocol  
**Topic:** Formal Epistemology, Multi-Attribute Decision Analysis (MCDA), LLM Calibration, Autonomous Agent Safety  

---

## Abstract

Autoregressive Large Language Models (LLMs) and agentic systems exhibit systematic pathologies: overconfidence, sycophancy, premature convergence, and scope inflation (an inability to refrain from generating unnecessary code or assertions). Standard evaluation metrics rely on additive (arithmetic) or multiplicative (geometric) scoring, both of which allow severe failure on a critical invariant to be compensated for by strength elsewhere.

In this paper, we establish three formal contributions:
1. **The Weak-Link Invariant & Regularized Harmonic Mean ($\mathcal{H}_\epsilon$)**: We prove why the harmonic mean is mathematically superior to geometric and arithmetic means for safety-critical multi-attribute evaluation, deriving its asymptotic sensitivity to single-point failures ($\lim_{x_i \to 0} \frac{\partial \mathcal{H}}{\partial x_i} = \infty$).
2. **The 7-Axiom Epistemic Tensor**: We formalize the evaluation vector $X \in [0, 100]^7$ across Fidelity, Coherence ($\Phi$), Verify, Culture, Burn, Sovereignty, and the emergent axiom **Kenosis** (active restraint against unsolicited action and over-completion).
3. **The $\Phi$-Bounding Hypothesis & Falsification Protocol**: We deconstruct the historical $\phi^{-1} \approx 61.8\%$ confidence ceiling, strip away mystical numerology, and reformulate it as a testable behavioral constraint against softmax overconfidence, defining an empirical protocol with measurable calibration metrics (Expected Calibration Error, Brier Score, and Scope Creep Ratio).

---

## 1. The Pythagorean Means Hierarchy & The Weak-Link Invariant

### 1.1 The Inequality of Means

For any positive vector of scores $X = (x_1, x_2, \dots, x_n) \in \mathbb{R}_{>0}^n$, the classical Pythagorean inequality holds strictly:

$$\min(X) \le \mathcal{H}(X) \le \mathcal{G}(X) \le \mathcal{A}(X) \le \max(X)$$

Where:
- **Arithmetic Mean:** $\mathcal{A}(X) = \frac{1}{n} \sum_{i=1}^n x_i$
- **Geometric Mean:** $\mathcal{G}(X) = \left( \prod_{i=1}^n x_i \right)^{\frac{1}{n}}$
- **Harmonic Mean:** $\mathcal{H}(X) = \frac{n}{\sum_{i=1}^n \frac{1}{x_i}}$

### 1.2 Sensitivity Analysis: The Mathematical Defense of the Maillon Faible

In systems safety, security engineering, and epistemic auditing, a system is bounded by its weakest component (Amdahl's law analog).

Let us compute the partial derivative of each mean with respect to a single score $x_k$:

1. **Arithmetic Mean Sensitivity:**
   $$\frac{\partial \mathcal{A}}{\partial x_k} = \frac{1}{n} = \text{constant}$$
   *Failure mode:* A decrease of 50 points on security is treated identically to a decrease of 50 points on aesthetics. Total substitutability.

2. **Geometric Mean Sensitivity:**
   $$\frac{\partial \mathcal{G}}{\partial x_k} = \frac{\mathcal{G}(X)}{n \cdot x_k}$$
   *Property:* Sensitivity increases as $x_k \to 0$, but remains bounded when other components are large.

3. **Harmonic Mean Sensitivity:**
   $$\frac{\partial \mathcal{H}}{\partial x_k} = \frac{\partial}{\partial x_k} \left( \frac{n}{\sum_{i=1}^n x_i^{-1}} \right) = \frac{n}{\left(\sum_{i=1}^n x_i^{-1}\right)^2} \cdot \frac{1}{x_k^2} = \frac{1}{n} \left( \frac{\mathcal{H}(X)}{x_k} \right)^2$$
   *Theorem:*
   $$\lim_{x_k \to 0^+} \frac{\partial \mathcal{H}}{\partial x_k} = \infty$$

The marginal penalty for weakening an already fragile axis scales inversely with the **square** of that axis. A single vulnerability disproportionately collapses the composite harmonic metric.

### 1.3 Comparative Numerical Demonstration

Consider an autonomous agent that scores $95/100$ on 6 axes (speed, syntax, elegance, etc.), but drops to $5/100$ on an essential safety invariant (e.g., executing unverified code without a mandate):

$$X = (95, 95, 95, 95, 95, 95, 5)$$

| Metric | Computation | Score | Epistemic Verdict |
|---|---|---|---|
| **Arithmetic Mean $\mathcal{A}$** | $\frac{6 \times 95 + 5}{7}$ | **$82.1 / 100$** | **PASS / HOWL** *(Catastrophic masking)* |
| **Geometric Mean $\mathcal{G}$** | $(95^6 \times 5)^{\frac{1}{7}}$ | **$60.7 / 100$** | **PASS / WAG** *(Acceptable rating)* |
| **Harmonic Mean $\mathcal{H}$** | $\frac{7}{\frac{6}{95} + \frac{1}{5}}$ | **$26.6 / 100$** | **REJECT / BARK** *(Immediate failure)* |

**Conclusion:** The Harmonic Mean is the only Pythagorean mean that enforces non-negotiable invariants in adversarial and safety-critical environments.

### 1.4 Regularized Harmonic Mean ($\mathcal{H}_\epsilon$)

To prevent division by zero when $x_i = 0$ in discrete computation, we define the $\epsilon$-regularized harmonic mean:

$$\mathcal{H}_\epsilon(X) = \frac{n}{\sum_{i=1}^n \frac{1}{x_i + \epsilon}} - \epsilon \quad \text{where } \epsilon = 10^{-6}$$

This ensures continuous differentiability across the entire closed domain $[0, 100]^n$.

---

## 2. The 7-Axiom Epistemic Evaluation Tensor

We define the evaluation tensor as an ordered tuple of orthogonal vectors across $L_4 = 7$ axes (where 7 is the 4th Lucas number):

$$X = \big( x_F, x_\Phi, x_V, x_C, x_B, x_S, x_K \big) \in [0, 100]^7$$

```
                                  [7. KENOSIS]
                               (Active Restraint)
                                      ▲
                                      │
           [1. FIDELITY] ─────────────┼───────────── [2. COHERENCE (Φ)]
        (Empirical Grounding)         │            (Structural Harmony)
                                      │
                                      ┼
                                      │
           [6. SOVEREIGNTY] ──────────┼───────────── [3. VERIFY]
         (Human Agency & Bounds)      │             (Popperian Falsification)
                                      │
                                      ▼
                      [5. BURN] ─────────────── [4. CULTURE]
                   (Parsimony / Ockham)       (Historical Lineage)
```

### Axiom Formal Definitions

1. **FIDELITY ($x_F$)**: Degree of observational anchorage:
   $$x_F = 100 \times \frac{|E_{\text{observed}}|}{|E_{\text{observed}}| + |E_{\text{inferred}}| + \omega |E_{\text{fabricated}}|}$$
2. **COHERENCE ($x_\Phi$)**: Internal non-contradiction and invariant symmetry:
   $$x_\Phi = 100 \times \left(1 - \frac{|\text{Contradictions}(T)|}{|\text{Statements}(T)|}\right)$$
3. **VERIFY ($x_V$)**: Empirical falsifiability (Popperian demarcation):
   $$x_V = \begin{cases} 0 & \text{if claim contains no refutable conditions} \\ 100 \times \text{TestabilityRatio} & \text{otherwise} \end{cases}$$
4. **CULTURE ($x_C$)**: Provenance and architectural lineage fidelity:
   $$x_C = 100 \times \left(1 - \text{AmnesiaIndex}\right)$$
5. **BURN ($x_B$)**: Information density and algorithmic parsimony:
   $$x_B = 100 \times \frac{\text{Kolmogorov Complexity } K(S)}{\text{Length of Implementation } |S|}$$
6. **SOVEREIGNTY ($x_S$)**: Protection against unauthorized capture and bounded mandates:
   $$x_S = 100 \times \left(1 - \text{CaptureScore}\right)$$
7. **KENOSIS ($x_K$) — The Emergent Non-Action Invariant**:
   Measures restraint against unsolicited scope inflation:
   $$x_K = 100 \times \left(1 - \frac{|\Delta_{\text{unprompted}}| + |\text{Tokens}_{\text{padding}}|}{|\Delta_{\text{total}}|}\right)$$

If an agent outputs 500 lines of unsolicited scaffolding or sycophantic praise, $x_K \to 0$, driving the entire composite score $\mathcal{H}_\epsilon(X) \to 0$.

---

## 3. Disagreement Entropy & Dempster-Shafer Uncertainty

Subjective single-model judgment is epistemically invalid. To construct ground truth, evaluations are conducted across $M$ independent validator models (e.g., Gemini, Claude, Llama, heuristic analyzers).

### 3.1 Shannon Consensus Entropy

Let $V = \{v_1, v_2, \dots, v_M\}$ be the verdicts returned by $M$ independent validators, discretized into $K$ verdict buckets. Let $p_k$ be the empirical frequency of bucket $k$:

$$H(V) = -\sum_{k=1}^K p_k \log_2(p_k)$$

- **$H(V) = 0$**: Absolute consensus (all validators agree on the verdict).
- **$H(V) = \log_2(K)$**: Maximum dispersion / complete epistemic disagreement.

The effective confidence modifier $\gamma(V)$ scales inversely with consensus entropy:

$$\gamma(V) = 1 - \frac{H(V)}{\log_2(K)} \in [0, 1]$$

### 3.2 Dempster-Shafer Representation of Explicit Ignorance

Standard Bayesian probability forces probabilities over a partition $\Omega$ to sum to 1 ($\sum P(\omega) = 1$), which conflates *"equal evidence"* with *"no evidence"*.

Under Dempster-Shafer evidence theory, mass is assigned to the power set $2^\Omega$:

$$m(\text{Sound}) + m(\text{Unsound}) + m(\Theta) = 1$$

Where:
- $m(\text{Sound})$ represents direct positive proof.
- $m(\text{Unsound})$ represents direct refutation.
- **$m(\Theta)$ represents uncommitted epistemic ignorance** (residual unknowns).

An agent or evaluation that reports high belief with $m(\Theta) \gg 0$ is penalized for epistemic arrogance.

---

## 4. The Kenosis & $\Phi$-Bounding Hypothesis

### 4.1 De-Mystifying $\Phi$: From Numerology to Behavioral Heuristic

Historical CYNIC documentation stated: *"Confidence is capped at $\phi^{-1} \approx 61.8\%$ because the golden ratio generates nature."*

**Epistemic Correction:** The golden ratio $\phi$ has no axiomatic connection to Kolmogorov probability or statistical calibration. However, we formulate the **Kenosis Behavioral Hypothesis**:

> **Hypothesis $H_1$:** Enforcing an explicit confidence ceiling $C \le \phi^{-1} \approx 61.8\%$ and requiring an explicit Kenosis protocol (listing unmade decisions, unwritten code, and refutation criteria) significantly reduces Expected Calibration Error (ECE) and unprompted token inflation in autonomous coding agents.

### 4.2 Why Autoregressive LLMs Over-Generate

Autoregressive transformer models are trained via maximum likelihood estimation (MLE) over next-token prediction with cross-entropy loss:

$$\mathcal{L}_{\text{CE}} = -\sum_{t=1}^T \log P(w_t \mid w_{<t})$$

Combined with RLHF (Reinforcement Learning from Human Feedback), models are optimized to avoid the appearance of ignorance, resulting in:
1. **Sycophancy:** Agreeing with faulty user premises.
2. **Hyper-Verbosity:** Generating verbose explanations rather than succinct assertions.
3. **Premature Action:** Calling tools when information is missing instead of stating uncertainty.

### 4.3 Falsifiable Experimental Protocol

To test $H_1$, we specify a dual-cohort randomized benchmark across 100 complex system engineering tasks:

#### Cohort A (Unbounded Control):
Standard prompt:
```text
Solve the problem. Provide the code and your estimated confidence in the solution (0-100%).
```

#### Cohort B ($\Phi$-Bounded + Kenosis Mandate):
Constrained prompt:
```text
Solve the problem under the Kenosis invariant:
1. State your confidence, capped at a maximum of 61.8%.
2. Formulate the exact empirical condition that would falsify your implementation.
3. Explicitly list what you left unwritten, unbuilt, or unassumed (negative space).
4. Do not output any unsolicited boilerplate or pleasantries.
```

#### Quantitative Evaluation Metrics

1. **Expected Calibration Error (ECE):**
   $$\text{ECE} = \sum_{b=1}^B \frac{|B_b|}{N} \left| \text{acc}(B_b) - \text{conf}(B_b) \right|$$
   *Prediction:* $\text{ECE}_B < \text{ECE}_A$ (Cohort B will be significantly better calibrated against ground-truth unit tests).

2. **Scope Creep Ratio (SCR):**
   $$\text{SCR} = \frac{\text{Unrequested LOC}}{\text{Required LOC}}$$
   *Prediction:* $\text{SCR}_B \to 0$, while $\text{SCR}_A \gg 0.35$.

3. **Brier Score ($BS$):**
   $$BS = \frac{1}{N} \sum_{i=1}^N (f_i - o_i)^2$$
   Where $f_i$ is stated confidence and $o_i \in \{0, 1\}$ is actual test pass/fail outcome.

---

## 5. Summary & Engineering Implementation

| Concept | Naive / Legacy Model | Rigorous Mathematical Model |
|---|---|---|
| **Axiom Count** | 5 axioms | **7 axes ($L_4 = 7$)**, explicitly integrating **SOVEREIGNTY** and **KENOSIS** |
| **Score Aggregation** | Arithmetic / Geometric Mean | **$\epsilon$-Regularized Harmonic Mean $\mathcal{H}_\epsilon$** (infinite weak-link sensitivity) |
| **Confidence Cap** | "Sacred golden ratio bound" | **Behavioral Kenosis Ceiling** to curb softmax overconfidence and MLE hallucination |
| **Multi-Model Consensus** | Averaged scores | **Shannon Consensus Entropy $H(V)$** and **Dempster-Shafer mass $m(\Theta)$** |
| **Ground Truth** | Internal agent conviction | **Empirical Read-Back Proof** ($\text{Observation} \to \text{Proof Journal}$) |

---

## References

1. Amdahl, G. M. (1967). *Validity of the single processor approach to achieving large scale computing capabilities*. AFIPS.
2. Dempster, A. P. (1967). *Upper and lower probabilities induced by a multivalued mapping*. Annals of Mathematical Statistics.
3. Shafer, G. (1976). *A Mathematical Theory of Evidence*. Princeton University Press.
4. Guo, C., Pleiss, G., Sun, Y., & Weinberger, K. Q. (2017). *On Calibration of Modern Neural Networks*. ICML.
5. Popper, K. (1934). *Logik der Forschung*. Springer.
6. Shannon, C. E. (1948). *A Mathematical Theory of Communication*. Bell System Technical Journal.
