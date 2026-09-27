# CLI Surface Audit — build graph, orphan taxonomy, canonical surface

- **schema**: `cynic.audit.cli_surface.v1`
- **status**: `read_only_audit`
- **method**: observation only. One file claim taken to write this report. No production file mutated.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` > `infra/runtime-authority.json` > `infra/registry.json` > workspace `Cargo.toml` > canonical crates > `infra/agent-exposure.json`
- **proof substrate**: the 4 merge conflicts in `crates/cynic-cli/tests/architecture_boundary.rs` were resolved first, so this audit rests on 34 tests that actually execute. Before that resolution, none of them had run since `983b63b6`.

## 0. Prior findings this audit revises

`infra/INFERENCE_SURFACE_AUDIT.md` §4.1 reported "thirteen further CLI source files are orphans".
The measured figure is **62 files, 11 044 LOC**. The earlier count considered only top-level
files. The correction is material: the uncompiled surface is an order of magnitude larger than
previously recorded, and it is organised into complete subsystems rather than scattered files.

`INFERENCE_SURFACE_AUDIT.md` also cited `runtime_surface/inference_surface/local_runtime.rs`
as a source of success-shaped reports. That file **is** compiled. Two sibling modules of the
same directory are not: `inference_surface/fleet.rs` and `inference_surface/host_disk_capacity.rs`.

## 1. Method, and two failed attempts worth recording

The measurement is a Rust module-graph resolution of `crates/cynic-cli/src`. The resolver is
naive text parsing of `mod X;` and is **not** a substitute for `cargo`. It is used because
`cargo` cannot report what it did not compile.

Two earlier versions of the resolver produced **false results** and were discarded:

| version | defect | false output |
|---|---|---|
| 1 | compared module *names* against file *paths*; every nested file counted as orphan | 134 orphans, 21 580 LOC |
| 2 | for a file module (`foo.rs`) returned no child directory, so descent stopped at `lib.rs` | 122 compiled, 194 orphans |
| 3 (accepted) | file-module children resolve to `d/foo/` when that directory exists — this repo uses the **2015** layout (`runtime_surface.rs` + `runtime_surface/`) while declaring edition 2021 | 62 orphans, 11 044 LOC |

Version 3 is accepted only because it passes an identity check and 14 known facts:

```
non-test .rs files : 194
COMPILED files     : 132
ORPHAN files       : 62  (11 044 LOC)
identity           : 132 + 62 = 194            OK
DANGLING mod decls : 0                         OK
```

Known facts asserted: `cli_args.rs`, `runtime_surface/disk.rs`, `cli_exit.rs`,
`surface_registry/assembler.rs`, `network_surface/base.rs`, `source_surface.rs`,
`chat_surface/runtime.rs` are compiled; `lifecycle_surface.rs`, `surface_audit.rs`,
`dev_surface.rs`, `bootstrap_surface.rs`, `fleet_surface.rs`,
`lifecycle_surface_render.rs`, `integrations_surface_render.rs` are not. 14/14.

**0 dangling `mod` declarations** is itself a finding: no source file references a module that
does not exist. The orphans are not broken references. They are whole files that no `mod`
declaration names.

## 2. The build graph

| | files | LOC |
|---|---|---|
| non-test `.rs` under `src/` | 194 | — |
| reachable from `lib.rs` / `main.rs` | 132 | — |
| **orphaned, not compiled** | **62** | **11 044** |

`src/lib.rs` declares 64 modules. `src/main.rs` is 10 lines and declares none — the library-first
facade is real, and it is also why the assertion target of 7 tests is wrong (§4).

`crates/cynic-cli/src/lib.rs:1` carries a crate-wide `#![allow(..., dead_code)]`, so the compiler
cannot object to any of this. The 62 files are not warned about, not denied, not reported.

## 3. Orphan taxonomy

Orphaned files grouped by owning domain. "entry" is the file that would have to be declared.

