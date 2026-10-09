# CYNIC Build Lifecycle — Desired State (proposal, validated 2026-09-04)

Operator machines never build. Cortex is an operator: it triggers through
`cynic build` surfaces, builders execute, proofs decide. Every stage below
names its owner and its read-back. A stage without read-back is not done.

## Stages

1. Intent (human or admitted agent): branch + commit + phase + package scope.
   Owner: operator. Read-back: exact 40-hex SHA exists on Forgejo SSOT.
2. Admission: SHA validity, idempotency key = SHA, attempt counter.
   Owner: trigger path. Read-back: `AdmissionBlocked` before any clone on
   invalid input; replay of retained proof on duplicate SHA, no rebuild.
3. Placement: runner resolved from the fleet registry (`build_runner` role),
   never a hardcoded IP. Explicit `--host` is breakglass only.
   Owner: `resolve_build_runner_transport`. Read-back: alias + trust gates.
4. Execution: per-SHA checkout, `HEAD == SHA` verified, canonical cargo argv
   (offline honored), bounded jobs. Owner: builder node.
   Read-back: exit status + tail stderr in the proof bundle.
5. Proof: `cynic.pipeline_build_proof.v1` with per-layer status.
   Converged only for the layers the promotion gate names.
6. Promotion (merge): human pushes, bot verifies. Fast-forward only, on a
   freshly proven tip. Conflicts stop the bot and page a human.
   Rollback is revert-forward, never history rewrite.
7. Delivery: artifact SHA must equal the proven commit. Mismatch on arrival
   means refuse + quarantine, never install. Rollout order: builder, then
   agent-host, then cortex. Rollback keeps the previous binary, human only.

## Validated decisions

- D1 retry: no automatic retry. Every retry is a new admitted trigger with
  an attempt counter. Revisit after measurement.
- D2 runner identity: transport alias from the registry. No IP literals in
  CLI defaults, unit files, or docs.
- D3 merge: human pushes, bot verifies. No autonomous push until manual
  merges are proven first.
- D4 strategy: fast-forward only. Agents prepare rebases, humans admit.
- D5 green bar for merge: `check` + the slice's own tests. Out-of-scope
  debt is named in the report and does not block.
- D6 canary order: builder, agent-host, cortex.
- D7 rollback: human only.

## Edge cases retained

- Webhook down: Forgejo retry + visible queue, no silent polling fallback.
- Builder busy or double push: SHA-idempotent replay, no duplicate builds.
- Partial green: layers decide, not a global boolean.
- Node offline at delivery: queue + replay on reconnect with receipts.
- Partition: local work continues, global waits, everything replays proof.

## Explicit unknowns

- BuildStore (SQLite idempotence) presence on the current node code: unverified.
- Queue fairness across branches on one builder: undesigned.
- Merge bot shape (checks-only): undesigned. P2 remains the largest hole.
- Freshness of the deployed webhook unit after each CLI change: manual step.
