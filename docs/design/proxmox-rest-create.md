# `cynic node hypervisor create` — REST LXC creation (spec)

## Status

Proposed. Implements option 2 (native mutating surface). Adversarial review
required before any slice.

## Shape

```
cynic node hypervisor create --hypervisor pve --vmid 107 \
  --storage-pool local \
  --template-spec "local:vztmpl/ubuntu-24.04-standard_24.04-2_amd64.tar.zst" \
  [--memory-mib 4096 --disk-gib 32 --vcpus 2 --bridge vmbr0 --hostname relay-107] \
  [--apply] [--json]
```

Default is plan-only (prints the REST call + rollback, mutates nothing).
`--apply` executes. No `--yes` alias, no env-var consent: the flag on the
command line IS the consent, auditable in shell history.

## Execution path (REST only, never root SSH)

1. **Preflight (existing):** `observe` must report zero blockers for the
   vmid/spec. Reuse `plan_lxc_provisioning` gates (memory, disk, pool,
   bridge, template, vmid free).
2. **Token ACL preflight (new, fail-closed):** `GET /access/users/{token}@pve!{id}`
   or an equivalent privilege read; block with `token_missing_capability:<name>`
   unless the token carries `VM.Allocate`, `Datastore.AllocateSpace`,
   `SDN.Use` on the target scope. Never attempt the create to "see if it
   works" — the 403 IS the finding, not the method.
3. **Create:** `POST /nodes/{node}/lxc` with exact body derived from the
   validated spec (vmid, ostemplate, storage, memory, cores, rootfs,
   net0, unprivileged=1, onboot=1). Response UPID is the execution receipt.
4. **Task poll:** `GET /nodes/{node}/tasks/{upid}/status` until `stopped`,
   fail-closed on timeout (default 120 s) with `create_task_timeout:<upid>`.
5. **Read-back (independent):** `observe`-equivalent re-query
   (`GET /nodes/{node}/lxc/{vmid}/status/current`) must show `running`
   (after an explicit start step) — a UPID `OK` alone is not convergence.
6. **Rollback intent:** on failed read-back, `DELETE
   /nodes/{node}/lxc/{vmid}` (destroy is idempotent — absent guest is
   success). Rollback is attempted, reported, never silent.

## Threading

Reuse the fixed observer (per-request thread boundary). The create executor
lives in `cynic-proxmox/src/adapters/` (substrate binding), consumes
`LxcProvisioningSpec` + preflight, never invents argv. No `pct` argv is
built: the plan's `pct` steps remain the SSH-breakglass projection, unused
by the REST path. One grammar per path, no shared builder.

## Proofs (JSON)

`{schema: cynic.agent_host.lxc_create_receipt.v1, vmid, upid, task_status,
read_back: {status, onboot}, rollback: {attempted, result}}`

## Measured precedents (task log, 2026-03 → 2026-10)

- 12 `vzcreate` runs: 21–36 s OK; template-missing fails in 3 s
  (`unable to create CT`). Poll timeout stays 120 s (3× worst case).
- `root@pam!cynic` (API token, not password) created CT 103 successfully —
  scoped-token creation has precedent; no root password needed in the path.
- Template check is load-bearing, not cosmetic: 2 failures on a template
  name not present in the pool.

## Open question for review

Token ACL preflight (step 2): `cynic@pve!runtime` reads nodes/resources
today (observe green). Whether it carries `VM.Allocate` is unmeasured.
Needed on pve (root): the token's effective permissions
(`pveum` ACL readout for the token identity). If unreadable without
exercising, the spec falls back to: attempt create, treat 403 as the
fail-closed finding (`token_forbidden:<scope>`), never retry, always
rollback-check (no guest left behind).

## Non-goals

- No QEMU, no clone-from-dirty (clean template only, per skill law).
- No secret handling: token resolution stays in `proxmox_control.rs`.
- No fleet scheduler integration (single create, no queue/fairness).
- No sudoers/membrane change: the token IS the authority; if it lacks a
  capability, the operator widens the token on pve, not the agent's rights.
