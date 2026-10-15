# Task P Report

## Fix round 1 — composition gate admits cargo actions (TDD, builder-verified)

### Finding (live canary, proven)

`cynic build submit --action build` is rejected with HTTP 400
`build_agent_request_action_not_allowed`. Root cause: the composition gate
in `crates/cynic-build/src/build/agent_request.rs` hardcoded a third
allowlist — `matches!(action, "make:check" | "make:build" | "make:build-release")` —
which Task P missed when it introduced the honest cargo mapping
(`map_submit_action`: build→`cargo:build`, build-release→`cargo:build-release`,
test→`cargo:test`; plus `cargo:check`). Every cargo submit therefore passed
mapping + argv translation but was refused at composition. Task P fixed the
mapping and the argv allowlist; the composition gate was the remaining
narrow-green hole.

### Change

- `crates/cynic-protocol/src/argv_allow.rs`: new
  `is_canonical_build_action(action) -> bool` predicate over
  `canonical_build_actions()` — single source of truth for action admission.
- `crates/cynic-build/src/build/agent_request.rs`: the composition gate now
  consults the predicate instead of the hardcoded `make:*` literal set, so
  the gate mirrors the policy set exactly (all 8 admitted actions: 3 `make:*`
  + `cargo:check`/`cargo:build`/`cargo:build-release`/`cargo:test`/`cargo:run`;
  unknown actions such as `cargo:deploy` and `bash -c make check` still
  blocked). `cargo:run` remains admitted by policy but is still refused at
  argv construction (`canonical_argv_from_build_action` → `None`), so there
  is no new execution path.
- `crates/cynic-build/tests/workload_build_integration.rs`: repaired
  pre-existing compile breakage on the branch tip (missing `branch` /
  `source_commit` fields in an `AgentWorkloadEnvelope` literal, introduced
  by 9646323a; without this, `cargo test -p cynic-build` compiled no test
  target at all).

### Covering tests

- RED (commit `cb1f8c72`):
  - `agent_request.rs`: `cargo_build_action_is_admitted` (task-specified:
    request with action `cargo:build` → no `action_not_allowed` blocker),
    plus `cargo_check/cargo_test/cargo_build_release_action_is_admitted`
    and `non_canonical_action_is_still_blocked` (`cargo:deploy`).
  - `builder_http.rs`: `submit_action_chains_through_admission_and_argv` —
    end-to-end-ish chain: for each submit verb (`check`, `build`,
    `build-release`, `test`), `map_submit_action` output feeds
    `admit_build_agent_request` (zero blockers) **and**
    `canonical_argv_from_build_action` (Some). A narrow green that fixes
    only one link cannot recur.
- GREEN (commit `1881ccd4`): `argv_allow.rs`:
  `admission_predicate_mirrors_policy_exactly` — predicate admits exactly
  the 8 policy actions and refuses unknown ones.

### Commands + outputs (builder 10.10.10.119, `sudo -u cynic`, proof checkout `/var/lib/cynic/checkouts/cynic-sdd-tPf1`, fresh clone of `agent/hermes/build-latency-science`, removed after)

RED at `cb1f8c72` (+ repair `b4eb3791`):

    RUST_MIN_STACK=16777216 cargo test --offline -p cynic-node --lib
    → FAILED. 452 passed; 1 failed
    → submit_action_chains_through_admission_and_argv panicked:
      "verb build -> action cargo:build refused by composition gate"
    cargo test --offline -p cynic-build --lib   (at b4eb3791)
    → FAILED. 133 passed; 4 failed
    → cargo_{build,build_release,check,test}_action_is_admitted:
      left "blocked", right "admitted"
    cargo test --offline -p cynic-build          (at cb1f8c72)
    → compile error E0063 missing fields branch/source_commit
      (workload_build_integration.rs:74, pre-existing)

GREEN at `1881ccd4`:

    cargo test --offline -p cynic-build
    → EXIT=0 — lib 137 passed / 0 failed (all 5 new agent_request tests ok);
      architecture_boundary 3, orphan_sources_ratchet 1, source_island_gate 7,
      workload_build_integration 11 — all ok
    cargo test --offline -p cynic-protocol --lib
    → admission_predicate_mirrors_policy_exactly ... ok
      (608 passed; 4 failed — pre-existing failures in agent_host_sandbox and
      coordination_score, untouched by this change)
    RUST_MIN_STACK=16777216 cargo test --offline -p cynic-node --lib
    → EXIT=0 — 453 passed / 0 failed
    → submit_action_chains_through_admission_and_argv ... ok

### Commits (branch `agent/hermes/build-latency-science`, pushed to origin)

- `cb1f8c72` test(build): Task P fix round 1 RED — cargo actions in composition gate + chain test
- `b4eb3791` fix(build): fill branch/source_commit identity fields in workload integration test
- `1881ccd4` feat(build): Task P fix round 1 GREEN — composition gate consults canonical_build_actions

### Notes / concerns

- `task-P-report.md` did not exist in the worktree; created here.
- The 4 `cynic-protocol` lib failures (`agent_host_sandbox::rejects_shared_git_admin_path`,
  `coordination_score::*`) are pre-existing on the branch tip and unrelated
  to this fix (this change is additive in `argv_allow.rs` only).
- The repo root `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC` has a broken
  directory listing (`ls` fails with ENOENT while `stat`/`git` succeed); the
  branch is checked out both there and in this worktree
  (`--ignore-other-worktrees` was used to check it out here). Flagged for
  the controller; no action taken on the repo root.
