# RFC-001: Magnetic Agent Coordination and Thermodynamic Phase Transitions

- **Status**: Draft
- **Author**: Titouan Corneloup (zeyxx) / Hermes Agent
- **Date**: 2026-09-10
- **Target Crates**: `cynic-protocol`, `cynic-kernel`
- **Related Requirements**: Foundation §481 (`one_boundary_one_reason`), §489 (anti-centralization), `cynic.agent_claims.v2`

---

## 1. Context and Problem Statement

Empirical observation of agent sessions and `.agent-claims.json` reveals systemic coordination frictions:
1. **Macroscopic Locking**: Active claims lock entire crates (`cynic-host`, `cynic-kernel`, `cynic-protocol`), creating an artificial exclusion zone that paralyzes concurrent agent development across disjoint modules.
2. **Remanent Ghost Locks**: Claims whose wall time exceeds their declared TTL by multiples (`temporal.ttl_seconds: 1800` vs `consumed.wall_seconds: 12000`) remain marked as `status: active`, generating permanent false-positive repulsion.
3. **Context-Stuffing Overhead**: Parent agents delegate to children by dumping verbose conversation history, flooding context windows and driving child agents past their coherence limit into hallucination loops.
4. **Thermal Dissipation in Git**: Analysis of recent history demonstrates a high churn ratio (fixes, dead-code purges, and convergence conflicts outweighing net forward progress), indicating excessive thermal dissipation before crystallization.

---

## 2. Formal Invariants

1. **INV-01 (Monotonic Delegation)**: A parent agent may only delegate a strict subset of its own budget (actions, tokens, wall-clock time) and capabilities. $\sum \text{budget}(\text{children}) \le \text{budget}(\text{parent})$.
2. **INV-02 (Topological Exclusion)**: Two agents may never simultaneously hold a write claim on overlapping scopes where topological distance $d_{ij} \le 1$ (same file or immediate module scope).
3. **INV-03 (Passive Demagnetization)**: Any claim whose logical or wall time exceeds $t_{\text{started}} + t_{\text{ttl}}$ without an active heartbeat is inert. It exerts zero repulsive force without requiring explicit operator revocation.
4. **INV-04 (Vector Alignment without History)**: A child agent receives only an orientation vector $\vec{B}$, the constraint tensor, the output schema, and the bounded budget. It never inherits the parent's conversational chat context.
5. **INV-05 (Phase Separation)**: An agent in the *Exploration* phase ($T > T_c$) cannot perform worktree mutations. Mutations require transition to the *Crystallization* phase ($T \rightarrow 0$), where strict exclusion locks are verified.

---

## 3. Mathematical & Physical Formulation

### 3.1 Topological Distance in Code Space ($d_{ij}$)

Let $S_i$ and $S_j$ be the targeted scopes of agents $i$ and $j$ in the project tree:
- $d(S_i, S_j) = 0$ if scopes target the exact same file or overlapping lines.
- $d(S_i, S_j) = 1$ if scopes target different files within the exact same module directory.
- $d(S_i, S_j) = 2$ if scopes target different modules within the same crate.
- $d(S_i, S_j) = 3$ if scopes target different crates within the workspace.
- $d(S_i, S_j) = \infty$ if scopes are fully disjoint (e.g., contracts vs documentation vs independent crates).

### 3.2 Repulsive Potential ($U_{\text{repulsion}}$)

The repulsive energy between two concurrent claims is given by:
$$U(d_{ij}) = \begin{cases} 
\infty & \text{if } d_{ij} \le 1 \text{ and either agent is mutating (Crystallized)} \\
\frac{k_{\text{rep}}}{d_{ij}^2} & \text{if } 1 < d_{ij} < 3 \\
0 & \text{if } d_{ij} \ge 3 \text{ or both agents are in Exploration (Read-Only)}
\end{cases}$$

When an agent plans an action, the planner computes the gradient $-\nabla U$. If the gradient indicates an exclusion barrier, the planner automatically pivots to the nearest orthogonal subtask.

### 3.3 Parent Field Induction ($\vec{B}_{\text{parent}}$)

Instead of propagating conversational history, the parent induces a magnetic field defined by the 4-tuple:
$$\vec{B} = \langle \vec{v}_{\text{intent}}, \mathbf{C}_{\text{invariants}}, \mathcal{S}_{\text{output}}, \mathcal{E}_{\text{budget}} \rangle$$
- $\vec{v}_{\text{intent}}$: Compact, unambiguous objective string (e.g., `"Extract parser module from cynic-host"`).
- $\mathbf{C}_{\text{invariants}}$: Set of boolean constraints (e.g., `{no_external_dependencies, 100_percent_english, zero_franglais}`).
- $\mathcal{S}_{\text{output}}$: JSON schema / typed struct defining the completion payload.
- $\mathcal{E}_{\text{budget}}$: Strictly clamped scalar allocation $\langle \text{actions}_{\max}, \text{time}_{\max} \rangle$.

### 3.4 Thermodynamic Phases & Curie Point ($T_c$)

1. **High-$T$ Exploration Phase**:
   - High entropy allowed.
   - Broad read-only operations across crates.
   - Repulsive potential deactivated ($U = 0$).
2. **Low-$T$ Crystallization Phase**:
   - Temperature clamped to 0.
   - Spin locked along $\vec{B}$.
   - Hard topological exclusion ($U = \infty$ for $d \le 1$).
3. **Curie Point ($T_c$) & Fatigue Detection**:
   - Ratio of useful work vs thermal dissipation:
     $$\eta = \frac{\Delta \text{ValidEvidence}}{\Delta \text{TokensConsumed} \times \Delta \text{Actions}}$$
   - When $\eta < \eta_{\min}$ or consecutive failed read-backs cross a threshold, the agent passes its Curie Point: magnetic alignment collapses.
   - Remediation: Immediate reaping of the child process. No context repair attempt. The parent reclaims remaining budget and spawns a fresh zero-entropy child.

---

## 4. Blast Radius and Non-Goals

- **In-Scope**:
  - Updating claim predicates in `cynic-protocol` to support module-level granularity (`Scope::Module`).
  - Implementing mathematical decay for expired claims (read-time demagnetization).
  - Designing pure types for parent vector induction (`AgentFieldEnvelope`).
- **Non-Goals**:
  - Building a centralized global scheduling daemon (violates §489).
  - Managing OS-level kernel threads or CPU affinities.
  - Adding persistent runtime databases for temporary exploration traces.

---

## 5. Verification and Property Tests

1. **Test-01 (Topological Exclusion)**: Verify that two claims on disjoint modules of `cynic-protocol` produce valid co-existence ($d=2$), while claims on the same file fail admission ($d=0$).
2. **Test-02 (Passive Demagnetization)**: Verify that an active claim whose timestamp is older than `started_at + ttl` is judged inert by pure evaluation predicates without mutation.
3. **Test-03 (Monotonic Budget Invariant)**: Verify that an attempt by a parent with 20 remaining actions to allocate 25 actions to a child envelope is rejected at construction time.

---

## 6. Migration and Rollout Plan

- **Step 1**: Register `specs/RFC-001-MAGNETIC-COORDINATION.md` in repository documentation index.
- **Step 2**: Patch `crates/cynic-protocol/src/coordination_lease/` and `agent_coordination.rs` with `Scope::Module` and passive demagnetization logic.
- **Step 3**: Clean up stale claims in `.agent-claims.json` using the new predicate logic.