| domain | files | LOC | entry | nature |
|---|---|---|---|---|
| `chat_tui` | 11 | 3 387 | `chat_tui/mod.rs` (compiled) | **complete subsystem, replaced by a stub** (§5) |
| `dev_surface` | 5 | 1 289 | `dev_surface.rs` | complete domain |
| `bootstrap_surface` | 7 | 1 025 | `bootstrap_surface.rs` | complete domain |
| `runtime_surface` | 12 | 826 | submodules of a compiled parent | partial domain |
| `inference_benchmark` | 1 | 731 | `inference_benchmark.rs` | orphan module, no domain |
| `surface_audit` | 5 | 649 | `surface_audit.rs` | **the surface auditor itself** |
| `kernel_boundary_audit` | 1 | 621 | `kernel_boundary_audit.rs` | orphan auditor |
| `fleet_observation` / `fleet_report` / `fleet_build_runner` / `fleet_surface` | 4 | 525 | `fleet_surface.rs` | complete domain |
| `lifecycle_surface` + `_render` + `_execute` + `_context` | 4 | 553 | `lifecycle_surface.rs` | complete domain, internally consistent |
| `hermes_kanban_adapter` / `hermes_runtime_surface` | 2 | 273 | — | orphan adapters |
| `autoresearch_surface` | 1 | 185 | `autoresearch_surface/mod.rs` (compiled) | partial |
| `lab_surface` | 1 | 170 | `lab_surface.rs` | complete |
| `integrations_surface` + `_render` + `_readonly` | 3 | 292 | `integrations_surface.rs` | complete |
| `proxmox` | 2 | 149 | `proxmox/mod.rs` | complete |
| `surface_registry/assembler/tests.rs` | 1 | 291 | `surface_registry.rs` (compiled) | `#[cfg(test)]`, reached only when the parent is |
| `chat_surface/tests.rs` | 1 | 45 | `chat_surface.rs` (compiled) | `#[cfg(test)]` |
| `agent_harness_adapter` | 1 | 33 | — | orphan adapter |

Two properties distinguish these from dead accretion:

1. **They compile as written.** `lifecycle_surface.rs:18-23` imports
   `crate::lifecycle_surface_{context,execute,render}`; all three files exist and export the
   imported items. The domain is internally consistent and would build.
2. **Their absence is silent.** Nothing fails. `cynic lifecycle`, `cynic dev`,
   `cynic bootstrap`, `cynic fleet` are absent from `--help` because the code that would
   implement them was never declared, not because it was never written.

## 4. The 15 red tests, classified by evidence

`cargo test -p cynic-cli --test architecture_boundary` → **19 passed, 15 failed**. The failures
are the first observation this suite has produced. Each is classified by what the test asserts
and what the build graph contains — not by plausibility.

### 4.1 Stale assertion target — 6 tests, zero real drift

These assert that `src/main.rs` declares a module. `main.rs` is 10 lines and declares none;
library-first moved every declaration into `src/lib.rs`. The architecture each test wants to
protect is in fact intact.

| test | asserts `main.rs` contains | actually declared at |
|---|---|---|
| `cli_runtime_surface_is_split_out_of_main` | `mod runtime_surface;` | `lib.rs:67` |
| `lifecycle_surface_uses_shared_exit_policy` | `mod cli_exit;` | `lib.rs:23` |
| `runtime_and_dev_surfaces_share_file_json_writer` | `mod fs_json;` | `lib.rs:43` |
| `deps_and_lifecycle_use_shared_json_emitter` | `mod cli_json;` `mod fs_json;` | `lib.rs` |
| `deps_surface_is_split_into_thin_cli_modules` | deps surface modules | `lib.rs` |
| `audits_share_rust_source_discovery_helpers` | `mod rust_source;` | `lib.rs` |

Repair is a retarget to `lib.rs`. Relaxing or deleting them would destroy a real guard.

### 4.2 Real drift, correctly detected — 5 tests

