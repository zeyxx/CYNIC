# CI Sovereignty — Role Split, Runner Pool, and the Single-Writer Rule

Status: `proposed` — governance design. No code, no runner, no token, no network
call is executed by this document. Every item in §5 is an operator TODO.
Schema: `cynic.ci_sovereignty.v1`
Companion: `docs/design/CI_EVOLUTION.md` (suite survivability + the five adoption
metrics, branch `agent/sub/ci-evol`), `docs/design/production-infrastructure.md`
(node roles and observed gaps), `contracts/foundation.json` §evolutive_policies.build
(the build/test gate contract this design implements).
Evidence date: 2026-10-19. `unknown` means unobserved, not absent.
Quarantine note: `docs/` is quarantined historical prose, not runtime authority
(`docs/README.md`). This document is a governance proposal, not law.

---

## 0. Why this document exists

Two facts, both observed 2026-10-19:

1. **The verification pipeline was GitHub-only.** `.github/workflows/ci.yml`
   (origin/main) is the only configured workflow in the tree: one job,
   `runs-on: ubuntu-latest`, four test suites plus JSON-contract validation and
   `cargo check --workspace`. There is no `.forgejo/workflows/` directory on
   `origin/main`.
2. **Forgejo Actions has never run.** Zero runs ever recorded, and no runner is
   registered against the forge. The forge is therefore a repository host with a
   protected-branch surface, not a CI provider — today.

That combination produced the incident that motivates this design: a
`--force` mirror operation against `main` overwrote history that existed on the
GitHub mirror, and a merge that was real on one provider was gone from the other.
A force push to a shared ref is a delete disguised as a write, and the mirror
cannot distinguish "I want a different history" from "I want no history".

The decision taken 2026-10-19 is therefore a **role split**, not a migration:

- **Forgejo is the technical, blocking gate** — the only plane that may stop a
  merge.
- **GitHub is the advisory hygiene gate** — it may report, never block.

This is a sovereignty decision, not a cost one: the forge holds the repository
SSOT (`infra/registry.json` → `source_workspace_layout.vcs_provider.sovereign_ssot.kind:
forgejo`), so the authority that can refuse a merge must be the authority that
owns the repository. A merge gate enforced by a third-party host is a gate the
host can change under you.

---

## 1. Role split — Forgejo blocking, GitHub advisory

### 1.1 Forgejo — technical blocking gate

The forge is the merge plane. It owns the repository and the branch protection
rule. Its verdict is the only one that can refuse a merge.

| # | Gate | Suite / command | Blocking? |
|---|---|---|---|
| F1 | Contract integrity | `jq empty` over `contracts/foundation.json`, `infra/runtime-authority.json`, `infra/registry.json`, `infra/agent-exposure.json`, `infra/ecosystem-pins.json` | **Yes** |
| F2 | Workspace compilation | `cargo check --workspace` (pinned toolchain 1.95.0, `CARGO_INCREMENTAL=0`) | **Yes** |
| F3 | Full test suites | per-crate `cargo test` for the `CORE_CRATES` set: `cynic-protocol`, `cynic-host`, `cynic-node`, `cynic-kernel`, `cynic-cli`, `cynic-inference`, `cynic-mcp` | **Yes** |
| F4 | Unified audit | `cynic audit all --json` (`make audit`) | **Yes** |
| F5 | Architecture ratchets | `make check.ratchets` (orphans, crate size ceilings, legacy debt) | **Yes** |
| F6 | Runtime divergence | `make audit.runtime` (exit-code gated on Critical findings) | **Yes** |

Rules:

- **One aggregate verdict per run.** F1–F6 run to completion in a single job and
  a final `if: always()` step is the only authority for the job's outcome. This
  is the aggregate-verdict discipline specified in `CI_EVOLUTION.md` §2; it is
  retained here unchanged, and its `outcome`-not-`conclusion` rule (§2.1) is
  binding.
