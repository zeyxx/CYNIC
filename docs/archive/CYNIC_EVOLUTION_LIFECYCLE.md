# CYNIC Evolution Lifecycle

Status: normative foundation.

Machine contract: `axioms/evolution-lifecycle-v1.json`.

This policy defines when a cortex-authored repo mutation can be called a
production evolution. It exists to prevent local edits, wrapper commands, or
partial runtime observations from being mistaken for organism convergence.

## Core Rule

An evolution is valid only when every layer in the chain has evidence:

```text
intent_captured
-> cortex_registered
-> attention_loaded
-> targets_claimed
-> repo_mutated
-> local_gates_passed
-> release_profile_resolved
-> payloads_validated
-> mempool_dispatched
-> organ_claimed_task
-> runner_validating
-> release_validated
-> deploy_dispatched
-> runtime_applied
-> production_observed
-> claims_released
-> closed
```

For non-deploying lifecycle profiles, `release_validated` may transition
directly to `production_observed` when the completed task itself is the runtime
evidence.

## Layer Responsibilities

| Layer | Evidence |
|-------|----------|
| Cortex | registered session, target claims, repo diff |
| Local gates | deterministic test or lint output |
| Registry | resolved build runner and deploy target |
| Kernel | mempool dispatch, claim, completion read-back |
| Organ | lifecycle task execution evidence |
| Runtime | health, service, or production observation |

## Forbidden Shortcuts

- A local shell build is not the canonical supply chain.
- A committed file is not production evidence by itself.
- A wrapper command cannot redefine lifecycle semantics.
- Runtime status cannot be promoted without registry authority and read-back
  proof.
