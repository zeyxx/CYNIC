# Signed storage inventory — builder 10.10.10.119, 2026-10-22

Read-only observation. No mutation performed. Unclassifiable is not
eligible: what cannot be proven reproducible is not proposed for eviction.

## Live map vs terrain (retention policy)

Three different canonical stories:

- Live registry (`/etc/cynic/registry.json`, root-owned, actually consumed
  by the scheduler): canonical = `build-targets/bootstrap`, ephemeral
  scope = `checkouts` only, max age 7 days.
- Repo registry (`infra/registry.json` on main): canonical =
  `build-targets/fgate`, `shared-target` declared noncanonical candidate.
- Reality: `build-targets/bootstrap` does not exist as a warm dir.
  Two caches are concurrently hot: `shared-target/debug` (written Oct 21)
  and `build-targets/fgate` (written Oct 22).

Consequence: the live registry is stale (the staged update was never
installed as root — open item). The invariant "every build writes the
canonical cache" cannot hold when the map names a phantom.

## Fossil workspaces (`/var/lib/cynic/source/workspaces`)

| dir | size | touched | liveness | git state | verdict |
|---|---|---|---|---|---|
| `CYNIC/` | 1.2G (target 864M) | Aug 18 | no process inside | BROKEN worktree metadata (main repo `source/repositories/CYNIC` worktree record unreadable) | unknown — git surgery first, never delete blind |
| `cynic/` | 108M (no target) | Oct 15 | no process inside | not a git repo at all | unknown — provenance missing |
| `federation-layer0/` | 1.4G (target 1.1G) | Aug 22 | no process inside | not a git repo at all | unknown — provenance missing |
| `layer0-base/` | 1.7G (target 1.4G) | Aug 22 | no process inside | clean repo, HEAD `73e4745 layer0 proof workspace` | evictable with operator go (re-clonable, stale, clean) |

Only `layer0-base/` (1.7G) is proposed for eviction. The other three are
explicit unknowns, not debris-proven.

## Other measured classes

- `checkouts/` 7.7G: 31 ephemeral clones, reclaim planned, 0 blockers.
  Native `--apply` pending operator go.
- `shared-target/` 35G: warm cache, runner asset. Never purge directly.
- `build-targets/` 14G: deployment workspaces + second hot cache.
  Never declare ephemeral (would destroy deployments).
- `runner-job.log` 61M: no rotation. Trivial fix, needs writer restart.
- `sccache-cache/` 2.5G: within policy assumptions, observe only.
- `/tmp`: `cynic-ext` 3.4G, `enroll-target` 2.8G, `enroll-proof` 1.9G,
  `rel-build` 1.2G — active proof surfaces, keep while proving.

## Dictated next (authority-gated)

1. Operator (root builder): install staged repo registry over
   `/etc/cynic/registry.json` — un-stales the live map.
2. Operator go: evict `layer0-base/` (bounded, with read-back), apply
   planned checkout reclaim.
3. Code: single canonical cache migration (writers off `shared-target`),
   `runner-job.log` rotation.
