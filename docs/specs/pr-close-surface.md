# Spec: `cynic source pr close` surface + PR-lifecycle debt register

## 1. Goal

Add a native `close` arm to the PR family so agents never need the Forgejo UI
to abandon a pull request:

```sh
cynic source pr close --number 104 [--delete-branch] [--json]
```

Non-goals: review comments, reopen, merge-strategy changes (out of scope, noted
as debt in §4).

## 2. Domain changes (`cynic-vcs::pull_request`, Zero-I/O)

- New schema const: `PR_CLOSE_PROOF_SCHEMA = "cynic.vcs.pr_close_proof.v1"`.
- New proof type (mirrors `PrMergeProof`):
  `PrCloseProof { schema, pr_number, closed: bool, branch_deleted: bool, message }`.
- New port method: `fn close(&self, number: u64, delete_branch: bool) -> Result<PrCloseProof, String>`.
- New pure evaluator `evaluate_pr_close(pr) -> Result<(), String>`:
  close is admitted **only** from `PrState::Open`; `Merged` and `Closed`
  are refused with `pr_not_open:state_{:?}` (same vocabulary as merge admission).
- Tests: admit open, refuse merged, refuse closed, proof schema const.

No new top-level command: `Close` nests under the existing `PrCommand` enum
(taxonomy law holds).

## 3. Adapter (`cynic-vcs-forgejo`)

- `PATCH {api}/repos/{owner}/{repo}/pulls/{number}` with `{"state":"closed"}`.
- Read-back: re-`GET` the PR and assert `state == "closed"` before returning
  the proof (observe → apply → read-back pattern).
- `--delete-branch`: `DELETE {api}/repos/{owner}/{repo}/branches/{head}`
  after close; `branch_deleted` reflects the outcome. Fail-closed: a failed
  delete returns `closed: true, branch_deleted: false` plus message, never a
  silent partial.
- Reuse one curl call path (see debt D2).

## 4. CLI (`cynic-cli/src/source_surface/pr.rs`)

- `PrCommand::Close { number: u64, delete_branch: bool, json: bool }`.
- Pre-check: resolve the PR via `port.list("open")`; refuse with
  `blocked: PR #N not found among open PRs` (same UX as merge).
- JSON output via `emit_pretty_json(&proof)` when `--json`.

## 5. Debt register (surfaced while mapping)

| # | Location | Debt |
|---|----------|------|
| D1 | `PrPort` | No `close` (this spec), no `comment`, no `reopen`. Triptyque reviews live in markdown, not on PRs. |
| D2 | `cynic-vcs-forgejo/pull_request.rs` | Three hand-rolled curl call sites (`curl_get`, `curl_post`, inline merge). One call path + typed errors. |
| D3 | `auth_header` | Returns literal `"Authorization: token ***"` — the token is never interpolated. Merges still succeed, so auth must flow from elsewhere; trace and prove the real path or fix. |
| D4 | `parse_pr` | Forgejo `"merged"` never maps to `PrState::Merged` — the variant is unreachable; merged PRs read as `Open`/`Closed`. Admission relies on the open-list, masking the gap. |
| D5 | `parse_pr` defaults | `mergeable: unwrap_or(true)`, `has_conflicts: unwrap_or(false)`, `number: unwrap_or_default()` — fail-open parsing against the fail-closed doctrine. Unknown must block, not admit. |
| D6 | `list` | `limit=50` hardcoded, no pagination. Fleet-scale PR counts will silently truncate. |
| D7 | merge/close | Head branches are never deleted (`review-*`, `convergence-*` accumulate on the remote — ~15 stale agent branches today). `--delete-branch` default policy needed. |
| D8 | `contracts/source-lifecycle.json` | Check whether `branch_state_machine` covers PR close/delete transitions; extend if not. |

## 6. Acceptance

- `cargo test -p cynic-vcs` new close-admission tests green.
- `cynic source pr close --number <test> --json` returns
  `cynic.vcs.pr_close_proof.v1` with `closed: true` against Forgejo.
- Builder `cargo check --workspace` green.
- D3 answered (auth path proven) before merge.
