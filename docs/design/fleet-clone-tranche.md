# Tranche 1 — Agent-runner template + ephemeral clones (spec)

## Status

Proposed (rescoped from relay clones). Adversarial review required
before any slice.

## Purpose

Spawn isolated agents / ephemeral builders in seconds, destroy after.
NOT relay clones (107 stays hand-built, tranche 0).

## Entry data (to measure, not assumed)

- Tranche 0 measured `vzcreate` at 21–36 s (task log, 12 runs).
- Clone duration: UNMEASURED. Measure `POST /nodes/{node}/lxc/{tmpl}/clone`
  on pve before claiming speed.
- Stop condition: if clone is not ≥2× faster than a comparable create
  (same pool, same size, UPID start→stop), this tranche STOPS.

## Capacity (measured)

- pve free memory: ~12 GiB (`observe`, post-107). At 2 vCPU / 4 GiB per
  clone → max ~3 concurrent clones. This is a hard ceiling, not a target.
- Queueing beyond 3 belongs to tranche 3 (scheduler), not here.

## Prism

*Speed without dirt* — clone only from a versioned minimal template,
never from a live guest. Short life: every clone carries a TTL and ends
with a destruction proof.

## Design

1. **Template `agent-runner-<date>` (minimal):** fresh CT via tranche-0
   path, then ONLY: OS updates, `cynic` user, SSH host keys REMOVED
   (regenerated per clone), build key, base dirs. NO secrets, NO product
   code, NO relay stack. Convert with `pct template` (API `POST
   /nodes/{node}/lxc/{vmid}/template`). Build steps live in CYNIC
   (typed argv or REST), never in chat history.
2. **Hygiene gate (fail-closed):** before templating, assert: no files
   under `/etc/cynic/secrets`, no `authorized_keys` beyond the build key,
   no shell history, machine-id reset, `journalctl --vacuum`. Each check
   is a named blocker; any failure stops templating.
3. **Clone surface:** same grammar as tranche 0 — `cynic node compute
   provision --clone-from <template-vmid>` reuses `LxcProvisioningSpec`;
   the executor switches `POST /nodes/{node}/lxc` (create) to `POST
   /nodes/{node}/lxc/{tmpl}/clone` + same UPID wait + read-back +
   rollback. Receipt gains `cloned_from` and `ttl`.
4. **Per-clone identity:** hostname set at clone time, SSH host keys
   regenerated on first boot, verified by read-back before `provisioned`.
5. **Ephemeral lifecycle:** every clone is created with a TTL; expiry →
   `DELETE /nodes/{node}/lxc/{vmid}` (destroy is idempotent — absent
   guest is success). The destroy receipt is archived like the create
   receipt. No clone outlives its TTL without an explicit, logged
   extension.

## Review gates

- Plan diff (create vs clone) + canary test: plant a canary secret in
  the BUILDER, assert it is absent in the clone.
- Same transverse review: spec → attack → tests → live proof → commit.

## Non-goals

- No QEMU (tranche 2). No queue/fairness beyond the hard cap of 3
  (tranche 3). No template GC (one template, manual deletion). No
  product code in the template, ever.
