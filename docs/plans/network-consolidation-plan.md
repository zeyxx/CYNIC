# Network consolidation plan

Status: draft, 2026-11-05. Written by hermes. Nothing here is applied except where marked.

## Why

The network is observed badly, and the observation is trusted.

- `cynic network audit` reports `WireGuard peers 0 ok / 0 stale / 0 dead`.
- `cynic network runtime` reports `wireguard_mesh_absent`, status `blocked`.
- `wg0` is up on cortex (`10.20.0.3/24`).
- Cause (observed): `wg show` needs root. Operator account `user` has no sudo.
- A dashboard built now would display false data.

Rule for this plan: make observation trustworthy first, then display it.

## Observed facts

| Fact | Source |
|---|---|
| `wg0` = `10.20.0.3/24` on cortex | `ip -br addr` |
| Registry declares mesh `100.64.0.0/10`, iface `wg0` | `infra/registry.json` `network_policy.planes.mesh` |
| Topology declares overlay `10.20.0.0/24`, peers `.1`-`.5` | `infra/os/network/topology.json` |
| Third address `100.74.31.10`, owner unknown | `topology-observation.json` (dated 2026-08-26) |
| Registry service `embedding-server` points at `10.20.0.3:8081` | `infra/registry.json` |
| `cynic-kernel` unreachable at `192.168.100.2:3030` | `cynic network audit` |
| Default route leaves cortex via Wi-Fi to the box (`192.168.0.254`) | `ip route` |
| Only `10.10.10.0/24` is routed via Proxmox (`192.168.100.1`) | `ip route` |
| `wg show wg0 dump` is called from 4 places in 3 crates | `grep` over `crates/` |

Four address plans disagree: registry (`100.64.0.0/10`), topology (`10.20.0.0/24`), the live interface, and an unexplained `100.74.31.10`.

## Access state (cause of the blocker)

- 2026-10-22 ~10:50: chantier "c41-access" removed `user` from groups `sudo`, `docker`, `cynic`. Journal: `docs/plans/c41-access-execution-log.md` on branch `agent/hermes/c41-access`.
- Same chantier cut `/etc/sudoers.d/claude-code` to 6 read-only `systemctl` verbs.
- Root account is locked (`!*` in `/etc/shadow`). `su -` can never work.
- That chantier kept an agent-held root shell as fallback. Reboot on 2026-11-04 23:36 destroyed it.
- Result: no root path exists on cortex, for the operator or for agents.
- Its step 6 (agents move to principal `cynic`) is marked NOT DONE.
- Observed now: hermes runs under `user` (3 processes). None under `cynic`.

Intended model (operator): agents run as `cynic`, the operator is `user`.
Actual state: the operator is disarmed and agents still run under his account.

## Order of work

### Step 0. Restore an admin path (operator, physical)

Not doable by an agent.

1. Reboot. At GRUB press `e` on the default entry.
2. On the `linux /boot/vmlinuz-...` line, append ` init=/bin/bash`. `Ctrl+X`.
3. In the root shell, one command per line:
   - `mount -o remount,rw /`
   - `usermod -aG sudo user`
   - `install -m 0440 -o root -g root /home/user/.hermes/cache/scratch/cynic-wg-observe.sudoers /etc/sudoers.d/cynic-wg-observe`
   - `visudo -c`
   - `reboot -f`

Checks done before: `/` and `/boot` share `/dev/sda2` (ext4, no LUKS), no GRUB password found, Secure Boot disabled, UEFI.
Unchecked: whether GRUB blocks the `e` key. If it does, stop and report.

Risk window: between `usermod -aG sudo user` and step 1 below, agents under `user` can reach root.
Mitigation: do not start agent sessions in that window.

### Step 1. Agents move to `cynic` (before any new agent session)

- Principal `cynic` exists (`/var/lib/cynic`, groups `cynic`, `cynic-agent`).
- No launcher runs agents under it yet.
- Deliverable: one declared way to start an agent session as `cynic`, plus the agent read path to registry and secrets that c41 step 6 left open.
- Not designed yet. Needs its own plan.

### Step 2. Key-free WireGuard read grant

Grant `cortex-wg-observe` in `infra/privilege-grants.json` (status `proposed`, commit `aa7534b9b`).

