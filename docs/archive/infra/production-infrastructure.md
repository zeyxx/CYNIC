# Production Infrastructure — target state and observed gaps

Status: `proposed` — awaiting operator validation before any code moves.
Schema: `cynic.production_infrastructure.v1`
Companion: `docs/design/runtime-roles.md` (who runs what),
`contracts/runtime-update.json` (how releases move),
`docs/design/fleet-network-architecture.md` (how nodes talk).
Evidence date: 2026-10-06. `unknown` means unobserved, not absent.

---

## 1. First principles — what production-grade means here

A sovereign fleet node is production-grade when all seven hold. Each is
independently verifiable by read-back; none is prose.

1. **Provenance**: every executed byte traces to a repo commit + builder
   proof. No orphan binaries.
2. **Atomic update**: versioned install, atomic switch, health proof,
   automatic rollback, GC (already contracted: `runtime-update.json`).
3. **Secret hygiene**: no cleartext material outside the sealed vault /
   OS store; presence observable without exposure (`cynic secret status`).
4. **Named service**: real DNS, no `/etc/hosts` dependence; TLS on every
   ingress; certificates rotated, not hand-installed.
5. **Observed health**: every daemon has a probe with an owner and an alert
   path. Silence is data (`unknown`), never success.
6. **Least privilege**: daemons run as `cynic`, root bridge only through the
   seed; remote actuation through an explicit grant, never ambient sudo.
7. **Recoverability**: registry + contracts + sealed backups rebuild any
   node. A node that cannot be rebuilt from Git + vault is snowflake debt.

## 2. Target topology (3 nodes + LXC satellites)

| Node | Role | Runs (target) | Update path |
|---|---|---|---|
| cynic-builder (10.10.10.119) | `build_runner` (canary) | `cynic-node.service` (+ build load, no steady-state fleet duties) | Local `source deploy --apply` after gate. First in `fleet_order`. |
| cynic-host-proxmox | `hypervisor` (ingress) | hypervisor-local `cynic-node.service`, watchdog (declared), PVE stack, ingress proxy + TLS | Only after builder converges. Transport TBD (§4.5). |
| cortex (192.168.100.2) | `cortex_host` | operator CLI, MCP projection, `cynic-kernel` if confirmed as kernel home | Operator-driven, never auto-updated. |
| LXC satellites | storage / mesh / forge roles | SurrealDB, Headscale, Forgejo runners | Behind the hypervisor; data backup policy per satellite (§4.7). |

Ingress: PVE terminates TLS (ports 80/443 already open at the box), reverse
proxies to named services (kernel API, cloud API if legitimized, Forgejo).
DNS: one real zone for the fleet; `/etc/hosts` entries recorded as tech
debt with an expiry, never as architecture.

## 3. Observed gaps (with evidence)

| # | Gap | Evidence | Severity |
|---|---|---|---|
| 1 | Orphan `cynic-cloud-api` on ingress hypervisor | DECIDED 2026-10-06: quarantine (stop+disable, binary kept for triage). The *principle* is retained: a `cynic-cloud` web-surface hosting role on the hypervisor is wanted — as a declared, sourced, basket-listed service, not as this orphan. See runtime-roles §5.1. | **Critical** — quarantine pending root execution (no sudo grant on PVE) |
| 2 | 13 un-GC'd releases on builder | `ls /var/lib/cynic/releases` → v0.2.0…v0.8.0 | Low — first GC-capable deploy cleans it |
| 3 | No daemon health endpoints | cynic-node has no HTTP surface; kernel unlocated | Medium — `active` + `observe` is the interim probe; real `/health` per daemon is the target |
| 4 | Port owners unknown | builder `:9090`; PVE `:80/443/4000/53/3128` (no sudo to attribute) | Medium — ingress you cannot attribute is ingress you cannot defend |
| 5 | PVE outside the update model | No releases dir, direct `/usr/local/bin` installs, dead Jul-1 `cynic-node` binary | Medium — blocks `fleet_order` step 2 until transport decided |
| 6 | Registry mostly `candidate`/`stub` | `local-builder` is a stub; kernel/satellite nodes unobserved | Low — registry follows observation, but convergence needs owners |
| 7 | Secrets mid-migration | 2/3 ids in OS store, cleartext files retained until new binary deploys | Low — tracked, ordered, resolves with the canary deploy |
| 8 | No backups observed | No satellite backup policy found in repo this session | Medium — SurrealDB (storage_primary) loss = journal loss |

## 4. Work packages (in order — each gated on the previous)

1. **Canary deploy** (running): builder gate → build → activate
   `cynic-node.service` → GC. Unlocks: sealed `secret install`, cleartext
   purge, GC proof.
2. **Orphan triage**: `strings`/build-ID the cloud API binary or rebuild it
   from source; either it enters the basket with provenance or it leaves
   the hypervisor.
3. **Health surfaces**: one probe per daemon (`/health` or `observe`
   contract), owner + alert path declared in the unit's contract entry.
4. **Ingress attribution**: identify every listener on PVE :80/443/4000 and
   builder :9090; undocumented listeners close or get documented.
5. **PVE transport decision**: ssh+sudo grant vs node-initiated pull for
   remote deploy; then enroll PVE in `fleet_order`.
6. **DNS + TLS**: real zone, reverse proxy with certificates, `/etc/hosts`
   sunset.
7. **Backup policy**: satellite data (SurrealDB first) — what, where, how
   often, restore proof.

## 5. Explicit non-goals (until §4 is done)

No third physical node. No public-internet services beyond what the box
already forwards. No new top-level CLI families for infra — arms of owning
surfaces only. No Kubernetes, no Docker (root-by-default violates principle
6; already decided), no Python in the runtime path.
