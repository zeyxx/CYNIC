# B19 — a native command for the layout-aware remote deploy

- **Status**: Proposed. Not implemented. Recorded 2026-10-26.
- **Work graph**: `infra/work-graph.json` nodes B17, B18, B19, B20, B22 and hyperedge H10.
- **Related**: `docs/specs/NATIVE_NODE_DEPLOYMENT_SPEC.md` (the signed, SSH-free pipeline; a longer road).

## 1. Problem

A fix merged on `main` does not reach a runner. No CYNIC tool replaces the
`cynic` program on a machine.

Measured 2026-10-25:

| Machine | `cynic` git sha | Layout |
|---|---|---|
| cortex | `7b9834ff` | flat (`~/.local/bin`) |
| builder | `65578621` | symlink (`releases/<tag>` + `current`) |
| cynic-compute-01 | `1126afec` | flat (`/var/lib/cynic/bin`, `.prev-<sha>` backups) |

`main` was far ahead of all three. `make release.deploy` and `cynic source deploy`
assume the symlink layout and would convert a flat machine without anyone
deciding it.

## 2. What already exists (library, no caller)

- `cynic-release/src/deploy_locations.rs` — `DeployMachine`, `DeployLayout`
  (symlink or flat), backup pattern, read-back commands, per machine, loaded from
  `infra/deploy-locations.json`.
- `cynic-release/src/deploy_orchestrator.rs` — `deploy_to_machine`,
  `deploy_across`: stage, read the live sha256, **mandatory backup**, swap, read
  back identity and smoke, with a typed `GateFailure` and `StepOutcome`.
- `cynic-release/src/deploy_remote.rs` — the `RemoteExecutorPort` and the pure
  command builders.
- `cynic-release/src/deploy_ssh.rs` — `SshRemoteExecutor` and
  `LocalRemoteExecutor`.

A search of `crates/` finds no call site of `deploy_to_machine` or
`deploy_across` outside the module. The capability is complete enough to test
and is not reachable.

## 3. The change (B19)

One CLI subcommand that drives `deploy_to_machine`:

- Inputs: a machine (a `registry_node_id`), the built binary path, its commit.
- **Default: plan only.** It prints what it would do on that machine: resolved
  deploy path, layout, backup name, read-backs. It mutates nothing.
- **`--apply`** executes. This matches the repository rule that mutation needs an
  explicit flag.
- Output: the `StepOutcome` / remote deploy report as JSON, with the observed
  sha256, the baked git sha and the smoke exit code. The report is the evidence.
- Refuses when the target's `in_flight` is not zero, when the commit does not
  match the binary's baked sha, or when the layout is not declared.

Where to put it: next to `cynic source deploy`, which is the local variant.

## 4. Scope and non-scope

In scope: replacing the **`cynic` CLI binary** on one machine, atomically, with a
backup and a read-back.

Not in scope — these are B20:

- Replacing the **`cynic-node` daemon**. It is not in the deploy model (one
  `deploy_path` per machine), and it is the program that serves `/capacity`.
- **Restarting** that daemon. On compute-01 the only sudo grant of the `cynic`
  user is `cynic privilege exec node-restart`; builder and cortex are unverified.

So B19 alone does **not** make `/capacity` report disk on compute-01. It lays
the base that B20 extends.

## 5. Transport

The existing executors cross SSH. That is the only transport this code knows.
B19 does not remove SSH; it makes the SSH use typed, bounded and read back
instead of ad hoc. The SSH-free road is the signed pipeline in the node
deployment spec.

## 6. Proof plan

1. Unit tests with the fake `RemoteExecutorPort` already used by the module:
   plan output, refusal cases, a failing read-back that triggers the rollback
   path, the flat and symlink layouts.
2. `cynic build submit --action check` on the builder for the compile.
3. A live run on **cynic-compute-01 only**, plan first, then `--apply`, then the
   read-backs; the previous binary is kept as `cynic.prev-<sha>`.

## 7. Blocked today

- Step 1 and 2 need a build. Builds were stopped by the operator on 2026-10-26
  after the builder was saturated (load ~24, a test run cancelled at the 880 s
  cap). A PR written now cannot be proven.
- `cynic-release` tests do not run natively: `cargo test` stops at the first
  failing binary elsewhere in the workspace.

## 8. Risks

- A swap of an executable on a production host. The mandatory backup and the
  read-back are the only rollback; both must be exercised on compute-01 first.
- A wrong layout declaration in `deploy-locations.json` makes the command act on
  the wrong shape. The command must refuse an undeclared or stale layout; the
  measured values in that file date from 2026-10-19 and are partly stale.
