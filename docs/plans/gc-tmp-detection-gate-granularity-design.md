# Build GC /tmp detection, gate granularity, compute-01 split — design (doc only)

Branch: `agent/hermes/gc-tmp-detection` from `origin/main` @ `d88f1312`.
Status: DESIGN ONLY. No host mutation, no code change. Answers operator R1/R2/R3 of 2026-10-22.

## 1. How to detect live /tmp job targets (R2 — the liveness rule)

Problem: `/tmp/*target` (23 GB on CT105) has no liveness signal. The checkout GC works because checkouts carry identity (remote, SHA, branch). `/tmp` dirs carry none.

Rule (triple-signal, delete only when ALL THREE are stale — never on one):

1. **Lease file**: whatever sets `CARGO_TARGET_DIR=/tmp/<job>-target` must write `/tmp/<job>-target.cynic-lease` with `{claim_id, pid, dir, heartbeat_unix_s}` and refresh the heartbeat while the job runs (same vocabulary as the builder's existing `heartbeat.jsonl` / lease store — check `build-store.db` and `agent-scheduler` schemas first; do NOT invent a fourth lease format).
2. **PID alive**: the lease pid still exists AND its cmdline still belongs to the claiming job (pid reuse guard).
3. **Mtime fresh**: newest mtime under the dir within the job window.

Deletion requires: lease missing-or-expired (> 24h) AND pid dead-or-foreign AND mtime older than the grace period. Legacy dirs with no lease (all current ones) fall back to signals 2+3 with a 7-day mtime grace.

Naming discipline (no code): all job target dirs MUST match `/tmp/<job>-target` so the GC matches by pattern plus lease. Anything else in `/tmp` is out of scope for the GC, always.

No-code wins to take first (config only, same effect per GB):

- Cap sccache: 13 GB across TWO dirs (`.cache/sccache` 10.5 + `sccache-cache` 2.5) — dedupe to one cache dir and set `SCCACHE_CACHE_SIZE` (e.g. 5G). No code, instant GBs.
- `SystemMaxUse` on builder journal (1.5 GB and growing).

## 2. Gate granularity (R2 — why builds are slow and how to split them)

Measured pain: `cynic audit all` is workspace-all-or-nothing (one blocked crate vetoes 41 green ones); CI runs 4 of 42 crates; the builder (4 overcommitted cores, load 3.5, 75% disk) does everything while compute-01 (16 threads, 10 GB RAM free, 407 GB free) does zero builds — `compute-policy.json` routes `rust_build` to the builder only.

Target shape:

- **Per-crate required checks**: `cynic audit crate --json` already exists (the merge proxy uses it). Make it the per-PR gate over the affected set, not the workspace.
- **Affected set only**: changed crates + their dependents from `cargo metadata`. Test that set. `cargo deny` stays workspace-level but runs once (cacheable), not per crate.
- **Redefine the gate**: merge = affected-set green + deny green. Full-workspace runs become nightly, not per-PR. Nightly failure opens a debt branch; it never blocks unrelated PRs.
- **Split runners by direction**: verification (check, test, deny, audit, docs) is placement-free — it may run anywhere. Release binaries are placement-bound (glibc, see §3) and stay on the builder.

## 3. compute-01 split (R2 — measured enrollment plan)

Observed 2026-10-22 (read-only SSH):

- Debian 13 trixie, glibc **2.41**, 16 threads, 15 GB RAM (10 free), 453 GB disk (407 free). No `cynic*` units, NO cargo/rustc. `/var/lib/cynic/{bin,identity}` and a CSR exist — enrollment started, toolchain never installed.
- glibc direction matters: binaries built on compute-01 (2.41) do NOT run on the builder (2.39) or Debian 12 LXCs (2.36). So compute-01 must NEVER produce deploy binaries for the current fleet (unless musl target — open question). It is ideal for verification: check, per-crate test, deny, audit, docs.
- Enrollment steps (separate branch, operator approval per step): install pinned toolchain (rust-toolchain.toml, 1.95) via rustup; register runner using the builder's `actions-runner` pattern; add `compute-policy.json` task classes (`rust_check`, `rust_test_crate`, `deny`, `audit_crate`) with the existing live-guard floors; prove with ONE crate check before routing anything real.
- Constraint: wifi-only NAT is fine (outbound pulls, inbound SSH from cortex works); never route release deploys through it.

## 4. Policy evolutions approved (R3 — each its own branch, none executed here)

1. Backup: off-sda target mandatory; disk hygiene as pre-backup gate; back up the store IN USE with its real namespace/database (cortex `main`), not the declared one.
2. Build: glibc/musl target declared per `host_family`; GC covers `/tmp` job targets with the §1 liveness rule.
3. Privilege: no credentials in argv fleet-wide; sudoers 0440 + `visudo -c -f` before install; rotation after any exposure.
4. Binaries: every deployed binary traced to a `[[bin]]` + commit SHA; `binary-locations.json` generated from observation.
5. DNS: single registry→zone projection; compat zones have a lifecycle (dual-serve → converged → retired only on `audit.deployed` green); no public fallback for internal names.
6. Done = runtime proof: merged is not complete; `audit.deployed` green on every target is.

## 5. Explicit unknowns

- Exact schema of `build-store.db` / `heartbeat.jsonl` (which lease format to reuse).
- Whether musl static builds are viable for fleet deploys (decides compute-01's ceiling).
- sccache hit rate before capping (cap blindly at 5G may slow builds; measure `sccache --show-stats` first).
- Runner pattern on compute-01 (copy builder `actions-runner` or Forgejo runner? unexamined).