- **Fail-closed on absence.** A gate that never executed is a failure, not a
  pass. The aggregate fails the run when a hard gate (F1, F2) did not succeed,
  because a broken hard gate means the suites did not all run.
- **The check context is the job's `name:`, not its `id:`.** Branch protection
  matches the display name exactly. Renaming the job silently orphans the rule
  and every PR sits blocked with a green pipeline. Any rename must update
  `status_check_contexts` **in the same change**.
- **No auto-retry.** A retry is a decision. Every retry is a data point in the
  flaky-rate metric (`CI_EVOLUTION.md` §3.3) and an automatic retry would hide
  the flake it measures.

### 1.2 GitHub — advisory hygiene gate

GitHub keeps a workflow. Its job is to report signal that is not technical
blocking power, and it must be structurally incapable of blocking a merge.

| # | Gate | Suite / command | Blocking? |
|---|---|---|---|
| G1 | Format hygiene | `cargo fmt --all -- --check` | No — report only |
| G2 | Docs link integrity | link check over `docs/**` (relative links resolve; dead links reported) | No |
| G3 | Secret scan | `make secret.literals` (rejects high-confidence literal credentials in tracked source) | No — report only, **never** a merge requirement |
| G4 | Public signal | open-source surface: README, licence presence, contribution note, public CI badge | No |

Rules:

- **GitHub check contexts are never added to the Forgejo protection rule.** The
  required context list contains exactly one entry — the Forgejo blocking job's
  `name:`. A hygiene context in that list makes a formatter veto a merge, which
  is the exact inversion this design removes.
- **GitHub may be disabled, rate-limited, or unavailable without consequence to
  mergeability.** A merge must be possible with GitHub fully down.
- **The GitHub job is not renamed to look like a gate.** It carries no `required`
  status semantics anywhere in the forge.
- **G3 is advisory because a secret scan is a hygiene signal, not a correctness
  proof.** A clean scan does not mean "no secret exists"; a red scan means
  "operator attention required" — the operator, not the merge queue, decides.
  Secret authority stays with `infra/runtime-authority.json` §secret_authorities.

### 1.3 The relationship to the existing workflow

`.github/workflows/ci.yml` today carries the *technical* suites (types,
knowledge, inference, canonical paths) under a blocking-shaped job. Under this
design those suites move to the Forgejo blocking gate (F3) and the GitHub job
is reduced to G1–G4. The migration is ordered so the blocking capability
exists on the forge **before** the GitHub job is demoted: a blocking gate that
is demoted before its replacement is live is an unguarded window.

---

## 2. Runner pool design

### 2.1 Labels

Forgejo Actions matches jobs to runners by label strings in `runs-on:`. The
label set below is the whole contract between a workflow and the pool; a runner
is a set of labels, a job is a set of required labels.

| Label | Meaning | Runner tier |
|---|---|---|
| `cynic/linux` | Linux x86-64, `cargo` on PATH, network allowed for crates.io | minimum |
| `cynic/stack-8g` | `RUST_MIN_STACK=8589934592` is safe on this host (test builds) | ≥ 32 GiB RAM class |
| `cynic/stack-4g` | 4 GiB stack safe (kernel-tier suites) | ≥ 16 GiB RAM class |
| `cynic/build` | Build SSOT — builder-bound, writes the canonical target dir | builder only |
| `cynic/win11` | Windows 11 x86-64, `cargo` on PATH | laptops only |
| `cynic/advisory` | Light, no compiler required | any host |

Rules:

- **A job may require at most one of `cynic/build`, `cynic/linux`, `cynic/win11`** —
  these are the host-class labels and they are mutually exclusive. The stack
  labels compose on top.
- **`cynic/build` is registered on exactly one host** (builder `10.10.10.119`,
  role `build_runner`). It is the build SSOT and it is not load-balanced; a
  second `cynic/build` runner would create two authorities for one verdict.
