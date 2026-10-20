# Operational drift killchains — builder 10.10.10.119

Status: traced from live state (2026-10-20) and from the code that produced it.
Each killchain names the condition that allowed the drift, the evidence that
the condition existed, and the correction that removes it.

Fact basis (measured, not assumed):

| Fact | Value | Source |
|---|---|---|
| Disk before correction | 92 % (15 GiB free) | `df -h /` |
| Disk after correction | 62 % (66 GiB free) | `df -h /` |
| `checkouts/` | 46.4 GiB, 26 entries, 25 orphans | `cynic runtime build artefact status --json` |
| `target/` share of checkouts | 45.0 GiB of 46.4 GiB | `du -x -s` |
| `shared-target/` | 32.7 GiB | `du -x -s` |
| `.cache/sccache` | 10.4 GiB, unbounded | `du -x -s`, `sccache --show-stats` |
| `/tmp` | 18.2 GiB | `du -x -s` |
| Reclaimed by native GC | 42.0 GiB, 26 → 3 entries | `cynic runtime build artefact gc --apply` |
| sccache requests | 26 579, 14 813 hits (71.7 %) | `sccache --show-stats` |
| sccache non-cacheable | 5 361 (`crate-type`) | `sccache --show-stats` |

---

## Killchain 1 — one checkout per SHA, never reclaimed

**Condition.** `SshBuildRunner::trigger` clones a checkout named
`{checkout_base}/cynic-{sha[..12]}` for every commit it builds, and never
removes it. The checkout is keyed by SHA, so a new SHA is a new directory by
construction, and nothing in the trigger path deletes the previous one.

**Evidence.**

- `crates/cynic-node/src/ports/build_runner_adapter.rs:171-173` — `checkout_dir` derives the path from the SHA.
- `crates/cynic-node/src/ports/build_runner_adapter.rs:261-290` — the clone step. No removal counterpart exists in the file: `grep -n '"rm"' build_runner_adapter.rs` returns nothing.
- Observed: 26 checkouts, 25 of them orphans (`remote_exists: false`), 46.4 GiB.

**Why it was allowed.** The retention mechanism that should have covered it
(`cynic-protocol::retention`, `cynic runtime build artefact gc`) was never
wired into the build path. It was a command an operator had to know to run.

**Correction.** The pre-trigger disk gate: read the runner's available bytes
before cloning, and refuse with an explicit `blocked_disk_full` instead of
materializing another checkout on a disk that cannot hold it. The policy
already states this — `infra/runner-retention.json` → `preflight.min_free_bytes`
— but the field was never read by any code path.

---

## Killchain 2 — the policy named an owner, nobody was the owner

**Condition.** `infra/runner-retention.json` declares the enforcement owner as
undecided: *"Enforcement owner undecided: runner-side cron vs pre-trigger gate
in SshBuildRunner vs CLI surface. This file is policy; the enforcer gets its
own slice with proof."* The file dates from the 2026-09-08 ENOSPC incident
(23 GiB) and repeats it: same disk, same failure mode, 6 weeks later.

**Evidence.**

- `infra/runner-retention.json:18-20` — the residual unknown, still open.
- No systemd timer on the builder: `systemctl list-timers --all | grep cynic` returns nothing.
- `grep -rn 'min_free_bytes' crates/` returns no implementation.

**Correction.** Pick one owner and make it observable. A timer-driven
retention pass and a pre-trigger gate are not interchangeable: the timer
reacts after the fact, the gate prevents the condition. Both are needed, and
the gate is the one that removes the killchain.

---

## Killchain 3 — sccache was installed without a bound

**Condition.** `RUSTC_WRAPPER=sccache` is set on the runner and the cache is
the single biggest source of cache hits — 71.7 % global. Nothing ever sets
`SCCACHE_CACHE_SIZE`, and no sccache config file exists, so the cache can
only grow. It reached 10.4 GiB.

**Evidence.**

