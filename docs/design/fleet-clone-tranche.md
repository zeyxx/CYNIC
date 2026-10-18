# Tranche 1 — Clean-template LXC clone (spec)

## Status

Proposed. Adversarial review required before any slice.

## Entry data (to measure, not assumed)

- Tranche 0 measured `vzcreate` at 21–36 s (task log, 12 runs).
- Clone duration: UNMEASURED. Measure `POST /nodes/{node}/lxc/{tmpl}/clone`
  on this cluster before claiming speed.
- If clone is not ≥2× faster than create, this tranche STOPS (no
  mechanism without a measured win).

## Prism

*Speed without dirt* — clone only from a versioned template, never from a
live guest. No guest state (keys, logs,shell history, ATP state) may leak
into clones.

## Design

1. **Template build (versioned, reproducible):** a fresh CT (next free
   vmid) is created via the tranche-0 path with a build spec (OS template
   + node v22 + `cynic` user + base dirs + units WITHOUT secrets), then
   converted with `pct template` (API: `POST
   /nodes/{node}/lxc/{vmid}/template`). The template is named
   `relay-base-<date>` (vmid recorded in the receipt, never hardcoded).
   Template build script lives in CYNIC (shell-free: typed argv or REST),
   not in chat history.
2. **Hygiene gate (fail-closed):** before templating, the builder asserts:
   no files under `/etc/cynic/secrets`, no `authorized_keys` beyond the
   build key, no shell history, no journal persistence of secrets
   (`journalctl --vacuum` + machine-id reset). Each check is a named
   blocker; any failure stops templating.
3. **Clone surface:** extend the canonical path, not a second grammar —
   `cynic node compute provision --clone-from <template-vmid>` (or
   `--kind clone`) reuses `LxcProvisioningSpec`; the executor switches
   `POST /nodes/{node}/lxc` (create) to `POST
   /nodes/{node}/lxc/{tmpl}/clone` + the same UPID wait + read-back +
   rollback. Same receipt schema, plus `cloned_from`.
4. **Per-clone identity:** hostname, IP, and SSH host keys are regenerated
   post-clone (never copied): `rm /etc/ssh/ssh_host_*` + reconfigure on
   first boot via the executor, verified by read-back before `provisioned`.

## Review gates

- Plan diff (create vs clone) + proof that no guest state leaks
  (clone a canary file in the template, assert absence... inverse:
  plant a canary secret in the BUILDER, assert it is absent in the clone).
- Same transverse review: spec → attack → tests → live proof → commit.

## Non-goals

- No QEMU (tranche 2). No scheduler (tranche 3). No template GC policy
  (one template, manual deletion).