| test | orphan that fails it |
|---|---|
| `integrations_surface_delegates_rendering_and_shared_json` | `integrations_surface_render.rs` |
| `lifecycle_surface_delegates_human_rendering` | `lifecycle_surface_render.rs` |
| `lifecycle_surface_delegates_command_execution` | `lifecycle_surface_execute.rs` |
| `lifecycle_surface_uses_shared_context_helpers` | `lifecycle_surface_context.rs` |
| `host_bootstrap_e2e_emits_non_mutating_runtime_contract` | `bootstrap_surface/host_e2e.rs` |

The tests were written against a `main.rs` that once held these declarations. They are correct
about intent, stale about location, **and** the thing they protect is genuinely unwired. Both
halves must be fixed: retarget to `lib.rs`, then wire the module.

### 4.3 Hardcoded count — 1 test

`cli_audit_crate_verification_passes` expects `report.crate_count == 45`, observes 47. The
workspace grew by `cynic-node-protocol` and `cynic-inference-runtime`. The literal is the
defect: a crate-count ratchet must be derived from the workspace, not pinned, or it produces a
red that must be silenced by editing a number.

### 4.4 Registry disagreement — 2 tests, partially unexplained

`commands_manifest_exposes_canonical_surface_boundaries` fails with
`missing canonical surface portable_lab`. `portable_lab` **is** declared in
`infra/surface-registry/manifest.json` and has a fragment at
`infra/surface-registry/fragments/portable_lab.json`. The assembler in
`surface_registry/assembler.rs` is compiled, and it fails to surface an entry that exists on
disk. The registry the code assembles disagrees with the registry the repository holds.

`surface_registry_is_split_into_catalog_and_types` asserts the catalog loads canonical data from
`infra` and that the facade stays thin. Not yet reduced to a single cause.

### 4.5 Unclassified — 1 test

`agent_exposure_audit_is_split_into_scan_and_check_modules` asserts that
`agent_exposure_audit.rs` contains `mod checks; mod json_scan; mod source_scan; mod types;`.
`agent_exposure_audit` and all four submodules **are** compiled, so the first assertion should
hold and the failure must come from a later assertion in the same test body. Reported as
`unknown`; not investigated.

## 5. `chat_tui` — a working 3 387 LOC subsystem behind a 17-line stub

The most consequential orphan set, and the only one where compiled code deliberately declines to
use working code.

`src/chat_tui/mod.rs` is 17 lines and compiled:

```rust
pub(crate) fn run_chat_tui(_mock: bool, _prefer_ods: bool, _stream: bool, _model: Option<&str>) -> ExitCode {
    eprintln!("Interactive chat TUI is currently undergoing stabilization. Please provide a prompt: `cynic chat --prompt \"...\"`.");
    ExitCode::FAILURE
}
```

It is called from the compiled `chat_surface/command.rs:28`. So `cynic chat` without `--prompt`
**always** prints a stabilisation notice and exits `FAILURE`.

Meanwhile 11 orphaned files hold a coherent TUI: `state.rs` (1 466 lines) imports
`super::actions::{TuiAction, TuiEffect}`, declares `pub(super) enum Focus`, `TuiTaskEvent`, and
`pub(crate) fn dispatch_action` at `:135`; `draw.rs`, `input.rs`, `theme.rs`, `state/` are present.
The stub is not a placeholder for unfinished work. It is a placeholder for **finished** work that
was set aside, and the message sent to the operator does not say so.

`git log` attributes the stub to `1fa6ec3d "fix(workspace): restore clean verified baseline from
24c9769d"`. A verified baseline was restored by discarding a subsystem without recording that
the subsystem existed. This is the same failure shape as the rest of the audit: a state change
with no read-back.

## 6. Canonical taxonomy, as measured

Observed, not proposed. Three disjoint sets, and the CLI currently has no authority that ranks them.

| set | count | source of truth | status |
|---|---|---|---|
| **compiled** | 132 files | `src/lib.rs` `mod` declarations | the only set that can produce behaviour |
| **clap-routable** | 25 top-level `Command` variants | `src/cli_args.rs` | the only set an operator can reach |
| **declared** | 165 advertised leaves, 24 manifest domains, 45 registry surfaces | `command_manifest.rs`, `infra/surface-registry/`, `infra/agent-exposure.json` | documentation |