- `systemctl --user cat forgejo-runner.service` → `Environment=CYNIC_STATE_DIR=/var/lib/cynic` only; no sccache bound.
- `/var/lib/cynic/.config/sccache/` does not exist.
- `grep -rn 'SCCACHE_CACHE_SIZE' crates/` → no hit.

**Correction.** Set an explicit cache size bound where the runner is
provisioned, so the cache stays a cache and never becomes a second disk
problem. The cache is the solution; an unbounded cache is a new killchain.

---

## Killchain 4 — two Cargo cache roots, one invisible to the observer

**Condition.** Two conventions coexist for the same thing:
`state_root/shared-target` (written by `SshBuildRunner`, 32.7 GiB) and
`state_root/build-targets/<profile>` (written by the scheduler and the CI
workflow, 7.3 GiB). The disk observer only knows the second one, so 32.7 GiB
of real cache is invisible to the operator's disk view.

**Evidence.**

- `crates/cynic-node/src/ports/build_runner_adapter.rs:151-153` — `default_shared_target_dir` → `shared-target`.
- `crates/cynic-host/src/adapters/local_disk.rs:228-243` — the observer reads `build-targets/<profile>` only.
- `crates/cynic-host/src/adapters/local_disk.rs:205-209` — the comment claims the legacy path is "the only one ever proposed for cleanup", but `shared-target` is neither legacy nor observed.
- `grep -rn 'shared-target' crates/` → 3 call sites, none of them the disk observer.

**Correction.** One Cargo cache root, declared in one place, observed by the
disk scanner. The registry already declares the canonical target per node
(`build_lifecycle.canonical_target_relative_path`) — the code must use the
declared path instead of hardcoding a second one.

---

## Killchain 5 — the retention engine was opt-in and never opted in

**Condition.** The whole engine exists and is correct: `reconcile_build_cache`
decides `ReclaimEphemeral` / `ReclaimCanonicalProfile` / `Blocked` /
`Satisfied`, and `reconcile_build_cache_resources` executes it with
`--apply`. But `ephemeral_eligible` returns an empty list when
`retention_max_age_ms` and `retention_pressure_grace_ms` are absent, and
`infra/registry.json` does not declare `ephemeral_retention` for
`cynic-builder`. The checkouts were therefore never eligible, by a decision
that was made once, in a registry file, and never revisited.

**Evidence.**

- `crates/cynic-host/src/domain/disk/build_runner/policy.rs:233-242` — the `else { return Vec::new(); }` early exit.
- `infra/registry.json` → `cynic-builder` → `build_lifecycle` has no `ephemeral_retention` key (verified live on the builder).
- `crates/cynic-host/src/domain/disk/build_runner/reconcile.rs:10-29` — the file's own doc comment describes this exact failure mode: *"A declared step with no executor is a policy that cannot be obeyed, which reads to the operator exactly like a policy that is being obeyed."*

**Correction.** Declare `ephemeral_retention` for the build runner with an
explicit age cap and a pressure grace, so the existing engine governs the
checkouts. The engine is the fix; only the declaration was missing.

---

## Killchain 6 — the reclaim command was a manual step, not a scheduled one

**Condition.** `cynic build reclaim --apply` executes the full engine
correctly, and `cynic runtime build artefact gc --apply` executes the
retention trio. Both are manual. Nothing invokes them, so the drift
reproduces on every build cycle and only stops when an operator remembers.

**Evidence.** `grep -rn 'reconcile_build_cache_resources' crates/` → only
the two CLI call sites. No timer, no build-path hook, no cron.

**Correction.** A scheduled retention pass on every runner, with the disk
read-back as the receipt. A manual command is a mitigation, not a control.

---

## Killchain 7 — the declared canonical target does not exist on disk

**Condition.** The registry declares
`build_lifecycle.canonical_target_relative_path = "build-targets/bootstrap"`
for `cynic-builder`, and the reclaim engine reclaims exactly that profile
under pressure. The real cache lives in `build-targets/fgate` and
`shared-target`. So the policy that is supposed to relieve disk pressure
reclaims a directory that does not exist, while the directories that do
exist are outside the policy.

**Evidence.**

