# Network Enrollment — mandatory gate before any plane access

Schema: `cynic.network_enrollment.v1`
Status: `ratified` — enrollment is **mandatory**. No machine receives an IP,
a route, a WireGuard peer, or a workload before its enrollment is recorded
in `infra/registry.json`. Ad-hoc wiring is forbidden (see 2026-10-19:
compute-01 stranded on the Freebox LAN, invisible to every CYNIC plane).

## 1. Status machine (registry `status` field — already in use)

- `stub` — intent declared (id, owner, role, planes requested). Nothing assigned.
- `candidate` — physical path proven: every requested plane answers its probe (§3).
- `observed` — services verified live for 24 h (runner polls, probes green).
- Regression: any failed re-probe demotes to `candidate`; workload assignment stops.

## 2. Enrollment record (per node in `infra/registry.json`)

Required fields: `id`, `owner` (human), `role`, `status`,
`network.{plane: address}` for each granted plane,
`transports.ssh.{target, principal}`,
`enrollment.{date, enrolled_by, review_after}` (max 90 days).
No record = no access. The record IS the permission.

## 3. Per-plane acceptance probes (run from an enrolled observer)

- admin plane (`192.168.100.0/24`): `ping -c2 <addr>` + `ssh <principal>@<addr> hostname`
- service plane (`10.10.10.0/24`): `ping -c2 <addr>` + `curl -s -o /dev/null -w '%{http_code}' http://<addr>:3000/api/v1/version` (Forgejo) where applicable
- mesh (`10.20.0.0/24` wg): `wg show` lists the peer with a recent handshake (`wg show <if> latest-handshakes`)
- egress: `curl -sI https://github.com --max-time 10` (only if the role requires it)

All green → `candidate`. Any red → stays `stub`, with the missing path named.

## 4. First application (2026-10-19)

- `cynic-compute-01`: `stub` — wifi-only, no RJ45, reaches no CYNIC plane
  (probes §3 all red from 192.168.0.25). Paths under review: USB-NIC,
  WireGuard peer (full-mesh, static peers — needs peer entries on PVE/cortex),
  or park as inference-only. No runner until a path goes green.
- `cynic-host-cortex` runner (`cynic/cortex-overflow`): enrolled locally, gated
  by the RAM watcher (service enabled only under threshold) — enrollment
  record to follow in the registry.

## 5. Enforcement

- Workload schedulers MUST check `status == observed` (or `candidate` for
  probationary runs) before assigning. Assigning to `stub` is a violation.
- The main-divergence watchdog model extends here: a plane-reachability
  watchdog re-probes enrolled nodes and demotes on failure.
