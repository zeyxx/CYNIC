# AMM Pool <-> CYNIC Bounded Surface Mapping

Date: 2026-08-18
Trigger: Hayden Adams (Uniswap founder) fading pools.trade launchpad launch.
Insight: the coordination primitive is not the launchpad, it's the invariant.

---

## The Uniswap invariant

```text
x * y = k
```

A swap transforms the reserves along the curve. The protocol does not
declare a price; the ratio of reserves reveals it. The invariant is
enforced by the math, not by a controller.

## The CYNIC invariant

```text
desired state -> observe -> reconcile -> bounded action -> read-back proof
                   ~ retain only explicit, machine-readable evidence ~
```

Every bounded surface in CYNIC follows this shape. The action is
admissible only if it satisfies the contract and proves it afterward.
The proof is not a claim; it is a read-back.

---

## Mapping

| AMM pool (Uniswap)            | CYNIC bounded surface                                  |
|-------------------------------|--------------------------------------------------------|
| `x * y = k` conservation law  | `cynic.coherence_pattern` per AGENTS.md § Coherence   |
| Permissionless pool creation  | `cynic_coord_register` / typed registry admission      |
| No market-maker oracle        | Truth flows upward from read-back, not declared        |
| Price = ratio of reserves     | Metrics emerge from observed + reconciled state        |
| LP enters with bounded deposit| Delegation envelope with monotonic budget <= parent    |
| LP exits with same rule       | Completion envelope with explicit unknowns upward      |
| Impermanent loss (residual)   | `residual_unknowns` budget, never fabricated as convergence |
| Settlement on-chain           | Solana commit for explicit disclosed public matter     |
| `dy = (y * dx) / (x + dx)`    | `action_budget_consumption` under `aggregate_sibling_budget_accounting` |

---

## Runtime verification sites (read-back proof)

The mapping is only as strong as the invariants actually enforced at runtime.
Verify these exist and are tested:

1. **Monotonic budgets** — a child delegation can never amplify parent authority or budget.
   - `crates/cynic-protocol/src/agent_workload/capability_status.rs::REQUIRED_INVARIANTS`
     includes `"monotonic_capabilities_and_budgets"` and `"aggregate_sibling_budget_accounting"`.
   - Test: `tests::unknown_capability_cannot_amplify_runtime_authority` — an unknown
     capability cannot activate the runtime (no silent amplification).

2. **Truth-upward, intent-downward** — desired state flows down from registries
     and requests; liveness, health, and proof flow upward only from read-back.
   - `crates/cynic-protocol/src/framework.rs` § propagation rule.
   - `crates/cynic-protocol/src/convergence.rs` — `residual_unknowns` lists
     every unobserved concern; never collapsed into a fake "converged" status.

3. **Residual unknown budget** — missing proof is an explicit `unknown`, not
     a silent allow.
   - `crates/cynic-node/tests/peer_proof_transport.rs` line 113:
     `// § "Unknown is data" — missing gate = residual unknown, never silent allow.`
   - `crates/cynic-protocol/src/convergence.rs::reproduce_convergence_report` pushes
     `ConvergenceStatus::Unknown` keys into `residual_unknowns`.

4. **Bounded action with read-back** — no mutation without `--yes` / explicit apply.
   - `crates/cynic-cli/tests/installed_runtime.rs` line 469:
     `contains("without --yes")` — start command refuses silent mutation.

5. **CPMM on-chain** — the literal `k = x * y` invariant lives in
     `programs/cynic-anchor/src/lib.rs`:
     `create_futarchy_market`, `place_futarchy_bet` with `dy = (y*dx)/(x+dx)`.
     This is CYNIC's Solana surface implementing the same curve math that
     Hayden proved in production on Ethereum.

---

## What "fading" misses

The launchpad is a UI over the pool primitive. Fading it bets against the
thesis that a permissionless coordination surface with an auditable
invariant is the durable layer. CYNIC stakes the same claim for agent
orchestration: the invariant is not a product, it is the contract.
A product built on a provably sound invariant survives the product cycle
that fades it.

---

## Coherence scores

Score : 5 = implémenté + testé ; 3 = implémenté mais preuve manquante ; 1 = absent.

| # | Surface                         | desired | observe | reconcile | bounded action | read-back | tests | Score |
|---|---------------------------------|---------|---------|-----------|----------------|-----------|-------|-------|
| 1 | Storage capabilities            | registry | HTTP probe + process argv | status + matrix + risks | mutates:false, command exposed | residual_unknowns | policy.rs tests | 5 |
| 2 | Toolchain lifecycle             | ToolchainRegistry + ProofHistory | ToolchainObservation (version, ICE, cache) | reconcile_toolchain_full → state machine | ToolchainTransition { SetToolchain, Halt, PromoteFallback, Noop } | ToolchainProof + history append + residual_risk | toolchain_lifecycle.rs ICE cycle | 5 |
| 3 | Agent recursion                 | AgentDelegationEnvelope | parent state + child request | admit_child_delegation → admission | admit/reject + violations | AgentCompletionEnvelope { consumed_budget } | agent_recursion/tests.rs budget enforcement | 5 |
| 4 | Update lifecycle                | target phase (lab→prod) | gate checks, ops findings | repair_apply / promote / gate | --yes gate, mutates flag | UpdateLifecycleState { last_gate_ok, last_promote_at } | integration tests | 5 |
| 5 | CPMM Futarchy (on-chain)        | market creation | pool state (yes/no units) | dy = (y*dx)/(x+dx) | place_bet / resolve / burn | on-chain settlement | anchor tests | 5 |
| 6 | Runtime convergence             | all capabilities proven | runtime evidence | status per capability | activate / inactive | residual_unknowns list | convergence tests | 5 |
| 7 | Inference backend               | model config | BackendStatus 5-state | health check + latency | route/failover (untested) | InferResponse { tokens } (no residual_unknowns) | model_process_pool test only | **3** |

---

## Violations / dette

1. **Inference: no residual_unknowns** — `InferResponse` ne porte pas de champ `unknowns`. Si le modèle est chargé mais la latence est inconnue, il n'y a pas de champ explicite pour le dire.
2. **Inference: bounded action untested** — le route/failover derrière `BackendStatus` n'a pas de test qui prouve que l'action est bornée (ex: pas de retry infini sans budget).
3. **Inference: is_available inline** — `BackendStatus::is_available()` est un `matches!` inline, pas une policy pure testée en isolation.
4. **Storage: redact_endpoint inline** — utilitaire derrière un port ? Non, inline dans policy (mineur).
5. **Storage: parse_version untested** — parseur de version sans test unitaire dédié.
6. **Domain tasks: drain action** — `domain_task_queue.rs` mentionne `drain` mais pas de test de monotonic budget pour le drain lui-même.
