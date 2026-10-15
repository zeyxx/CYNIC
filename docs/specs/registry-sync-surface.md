# Registry Sync Surface (declared repo vs runtime copy)

Status: design for review. No code.

## Problem (observed 2026-10-15)

- Declared truth lives in `infra/registry.json` (11 nodes, trio planes,
  DNS zone, live).
- The runtime copy at `CynicPaths::registry_path()`
  (`/etc/cynic/registry.json`) was an August fossil (6 nodes,
  `management`/`synapse` planes) until manually refreshed 2026-10-15.
- No sync mechanism exists (no code, no docs, no env). Every default-path
  CLI command silently read the fossil.
- The reconcile path writes an `observed` top-level section into the
  runtime copy, so a naive overwrite would destroy observed state.

## Proposal

- Nest: `cynic runtime registry sync` (runtime family: path resolution +
  conformance; no new top-level per taxonomy law).
- Pure core in `cynic-network::registry_maintenance` next to
  `reconcile_registry_observations`; thin CLI surface.
- Source defaults to the repo file, target to
  `CynicPaths::registry_path()` (both overridable). `CYNIC_REGISTRY_PATH`
  already exists as an escape hatch and stays untouched.
- Merge rule: declared keys replaced wholesale from source; `observed`
  (and any future runtime-only sections) preserved and re-validated,
  never deleted by a sync.
- Safety: backup `<target>.bak-<source-sha>` before write; refuse when
  the source does not parse or the schema version diverges; dry-run by
  default (prints node/plane diff, writes nothing); `--yes` required.
- Proof: before/after digests plus a summary diff (nodes added/removed,
  planes changed, zone changed). Re-sync is idempotent
  (digest unchanged).

## Historical evidence (2026-10-15 study)

- Last 15 repo changes: top-level key set frozen (12 keys), nodes 8-11
  growing. `observed` never present as a live key (July hits are
  removed doc text, not data). Repo file is pure declared.
- Planes cut cleanly once: `synapse/overlay` to `service/admin/mesh`
  in a single commit, stable since. No gradual drift to accommodate.
- The `/etc` fossil (`management/synapse`, 6 nodes, Aug 22) predates
  the oldest sampled lineage: seeded once, never synced.
- Consequence for the merge rule: wholesale declared replacement plus
  `observed` preservation is sufficient; no per-key migration needed.

## Open questions for review

1. Exact runtime-only key set: is `observed` the only section a sync
   must preserve? Historical study below must confirm.
2. Direction: should reconcile write into the repo file instead, making
   the runtime copy a pure render? (Bigger change, not proposed.)
3. Fleet scope: cortex only, or each host syncs its own copy (builder
   reads checkouts; LXCs carry no copy)? Unresolved.
4. Provider nondeterminism note: DNS bundle content varies with the
   caller's PATH (dnsmasq visibility). Document the required PATH in
   the runbook; not part of this surface.

## Acceptance

- After sync, default-path `dns bundle` renders the service zone.
- `observed` survives sync byte-identical.
- Second sync with no repo change is a no-op by digest.