The governance rule that follows, and which no file currently states:

> A surface is **real** only if it is in the compiled set. A surface in the declared set but not
> the compiled set is a **declaration without an implementation** and must be reported as
> `unknown`, never as available. A surface in the compiled set but not clap-routable is
> **unreachable** and must be reported as such.

Under this rule, today: 165 declared, 91 unroutable (`INFERENCE_SURFACE_AUDIT.md` §4.1), and
11 044 LOC of implementation that is neither declared nor reachable.

### 6.1 Two names, one surface

`infra/surface-registry.json#runtime_disk_management` declares canonical entrypoints
`cynic runtime disk` and `cynic runtime disk-clean`, and names `domain::disk` in
`cynic-host` as owner. `RuntimeCommand::Disk` and `DiskClean` existed and were dispatched at
`runtime_surface.rs:529` but were not reachable from argv. That single omission is the whole
`runtime` row of the 91. It is now wired (`9a0ad77b`).

The same pattern explains the other five unroutable manifest domains — `dev`, `bootstrap`,
`lifecycle`, `fleet`, `deps` — with one difference: for four of them the implementing code is
also not compiled. Fixing argv alone is insufficient and would be a false repair.

## 7. Execution order

Each step produces the proof the next one consumes.

| # | step | proof | why here |
|---|---|---|---|
| **A1** | Retarget the 6 stale tests to `lib.rs` | those 6 go green with no production change | free, and separates stale from real before anything is wired |
| **A2** | Derive the crate-count ratchet from the workspace | `cli_audit_crate_verification_passes` green without a pinned literal | removes the only count that must be hand-edited |
| **A3** | Wire `surface_audit` (649 LOC) | `canonical_surface_audit_executes_against_real_modules` can run `cynic dev surface-audit --json` | the auditor must exist before the rest is wired, or the wiring is unobserved |
| **A4** | Wire `dev`, `bootstrap`, `lifecycle`, `fleet`; declare the 4 lifecycle helpers in `lib.rs` | 4 domain tests green; `--help` lists the domains | the 5 real-drift tests turn green as a side effect |
| **A5** | Reduce the `portable_lab` assembler disagreement | `commands_manifest_exposes_canonical_surface_boundaries` green | assembled registry must equal on-disk registry |
| **A6** | Decide `chat_tui`: rewire the 3 387 LOC, or delete them and say so in the help text | `cynic chat` either works or refuses honestly | 3 387 LOC cannot stay silently orphaned |
| **A7** | Add the build-graph check itself | a test fails when a `src/**/*.rs` file is unreachable from `lib.rs` | converts this audit from a document into a gate |
| **A8** | Only then: argv registry + `cynic audit surface-registry --json` | non-zero on a planted violation | enforcement is worthless before the sets it compares are true |

A1 and A2 are safe and independently provable. A3 is the load-bearing one.

## 8. Explicit unknowns

- `agent_exposure_audit_is_split_into_scan_and_check_modules`: the first assertion should hold
  given all five files are compiled. Cause not isolated. `unknown`.
- `surface_registry_is_split_into_catalog_and_types`: not reduced to a single cause. `unknown`.
- The exact cause of the `portable_lab` omission in the assembler is not established; only that
  the fragment and the manifest entry both exist and the compiled assembler omits it. `unknown`.
- `chat_tui`: whether the 11 orphaned files still compile against the current
  `cynic-inference` and `chat_surface` APIs is **untested**. They have never been in a build
  graph together with their present dependencies. Assuming they are rewireable is an
  assumption, not a finding.
- Whether the 3 387 LOC TUI was set aside deliberately for a reason recorded outside the
  repository. `1fa6ec3d` restores a baseline; the reason is not in the message.
- The resolver is text-based. A `#[cfg]`-gated `mod` declaration, or a module reachable only
  through a macro, would be misclassified. No such case was found, but absence was not proven.