- **`cynic/stack-8g` is not a suggestion.** Test builds monomorphise far harder
  than check builds; the Makefile already encodes this as `TEST_RUST_ENV`
  (`RUST_MIN_STACK=8589934592`, `CARGO_INCREMENTAL=0`). A test job scheduled on
  a runner that cannot hold 8 GiB of stack fails as an OOM/SIGSEGV, not as a
  test failure — which is exactly the class of "flake" the flaky-rate metric
  must never be able to manufacture. Runner RAM class is a gate input, not a
  tuning knob.
- **Toolchain is pinned, not assumed.** The root `rust-toolchain.toml` pins
  1.95.0; `infra/rust-toolchain.json` records the per-crate `min_stack` and the
  observation that the six per-crate `crates/*/rust-toolchain.toml` files are
  inert for a workspace build. A runner whose live `rustc --version` disagrees
  with the pin is not a valid `cynic/linux` runner — observe before admitting.

### 2.2 Fair queuing across agents

Forgejo/Gitea Actions has no native cross-workflow fair queue: a runner takes
one job at a time and a workflow is either running or queued. The fairness
policy is therefore explicit and mechanical, not emergent.

1. **One job per runner at a time.** No runner config runs concurrent jobs.
   Queueing is the forge's; fairness is the workflow's.
2. **Never cancel an in-flight run on a new push.** Every blocking job sets
   `concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: false }`.
   Cancelling a run mid-suite destroys the evidence that run was producing and
   lets a later push mask an earlier failure. Queued is slower than cancelled,
   and slower is the price of a trustworthy record.
3. **`fail-fast: false` on every matrix.** One leg's failure must not cancel the
   other legs — the aggregate verdict exists precisely because a partial run is
   the failure mode to avoid (`CI_EVOLUTION.md` §1).
4. **Dedicated labels for exclusivity, shared labels for throughput.** An agent
   that must not contend registers its own runner with `cynic/agent-<id>` plus
   the host-class label; a job that may contend uses the shared class label. A
   dedicated runner is a reservation, not a faster lane.
5. **Queue-depth watchdog.** A scheduled observation reports, per host-class
   label, the number of queued jobs older than a threshold. A queue that grows
   monotonically is a capacity signal, not a correctness problem — it is
   reported, never auto-remediated by adding runners (§5 owns capacity).
6. **Ordering is not guaranteed and must not be assumed.** "Fair" here means
   "no starvation by design and no evidence destruction by cancellation", not
   "first-in-first-out".

### 2.3 Which suite runs where

