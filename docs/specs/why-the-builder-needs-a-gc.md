# Why the builder needs an admin garbage collector

Measured on the builder (10.10.10.119, 177 Go) on 2026-10-20, after the
disk reached 100% and took the whole pipeline with it: truncated clones,
corrupted `.git` objects, builds aborted with `couldn't create a temp
dir`.

## THE MEASURED VOLUMES

| category | size | reclaimed today? |
|---|---|---|
| `checkouts/` (per-SHA evidence checkouts) | 46 Go | YES, by age |
| `  of which local `target/` dirs | 22 Go | only when the checkout dies |
| `shared-target/` (canonical cargo cache) | 31 Go | YES, below 4 GiB free |
| `build-targets/` (deployment workspaces) | 11 Go | NO — not a cache |
| `sccache/` (compiler cache) | not measured | NO — never declared |
| `/tmp` (build temp targets) | not measured | NO |

88 Go accounted for on a 177 Go disk.

## WHY A GC, NOT JUST A RETENTION POLICY

`cynic build reclaim` is a **cache** policy: it knows the cargo cache
and the checkouts. It is not an **object** policy: it has no registry of
what exists on the machine, who created it, how old it is, or whether
anything still holds it.

The gaps show it:

- **A build without `CARGO_TARGET_DIR` writes its cache inside its own
  checkout.** Six checkouts carried 23 Go that way — 5.1, 5.1, 4.0, 3.5,
  2.8 and 2.7 Go. The checkout is ephemeral and reclaimed by age; the
  cache inside it is a different object with a different lifetime, and
  nothing reconciles the two.
- **`sccache/` is a real cache with no declared owner.** It is the
  compiler cache, keyed by content, and nothing ever reclaims it.
- **`build-targets/` (11 Go) holds deployment workspaces** — `cynic-canon`,
  `cynic-deploy-check` — not cache. A policy that reclaimed it would
  destroy deployments.

## THE SHAPE OF THE FIX

A GC needs a **registry of objects**, not a list of directories:

- every object has an id, a path, a class (cache / checkout / workspace
  / log), an owner, a creation time, a last-use time, and a size;
- reclamation is a pure decision over that registry — the same shape as
  `reconcile_build_cache`: a plan with blockers, never a side effect;
- a liveness check answers "is anything still using this?" — for a
  checkout, "is a build running in it"; for a cache, "is it younger than
  the reuse window";
- the registry is derived from the filesystem, so a directory nobody
  declared surfaces as `unknown` and is reported, not silently kept.

`reconcile_build_cache` already encodes the decision order to copy:
**dead weight goes before the ungoverned**, and an ungoverned object is
a reported blocker, never a silent deletion.

## WHAT NOT TO DO

- **Do not `rm -rf` the local `target/` dirs by hand.** A build is
  usually running in one of them. The reclaim knows how to tell; a
  shell does not.
- **Do not widen the reclaim to `build-targets/`.** It holds deployments.
- **Do not lower the retention age to force the 22 Go out.** The
  6-hour window exists so a checkout of evidence stays reusable.

## THE ROOT CAUSE IS AN INVARIANT, NOT A CLEANUP JOB

The 22 Go exists because `CARGO_TARGET_DIR` was not set for some builds.
A GC would then clean up after a broken invariant forever. The
invariant is the fix:

> every build writes its target dir to the declared canonical cache,
> and the canonical cache is the only place build artifacts land.

A GC is still worth having — the machine is shared, and other agents
will break the invariant again. But it is the second line, not the
first.