- `infra/registry.json` → `cynic-builder` → `build_lifecycle.canonical_target_relative_path` = `"build-targets/bootstrap"`.
- On the builder: `build-targets/` contains `fgate/`, not `bootstrap/`.
- `crates/cynic-host/src/domain/disk/build_runner/policy.rs:92` — the reclaim target is derived from that declared path.

**Correction.** Make the declared path and the real path the same path, and
add a read-back that fails loudly when they diverge. A policy pointing at a
directory that does not exist is worse than no policy: it looks satisfied.

---

## Killchain 8 — the non-canonical candidates point at nothing

**Condition.** `noncanonical_cache_candidates` declares
`{temporary_directory, cynic-cargo-targets}` and
`{state_root, source/repositories/CYNIC/target}`. Neither path exists on the
builder. The engine refuses to reclaim the canonical profile while a
non-canonical cache is present — and since the declared candidates are
absent, the refusal never triggers, so the canonical reclaim fires against a
path that does not exist (Killchain 7) while the real caches stay
unmanaged.

**Evidence.** Verified live: neither candidate path exists under
`/var/lib/cynic` or `/tmp`.

**Correction.** Point the candidates at the real stray caches
(`shared-target`, the non-canonical `build-targets` profiles), or delete the
section. A candidate list that names nothing is a false sense of coverage.

---

## Killchain 9 — a production daemon runs a binary out of the build cache

**Condition.** The soak daemon
(`/etc/systemd/system/cynic-supervise-soak-node.service`) executes
`/var/lib/cynic/shared-target/debug/cynic supervise run ...`. The binary it
runs lives inside a Cargo target directory.

**Evidence.**

- The unit's `ExecStart` points at `shared-target/debug/cynic`.
- The process is live: PID 2901553, started 2026-10-16, 6654 minutes of CPU.
- `infra/directory-tree.json` declares the canonical home of the binaries as
  `bin/` with symlinks into `current/` — a target dir is not a binary home.

**Why it matters.** The retention policy that reclaims build caches and the
daemon that runs from one are the same tree. Any purge of `shared-target`
kills a running system service, and the policy does not know it: the
`protected` list in `contracts/builder-retention.json` names
`/var/lib/cynic/source` and `/var/lib/cynic/sccache`, not the directory a
daemon is executing from.

**Correction.** A daemon never executes from a reclaimable cache. Its binary
belongs under the declared binary root (`bin/` → `current/`), and the
retention policy must refuse to reclaim any directory that a live unit
executes from. The general rule is stronger than this one daemon: the
retention decision reads the unit files, not a hand-maintained protected
list.

**Status.** The fence is in place and proved by test
(`crates/cynic-host/src/adapters/local_build_cache.rs`,
`noncanonical_reclaim_never_removes_a_cache_a_live_unit_executes_from`): the
reclaim reads the unit files and skips the path, reporting it as
`skipped_live_service` — distinct from `skipped_absent`, so an operator
reading the evidence knows which happened. The second half is not done: the
soak binary still executes from the cache, because the installed
`/usr/local/bin/cynic` (0.8.0) does not know the `supervise` verb. The binary
must be rebuilt and reinstalled before the `ExecStart` can move.

---

## Killchain 11 — a running service defined only by the host it runs on

**Condition.** The two soak units existed on builder-119 and nowhere else.
`/etc/systemd/system/cynic-supervise-soak-node.service` and
`cynic-supervise-watch-soak-node.service` had no counterpart in the
repository, and their `ExecStart` comments referenced
`docs/design/supervision-convergence-map.md` and
`docs/design/supervision-watch-pattern.md` — two documents that are not in
the repository either.

**Evidence.**

- `git ls-files | grep -i soak` returns nothing.
- The units were installed by hand on 2026-10-16; the file timestamps and
  the state directory both start that day.
- The two referenced design documents do not exist in `docs/design/`.

**Why it matters.** A unit file on one host is the only record of a running
process. It cannot be audited against the code, it cannot be reinstalled
after a rebuild, and its references point at nothing. When the reclaim
policy reads the unit files to decide what is safe to remove, an unversioned
unit is an unversioned input to a safety decision.

