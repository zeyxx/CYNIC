# Fleet build sequence — organic tranche plan

## Mother rule

No tranche starts without the previous tranche's data (logs, measurements,
real ACLs). No design on hypotheses.

## Tranche 0 — LXC create (in progress)

- Spec: `docs/design/proxmox-rest-create.md`.
- Prism: *proof before trust* — preflight, UPID receipt, independent
  read-back, idempotent rollback.
- Review: operator on the spec + one second look attacking the 3 open
  questions (ACL preflight, consent shape, auto-rollback).
- Exit: `create` green on 107 + archived receipt JSON.

## Tranche 1 — Clean-template clone

- Entry data: measured create-vs-clone durations on 107, task log.
- Prism: *speed without dirt* — clone only from a versioned template,
  never from a live guest.
- Review: plan diff (create vs clone), proof that no guest state leaks.

## Tranche 2 — QEMU

- Entry data: one named real need (never "just in case") + the LXC
  measurements that block it.
- Prism: *same grammar, other backend* — `create` takes `--kind qemu`,
  the receipt schema does not change.
- Review: the QEMU REST surface audited like LXC, no hidden second path.

## Tranche 3 — Fleet scheduler

- Entry data: observed queues (at least 2 real concurrent workloads),
  defined slots and fair-queuing.
- Prism: *provable fairness* — existing FIFO + SHA dedupe keep working,
  supervised tranches add on top without breaking them.
- Review: mandatory adversarial review (watchdog doc requires it) +
  starvation test (no workload waits forever).

## Transverse review (every tranche)

1. Short spec → 2. attack (operator + one second look, minimum 3 named
   weaknesses) → 3. TDD (RED/GREEN on the nominal path + the refusal
   path) → 4. archived live proof → 5. commit with scope claim.