| Suite / gate | Host class | Required labels | Stack | Blocking |
|---|---|---|---|---|
| F1 contract integrity | any Linux | `cynic/linux` | 256 MiB | Forgejo |
| F2 `cargo check --workspace` | builder | `cynic/build` `cynic/linux` | 256 MiB | Forgejo |
| F3 test `cynic-protocol` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-host` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-node` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-cli` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-mcp` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-inference` | Linux pool | `cynic/linux` `cynic/stack-8g` | 8 GiB | Forgejo |
| F3 test `cynic-kernel` | Linux pool | `cynic/linux` `cynic/stack-4g` | 4 GiB | Forgejo |
| F4 `make audit` | builder | `cynic/build` | 256 MiB | Forgejo |
| F5 `make check.ratchets` | Linux pool | `cynic/linux` | 256 MiB | Forgejo |
| F6 `make audit.runtime` | builder | `cynic/build` | 256 MiB | Forgejo |
| G1 `cargo fmt --check` | hosted | GitHub `ubuntu-latest` | n/a | never |
| G2 docs links | hosted | GitHub `ubuntu-latest` | n/a | never |
| G3 `make secret.literals` | hosted | GitHub `ubuntu-latest` | n/a | never |

Notes that keep the table honest:

- **`cynic-inference` is a candidate, not a proven, suite runner.** Its
  `min_stack` is 8 GiB and it is `status: extracting` in
  `infra/runtime-authority.json` §experimental_workspace_crates. It runs in the
  pool only after a measured green run on a real runner; until then its cell
  reads "unknown", and an unmeasured suite is not a blocking gate.
- **F2/F4/F6 are builder-bound because they are the build SSOT verdicts.** The
  builder is `cynic-builder` (`10.10.10.119`, `lxc/105`, `role: build_runner`,
  `energy_profile: burst`, `deploy_target: true`). It is the only host allowed to
  write the canonical target (`build-targets/bootstrap`) and the only host whose
  check result is authoritative (`AGENTS.md` — builder SSOT).
- **`cortex` is not a runner.** `AGENTS.md` forbids running `cargo`/`rustc`
  locally on cortex (faulty RAM). The registry lists `cynic-host-cortex`
  (`192.168.100.2`, `role: cortex_host`, `status: candidate`, `deploy_target:
  false`) as an operator host, and §5 assigns it **no** runner role. A RAM-faulty
  host produces corrupt builds that read as test failures — the same
  evidence-destruction class as a cancelled run.
- **`cynic/win11` is a check leg, not a full suite.** The Windows laptops are
  `energy_profile: portable_intermittent` and diskless. They carry a
  cross-platform `cargo check` leg only, until a full suite is measured green
  there. A platform leg that has never run green is a claim, not a gate.
- **Parallelism is the repository's value, not the runner's.** `jobs = 2` in
  `.cargo/config.toml` is measured on the builder (56–61 % reduction, peak RSS
  +7.6 %, zero swaps, four clean runs) but **not measured on the canonical
  gate** (`infra/build-policy.json` §parallelism). The canonical path keeps
  `cargo_build_jobs: 1` until that measurement exists; a runner that silently
  inherits a different value makes the gate's verdict non-reproducible.

---

## 3. Single-writer rule for `main`

### 3.1 The rule

> **`main` has exactly one writer at a time, and that writer is the Forgejo
> protected-branch merge. Every other ref is a reader of `main` history.**

Concretely:

1. **No direct push to `main`.** Every change arrives through a pull request
   whose head is a feature branch.
2. **The merge is executed on the forge, under the protected-branch rule**, with
   the Forgejo blocking job (§1.1) as the single required context. The rule is
   what makes the gate real: a gate that is a workflow but not a protection rule
   is a recommendation.
3. **After the forge merge, `main` is back-synced to the mirror** — GitHub
   receives `main`, it does not originate it. The back-sync is a
   **fast-forward, non-force** push.
4. **The mirror is never force-pushed to `main`.** `--force` and
   `--force-with-lease` against `main` on the mirror are prohibited. A divergence
   that requires a force push is a divergence that must be resolved on the forge
   (the writer) and re-published, never overwritten.

### 3.2 Why the incident is not repeatable under this rule

The wipe happened because two writers held the same ref. Under the rule above:

- The mirror cannot rewrite `main`, so it cannot delete a merge that the forge
  has already accepted.
- The forge is the only writer, so the merge's history is the canonical history.
- A mirror that falls behind is a stale reader, and staleness is detectable
  (§3.4) — it is never silently reconciled.

The prohibition is asymmetric on purpose: a **fast-forward back-sync is safe
because it can only add commits the forge already accepted**, and a force push is
unsafe because it can remove them. The rule permits the operation with a
provable safety property and forbids the one without.

### 3.3 Merge procedure

```
1. feature branch pushed to origin (forgejo), PR opened against main
2. Forgejo blocking job runs on the head SHA  →  green
3. merge on the forge, under the protection rule
4. read back: origin/main SHA == forge-reported merge commit
5. back-sync: non-force push of the forge merge commit to the github mirror
6. read back: github/main SHA == forge merge commit
```

Step 4 and step 6 are read-backs, not assertions. A merge that is not read back
on both providers is a merge whose state is unknown on one of them.

### 3.4 Divergence watchdog — specification

A watchdog that only reports is safe; a watchdog that repairs is a second writer.
This one observes and alerts.

**Definition.** For each provider `P ∈ {forgejo, github}`, let `S_P` be the SHA
of `main` as read from `P`'s remote-tracking ref.

**Signals:**

| Signal | Formula | Meaning |
|---|---|---|
| `divergence_ahead` | `S_forgejo` is not an ancestor of `S_github` **or** vice-versa | the two `main`s have different histories |
| `divergence_depth` | `git rev-list --count S_forgejo..S_github` (and the reverse) | how far the mirror lags or leads |
| `staleness_seconds` | `now - max(committerdate(S_forgejo), committerdate(S_github))` | how long since `main` moved |
| `watchdog_lag_seconds` | `now - last_watchdog_run` | a silent watchdog is an unknown, not a pass |

**Thresholds (initial; operator-tunable):**

- `divergence_ahead == true` → **page**. Divergence is never a tolerated state.
- `divergence_depth > 0` on the mirror (mirror behind) for `staleness_seconds >
  300` → **alert**. A short lag is a back-sync in flight, not a fault.
- `watchdog_lag_seconds > 2 × interval` → **alert**. Silence is data.
- `divergence_depth > 0` with the mirror **ahead** of the forge → **page
  immediately**. A mirror ahead of the writer is the incident's signature.

**Collection.** Read-only, no credentials beyond the existing remotes' read
access, no push, no repair:

```bash
git fetch --no-tags origin main          # forgejo
git fetch --no-tags github  main         # mirror
git rev-parse origin/main github/main
git merge-base --is-ancestor origin/main github/main; echo "forgejo_is_ancestor_of_mirror=$?"
git merge-base --is-ancestor github/main origin/main; echo "mirror_is_ancestor_of_forgejo=$?"
git rev-list --count origin/main..github/main
```

**Owner and cadence.** CI steward, every 15 minutes, with the alert path named in
the run report. The watchdog writes to the evidence journal, never to a ref.

**Explicit non-goals.** The watchdog does not push, does not force, does not
merge, and does not open or close PRs. It has no write capability by
construction: if it can only read and append to a log, it cannot become a
second writer.

---

## 4. Authority and honesty boundaries

- **This document is prose.** It is not a contract in `contracts/foundation.json`
  and not a runtime surface in `infra/runtime-authority.json`. Where it and the
  runtime disagree, the runtime wins. Its value is that it makes the intended
  split explicit before the workflows encode it.
- **Nothing here is measured on a runner that does not exist.** Every suite-row
  in §2.3 is a *routing decision*, not a result. No suite is claimed green
  anywhere in this document.
- **The five adoption metrics are not implemented.** They are specified in
  `CI_EVOLUTION.md` §3 and remain operator queries until a collection surface
  exists. No metric is quoted here because no run has produced one.
- **The blocking gate is not live.** Forgejo Actions has zero runs and no
  registered runner as of 2026-10-19. Until §5 is executed, the only configured
  workflow is the GitHub one, and §1.3's ordering rule applies: the forge gate
  is live before the GitHub job is demoted.

---

## 5. Provisioning checklist — operator TODO, not executed

**Nothing in this section has been performed.** Each item is an operator action
with the exact read-back that proves it. An unchecked item means the
corresponding capability is `unknown`.

### 5.1 Builder — `cynic-builder` (`10.10.10.119`, `lxc/105`)

- [ ] Register a Forgejo Actions runner on the builder with labels
      `cynic/build`, `cynic/linux`.
- [ ] Confirm the runner's live toolchain: `rustc --version` → must equal the
      root pin **1.95.0** (`rust-toolchain.toml`).
- [ ] Confirm `RUST_MIN_STACK` handling on the runner: the 256 MiB default for
      check jobs, 8 GiB for test jobs, sourced from the Makefile's `RUST_ENV` /
      `TEST_RUST_ENV` — never from `.cargo/config.toml`, which deliberately does
      not set it.
- [ ] Confirm the canonical target dir is the builder's
      `build-targets/bootstrap` and that `CARGO_TARGET_DIR` is passed explicitly
      (`infra/registry.json` → `build_lifecycle.canonical_target_relative_path`).
- [ ] Confirm free space above the 4 GiB minimum
      (`build_lifecycle.minimum_available_bytes`).
- [ ] Read-back: `cynic build proof` on a known commit returns a verdict bound to
      that commit SHA.

### 5.2 Cortex — `cynic-host-cortex` (`192.168.100.2`)

- [ ] **Assign no runner role.** Cortex is an operator host with observed
      RAM faults; `AGENTS.md` forbids `cargo`/`rustc` there.
- [ ] Record the decision in `infra/registry.json` (`local-builder` is currently
      a `stub` with no role) so the absence of a runner is an explicit state,
      not an oversight.
- [ ] Read-back: `infra/registry.json` shows no node on cortex with
      `role: build_runner` or any `cynic/*` label.

### 5.3 Windows 11 laptops — future runners

- [ ] Confirm the second laptop. Only **one** laptop is in the registry
      (`cynic-laptop-ax16pro`, `192.168.0.88`, `role: workstation_portable`,
      `status: observed`, `deploy_target: false`, `energy_profile:
      portable_intermittent`). The second diskless Win11 laptop is **unobserved**
      — its existence, address, and hardware are `unknown`.
- [ ] Install a Forgejo Actions runner on each laptop with labels `cynic/win11`
      and `cynic/linux`.
- [ ] Confirm the runner works diskless across reboot: a runner whose working
      directory does not survive a power cycle cannot hold a cache and will
      rebuild cold on every run.
- [ ] Measure a full `cargo test` leg on one laptop before labelling it
      `cynic/stack-8g`; a cold 8 GiB-stack test build on a 2-core i5 is
      unmeasured.
- [ ] Read-back: a `cargo check -p cynic-cli` leg green on `cynic/win11`, with
      the duration recorded in the evidence journal.

### 5.4 Forge — `forgejo` (`10.10.10.101`, `lxc/101`)

- [ ] Create the `.forgejo/workflows/` directory (absent on `origin/main`) with
      the blocking workflow encoding F1–F6 and the aggregate verdict.
- [ ] Configure branch protection on `main`: `enable_push: all` is **not**
      required — the rule must require exactly one status context, the blocking
      job's `name:`, and must not list any GitHub hygiene context.
- [ ] Read-back: `GET /repos/{owner}/{repo}/branch_protections/main` shows
      `enable_status_check: true` with the blocking context present and no
      GitHub context.
- [ ] Confirm the mirror remote's refspec cannot force `main`: the GitHub relay
      token (`infra/registry.json` → `vcs_provider.relay.token_env:
      GITHUB_TOKEN`) must be scoped **without** force-push rights on `main`.

### 5.5 Divergence watchdog

- [ ] Deploy the §3.4 collector on a 15-minute schedule.
- [ ] Wire the alert path (page on `divergence_ahead`, alert on stale mirror).
- [ ] Confirm the watchdog has no write credential to any remote.
- [ ] Read-back: a deliberate one-commit lag on the mirror produces exactly one
      alert and **zero** pushes.

---

## 6. Honest limits

- **Zero Forgejo runs means zero observed behaviour.** Every claim about label
  matching, queueing, concurrency groups, and check-context naming is derived
  from the provider's documented semantics, not from a run on this forge. The
  first real run is the first real observation.
- **Runner RAM classes are inferred from host hardware, not measured on the
  runners.** `cynic/stack-8g` means "this host's RAM class makes an 8 GiB stack
  safe", which is a hardware statement, not a runner configuration statement.
  The only proof is a green 8 GiB-stack test build on that host.
- **The mirror's pre-incident state is not reconstructed here.** This document
  records the mechanism (a force push over a shared ref) and the rule that makes
  it impossible, not a forensic history of which commits were lost. That history
  belongs in the evidence journal with its own provenance.
- **`cynic-inference` and the Windows legs are explicitly unproven.** Both are
  routed in §2.3 and explicitly marked as not-yet-gates.
- **This document is quarantined prose** (`docs/README.md`) and carries no
  runtime authority until a workflow or a contract encodes it.