**Correction.** The units are versioned under
`infra/os/systemd/soak/`, with a README stating what they are (a durability
bench, not a production role), why they are not in `installed/` (the bootstrap
bundle must not install a 30-day crash-rate bench on a production node), and
how to install them. The comment referencing a missing document is corrected
to point at the README.

---

## Killchain 12 — a build replaced a running service's binary in place

**Condition.** The soak units execute
`/var/lib/cynic/shared-target/debug/cynic`. That path is a Cargo target
directory, so every build that writes to it rewrites the binary a running
system service depends on — in place, under the service's feet, with no
restart and no coordination.

**Evidence.** The chain is fully dated:

| When | Fact | Source |
|---|---|---|
| 2026-10-16 02:25 | The watcher starts and keeps running | `ps -o lstart= -p 2901553` |
| 2026-10-19 00:57 | The parent's last successful observation: `state: active`, `restart_count: 0` | `node.status.json` mtime and content |
| 2026-10-20 16:44 | The binary at that path is rewritten | `stat -c %y /var/lib/cynic/shared-target/debug/cynic` |
| now | The parent is `failed` with `NRestarts=4` | `systemctl show` |

The replacement binary does not know the verb the unit asks for:
`cynic supervise --help` on the current file answers
`error: unrecognized subcommand 'supervise'`. The watcher survives only
because its image was already loaded into memory on 16 October; the parent,
which systemd restarts, cannot start at all.

**Why it matters.** This is the end state of Killchain 9. The fence added for
Killchain 9 stops the *reclaim* from removing the path — but it does nothing
about a *build* overwriting the file inside it. The path was treated as a
disposable cache by the build system and as an installation directory by
systemd, and no artifact in the system recorded the contradiction.

**Correction.** A service never executes from a build output directory. The
soak's `ExecStart` must point at a binary installed under the declared
binary root, installed atomically, and never overwritten in place by a
build. The build system and systemd must agree on one directory per role:
`infra/directory-tree.json` already declares it (`bin/` → `current/`), and
the unit does not use it.

The blocker to the move is now identified and it is not the fence: the
`supervise` verb does not exist in the CLI of this repository at all
(`grep -rn "supervise" crates/cynic-cli/src/` finds only unrelated
supervision fields), so the installed `cynic` cannot serve the soak whether
it is installed at `/usr/local/bin` or in the cache. The bench cannot run
until the verb it exercises is in the tree.

---

## Hygiene defects found in the same pass

| Defect | Evidence | Correction |
|---|---|---|
| Ten historical Cargo cache profiles in `build-targets/` (14 GiB) | `ls /var/lib/cynic/build-targets` | One profile per declared profile; the rest are reclaimable |
| 1.53 GiB of root-owned files no unprivileged process can remove | `rm: Permission denied` on `agent-hermes-aegis-consortium` | The runner writes what it owns; the leftovers are a bootstrap-ordering defect |
| `/tmp` holding 18.2 GiB of build state | `du -x -s /tmp` | `builder_tmp_ttl` already declares a 2-day TTL for `/tmp/cynic-*`; the 7 other directories are undeclared and must be named or removed |
| sccache cache and the Cargo cache both under the state root, unbounded | see Killchains 3 and 4 | One bounded, declared layout |

---

## What the corrections are not

- Not a bigger disk. The drift was unbounded growth, not insufficient
  capacity: the 46.4 GiB of checkouts held 45.0 GiB of `target/` — reproducible
  build cache, one copy per SHA, for 25 SHAs that are no longer reachable.
- Not disabling sccache. It is the main reason a warm build is 96 s instead
  of 194 s. The problem was that it was unbounded and that the proc-macro and
  binary crates it cannot cache (5 361 requests) were recompiled in 18–19
  separate executions.
- Not a shell script. The retention engine, the GC command and the reclaim
  command already existed; the corrections are declarations and wiring, so
  the enforcement stays in CYNIC and observable through it.