- Permits seven views of `wg0`: `peers`, `latest-handshakes`, `endpoints`, `allowed-ips`, `transfer`, `listen-port`, `public-key`.
- Forbids `dump` (first line prints the interface private key), `private-key`, `preshared-keys`, `showconf`, any `set`, any other interface, any extra argument.
- Abuse ceiling: reader learns mesh topology. Cannot read secret keys, change the interface, or get a shell.
- Revoke: delete `/etc/sudoers.d/cynic-wg-observe`.
- Read-back: `sudo -n /usr/bin/wg show wg0 peers` lists peer public keys without a password prompt.

Code follow-up: the 4 `dump` readers must move to the key-free verbs through ONE adapter calling `sudo -n /usr/bin/wg`.

Known deviation: this rule lives in the grants registry, not the capability catalog. The catalog installer writes `*` for every placeholder and its executor needs an installed `cynic` run as root. Migrate when the network crates are redone (step 4).

### Step 3. Fresh observation of the real network

- Run `cynic network audit`, `network runtime`, `network observe` with step 2 in place.
- Save the output as dated evidence under `infra/os/network/`.
- Replace the stale `topology-observation.json` (2026-08-26).
- Resolve the open unknowns: owner of `100.74.31.10`, owner of Proxmox listener `:4000`.

### Step 4. One canonical address plan

- Pick one overlay plan from observation, not from the registry.
- Fix `infra/registry.json` where it disagrees (`100.64.0.0/10` vs `10.20.0.0/24`).
- Remove the `service_via_nat` workaround and the ACL gap noted for `10.20.0.3` (Headscale ACL blocks builder).
- Registry stays the SSOT for addresses. Nothing hardcoded elsewhere.

### Step 5. Crate consolidation

Network logic is spread over `cynic-network` (36 files), `cynic-node` (79 files mentioning mesh/wg), `cynic-host` (32), `cynic-protocol` (20), `cynic-cli` (16).

- `wg show` readers: `cynic-host/src/wireguard_readback.rs`, `cynic-node/src/adapters/runtime_observation/mod.rs`, `cynic-network/src/adapters/host_probe.rs` (2 sites), plus `cynic-node/src/watchdog/peer_handshake.rs` (`latest-handshakes`).
- Target: `cynic-network` owns observation. `cynic-node` and `cynic-host` consume it. Pure types and ports in `cynic-protocol`.
- Failure semantics today differ: one reader degrades each peer to `Unreachable`, another reports "zero peers" on any error. Zero peers must never stand for "could not observe".
- Check consumers with a grep before moving or deleting anything.
- Needs its own plan with a per-symbol move table. Not done.

### Step 6. Simple dashboard

- Source: `cynic network audit --json`. No new collector.
- Existing spec: `docs/specs/OPERATOR_TUI_AND_LIVE_OBSERVABILITY_SPEC.md`. It cites `crates/cynic-observer/src/dashboard.rs`. That crate does not exist. The spec is ahead of the code.
- Decision needed: TUI panel or small HTML page. Not decided.

### Step 7. Egress node for X capture

- Only after steps 2-4.
- Option kept by the operator: egress through the WireGuard mesh.
- Open design points:
  - `AllowedIPs` that covers X addresses.
  - A second ACL case on Headscale.
  - The proxy still re-signs TLS, so a CA stays trusted by the browser.
  - A lower proxy placement does not fix the observed 403 on `timeline/home.json` (hypothesis: TLS fingerprint on the proxy-to-X leg).
- Suspended project: `docs/specs/SPEC_X_SOCIAL_GRAPH_AGENT.md`, branch `agent/hermes/x-graph-spec`.

## Open decisions

1. Which overlay plan is canonical: `10.20.0.0/24` or `100.64.0.0/10`.
2. How agents start under `cynic`, and how they read registry and secrets.
3. Dashboard form: TUI panel or HTML.
4. Whether to create a separate admin account instead of restoring `user` in group `sudo`. Not chosen.
5. Whether GRUB blocks editing. Unknown until tried.

## Not covered by this plan

- Rotation of secrets exposed until 2026-10-22 10:50 (c41 step 5). Marked NOT DONE in its own log.
- `NET-SEC-001`: a storage credential passed as a process argument (critical, from the August observation).
- Unverified: whether loss of group `cynic` also blinds `cynic network runtime`.

## State of branches

- `agent/hermes/x-graph-spec`: spec + DDD rule in `AGENTS.md`. Pushed. No PR.
- `agent/hermes/network-observe-grant`: grant proposal and this plan. Pushed before this file. No PR.
