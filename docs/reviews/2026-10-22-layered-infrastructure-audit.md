# CYNIC Layered Audit — 2026-10-21/22

- **Auditor:** Claude Code (Opus 5.5), session on `cynic-host-cortex` (192.168.100.2)
- **Requested by:** operator
- **Intended consumer:** Hermes Agent
- **Method:** bottom-up, one layer at a time (C0 substrate → C5 operations). Every finding carries the observation that produced it. All observation was read-only.
- **Code baseline:** `origin/main` @ `d88f1312`. The local branch `agent/hermes/fleet-chat-inference-2` was 535 commits behind main, so static analysis was rerun on a `git archive` snapshot of `d88f1312`.
- **Mutations performed during the audit:** one ephemeral source snapshot at `/srv/cynic/audit-ephemeral-d88f1312` on the builder (CT105), used to run the gate and `cargo deny`. It was **deleted afterwards** and freed 1.7 GiB. Nothing else was changed on any host.

---

## 0. Instructions for Hermes Agent

1. **This report is evidence, not authorization.** Every remediation below is a mutation. Each one needs explicit operator approval, following AGENTS.md: scope claim, `--yes`/`--apply`, read-back proof.
2. **Re-observe before acting.** The state below was true at audit time. Re-run the evidence command attached to a finding before you remediate it.
3. **Respect the build policy.** Never run cargo or rustc on cortex. Build only on CT105 (10.10.10.119), or on CT104 for Debian targets (see C2-2).
4. **Use the finding IDs** (`C0-1`, `C4-3`, …) in branch names, claims and commit messages, so closure can be traced.
5. **Order matters.** Follow §8, "Remediation order". Access and data safety come before everything else.
6. **Severity scale:** 🔴 critical (exploitable or data loss now) · 🟠 high · 🟡 medium · 🔵 low/info.

---

## 1. Executive summary

**Sound:** the code. L0 is pure, there are almost no production panics, no secrets are committed, no known dependency CVEs exist, and MCP runs over stdio only. The **measurement tooling** is also strong: the audit engine, registry validators, `make audit.deployed` and the debt ledgers all detect the problems correctly.

**Broken:** nothing connects *declared* state to *running* state. The same pattern shows up in every layer:

| Declared (repo) | Observed (machines) |
|---|---|
| Canonical gate blocks bad code | CI never calls the gate; `main` itself is `blocked` |
| Releases go through `releases/<tag>` + atomic switch | Debug binaries hand-copied, up to 3,049 commits behind main |
| `registry.json` is authoritative | 5 divergent copies, a phantom kernel IP, 3 unregistered LXCs |
| Backups, GC, health timers | Written and merged, never installed |
| `cynic` principal owns installed state and secrets | Desktop `user` has `NOPASSWD: ALL` and owns the fleet SSH keys |
| Observation → evaluation → action → proof | 0 alert rules; a 2-week crash loop (250k restarts) went unnoticed |

**Highest risks, in order:**
1. Compromising cortex `user` gives root on the whole fleet (C4-1, C4-2, C4-3).
2. There is no backup of anything: no vzdump job and no SurrealDB dump. All storage sits on one 2.5" HDD (C0-1, C0-12, C5-2).
3. Internet-facing services run on a hypervisor with no firewall (C0-2, C0-3, C0-10).
4. The storage node has no live CYNIC runtime because of a glibc mismatch (C0-11, C2-2).

---

## 2. Inventory observed (C0 baseline)

### Hypervisor `pve`
- **Software:** Proxmox VE 8.4.0, kernel 6.8.12-9-pve, up 15 days.
- **Hardware:** i5-6500T (4 cores), 15 GiB RAM.
- **Interfaces:** `eno1` 192.168.100.1 (admin), `wlp1s0` 192.168.0.37 (site LAN, Wi-Fi, default route), `vmbr0` 10.10.10.1 (service), `wg0` 10.20.0.1.
- **Storage:** `local` at 21 %, `local-lvm` at 28 %, both on `sda`, an HGST HTS541010 1 TB 2.5" HDD. `nvme0n1` reports "SSD 4TB" with a size of **0 B**.

### Containers

| CT | Hostname | IP | In registry? | Notes |
|---|---|---|---|---|
| 100 | organ-headscale | 10.10.10.40 | yes (`cynic-lxc-headscale`, candidate) | `tailscaled`, `systemd-networkd` failed |
| 101 | organ-forgejo | 10.10.10.101 | yes (`forgejo`, candidate) | wg 10.20.0.2 |
| 102 | cynic-agent-host | 10.10.10.102 | yes (observed) | `cynic`, `cynic-node`, `hermes` installed; **no service active** |
| 103 | cynic-lxc-surrealdb | 10.10.10.103 | yes (candidate) | Debian 12; `cynic-node` crash loop |
| 104 | cynic-build-bookworm | 10.10.10.104 | **no** | Debian 12 with cargo/rustc, 8 GiB, no `onboot` |
| 105 | cynic-builder | 10.10.10.119 | yes (candidate) | Ubuntu 24.04, disk 75–81 % |
| 106 | cynic-lxc-embedding | 10.10.10.106 | yes (observed) | llama-server embed |
| 107 | relay-107 | 10.10.10.127 (DHCP) | **no** | Kora mainnet + relay-gateway |
| 113 | agent-113 (stopped) | DHCP | **no** | `onboot: 1`, template base disk |

### Other hosts
- **Cortex** (Ubuntu, 13 GiB RAM, disk 76 %). Runs `cynic-kernel` as `cynic`, plus `hermes` (0.0.0.0:8644), `llama-server` (0.0.0.0:8081), cortex embedding, and `cynic-dns-service`.
- **compute-01** (192.168.0.25): Ryzen 7 5700U, 15 GiB RAM, disk 6 %. SSH works as `cynic`, ICMP is filtered. No `cynic*` units; ports 8080 and 8088 listen.
- **Laptop** (192.168.0.88): unreachable, which is expected for a portable.
- **Not in the registry but referenced elsewhere:** `192.168.0.12:3031`, the nginx `/demo/*` upstream, and `cynic-forge-vm` (10.10.10.60, from `~/.ssh/config`, unreachable).

### WireGuard `wg0` on pve
Overlay 10.20.0.0/24. Peers: 101 (10.20.0.2), 103 (10.20.0.4), 105 (10.20.0.5), and cortex (10.20.0.3 **plus 10.10.10.100/32**).

---

## C0 — Physical substrate

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C0-1 | 🔴 | **No scheduled backups.** `/etc/pve/jobs.cfg` is empty. The only vzdump is `vzdump-qemu-102-2026_06_16`, which is obsolete because 102 is now an LXC. Forgejo (101), SurrealDB (103) and the builder (105) have no backup at all. | `cat /etc/pve/jobs.cfg; ls /var/lib/vz/dump` on pve | Create vzdump jobs (snapshot mode) for 100, 101, 103, 105 and 107, with a target **off `sda`**. Prove it by listing the dump file and running `pct restore` into a scratch CT ID. |
| C0-2 | 🔴 | **Root SSH with password on the hypervisor.** `permitrootlogin yes` and `passwordauthentication yes`, listening on all interfaces including Wi-Fi LAN 192.168.0.37. `/root/.ssh/authorized_keys` has 5 keys: one without a comment (unidentifiable), plus `root@pve`, `kairos` (unknown to the registry), `cynic@cortex` and `cynic@ubuntu`. | `sshd -T \| grep -E 'permitroot\|passwordauth'` on pve | Set `PasswordAuthentication no` and `PermitRootLogin prohibit-password`. Identify the comment-less key and `kairos`, or remove them. Keep a console path open before reloading sshd. |
| C0-3 | 🔴 | **No firewall on the hypervisor.** `pve-firewall` is disabled and iptables policies are ACCEPT. nginx (80/443), `cynic-cloud-api` (:4000), pveproxy (:8006), rpcbind (:111), spiceproxy (:3128) and dnsmasq (:53) all bind `0.0.0.0`. `ip_forward=1`. | `pve-firewall status; iptables -S; ss -tlnp` | Enable pve-firewall. On `wlp1s0`, allow only 80/443 (if public exposure is intended) plus established traffic. Restrict 8006/22/111/3128 to `eno1` (admin). Read back with an external port scan. |
| C0-4 | 🟠 | **The kernel IP is a phantom.** The registry places `local-kernel` at 10.10.10.100, which does not answer ping. On pve, 10.10.10.100 is only an `allowed-ip` of cortex's wg peer. The kernel actually runs on cortex at `/run/cynic/kernel.sock` and `127.0.0.1:3030`. Already noted unresolved in `contracts/infrastructure-discovery.json:135`. | `ping 10.10.10.100`; `ip route` on pve; `wg show` | Decide the kernel's real placement, then update the registry, DNS (`kernel.service.cynic`) and the wg allowed-ips together. See C1-5. |
| C0-5 | 🟠 | **Unregistered running workloads:** CT104, CT107 (DHCP) and CT113 (stopped, `onboot: 1`). The reverse case also exists: `cynic-forge-vm` 10.10.10.60 is in ssh config but unreachable. | `pct list` | Either register each CT (static IP, role, status) or destroy it. Decide CT113's `onboot`. |
| C0-6 | 🟡 | **Hypervisor patching is behind:** 219 upgradable packages; the kernel 6.8.12-9 is old. | `apt list --upgradable \| wc -l` | Plan a maintenance window *after* backups (C0-1) exist. |
| C0-7 | 🟡 | **Overcommit.** Running CTs declare about 30 GiB RAM against 15 GiB physical. CT104 and CT105 claim 4 cores each, on a 4-core host. | `pct config <id>` | Lower CT104 to `onboot` 0 or reduce its RAM. Never build on 104 and 105 concurrently. |
| C0-8 | 🟡 | **Registry statuses are stale.** Running nodes are still `candidate`. `local-builder` is a role-less `stub` sharing the alias `ubuntu` with cortex. | `infra/registry.json` | Covered by C1-2 and C1-4. |
| C0-9 | 🔵 | **Cortex hygiene.** 4 failed units (apport, chrome-remote-desktop, fwupd-refresh, openipmi). `hermes` :8644 and `llama-server` :8081 bind `0.0.0.0`. `sshd` listens on 100.74.31.10, a Tailscale address that no active interface carries. | `systemctl --failed; ss -tlnp` | Bind `hermes` and `llama` to 127.0.0.1 or the wg address. Clean up the dead sshd ListenAddress. |
| C0-10 | 🟠 (revised from 🔴) | **Legacy `cynic-cloud-api` is internet-facing.** `api.talaria.build` → 82.65.26.228 → pve nginx → `/api/deploy` and `/health` → `127.0.0.1:4000`. The process runs **as root** with no `NoNewPrivileges` or `ProtectSystem`. It is a static binary from 2026-06-12 whose source is not in the repo; the repo calls it a *"legacy orphan"* (`crates/cynic-node/src/usecases/compute_provisioning_uc.rs:3`). Mitigation observed: the binary embeds ed25519 signature verification, and nginx logs show no `/api/deploy` hits. | `systemctl show cynic-cloud -p User,ExecStart`; `dig @1.1.1.1 api.talaria.build`; `strings` | Stop and disable `cynic-cloud.service`. Remove the `/api/deploy` location from `/etc/nginx/sites-enabled/api-talaria`. Archive the binary. |
| C0-11 | 🔴 | **The storage node's CYNIC runtime is dead.** `cynic-node` on CT103 has restarted **250,542 times**: `GLIBC_2.38/2.39 not found`. It was built on Ubuntu 24.04 and deployed on Debian 12 (glibc 2.36). | `systemctl show cynic-node -p NRestarts; journalctl -u cynic-node -n 5` on 103 | Stop the unit now, to end the crash loop and its log churn. Rebuild for glibc ≤ 2.36 (see C2-2) and redeploy through the release path. |
| C0-12 | 🔴 | **The NVMe is non-functional.** It reports "SSD 4TB" with size 0 B, a generic model string typical of a counterfeit or dead drive. All data lives on one non-redundant 2.5" HDD. | `lsblk -d -o NAME,SIZE,MODEL,ROTA` on pve | Replace the drive physically. Until then, C0-1 is the only protection. |
| C0-13 | 🟠 | **Public headscale is misrouted.** The `cynic-hs.duckdns.org` vhost proxies to `10.10.10.100:8080`, the phantom IP, where nothing listens. The real headscale is `10.10.10.40:8080`, which returns 200. | `/etc/nginx/sites-enabled/headscale:5`; `curl 10.10.10.40:8080/health` | Change `proxy_pass` to `http://10.10.10.40:8080`. Read back with the public `/health`. |
| C0-14 | 🟠 | **Binaries diverge everywhere.** `cynic` has 4 distinct sha256 values, all reporting `0.8.0`: cortex `6da2745f`, ~/.local `998ddf12`, 102 `b8e3e6b6`, 105 `cd62eaed`. `cynic-node` has 3 distinct hashes. `cynic-kernel` on cortex is a **debug** build. | `sha256sum /usr/local/bin/cynic*` per host | Covered by C2-1. |
| C0-15 | 🟠 | **Unregistered upstreams.** nginx proxies `/demo/*` to `192.168.0.12:3031`, a LAN host not in the registry. CT107 listens on 3000, 4021 and 14001. | `/etc/nginx/sites-enabled/api-talaria` | Register the host or remove the location. |
| C0-16 | 🟡 | **The builder is degraded.** Disk was at 81 % (126–137 GiB / 177 GiB); checkouts account for only 2 GiB, the rest is unexplained. Failed units: `forgejo.service`, a duplicate of CT101, and `cynic-supervise-soak-node.service` (the 30-day soak is dead). | `df -h /; systemctl --failed` on 105 | Find the large consumers (`du -xh --max-depth=3 /`). Remove the duplicate forgejo. Fix or retire the soak. |
| C0-17 | 🟡 | **All LXCs share the same entry posture.** Every one has `permitrootlogin without-password` plus `passwordauthentication yes`. Upgradable package counts range from 52 to 212. | `sshd -T` per CT | Disable password auth fleet-wide. Patch. |
| C0-18 | 🟡 | **SurrealDB is reachable from the whole flat network.** It binds `0.0.0.0:8000/8443` with no network filtering, so CT107 can reach it. | `ss -tlnp` on 103 | Bind to the wg address or firewall to the kernel and node only. |
| C0-19 | 🔵 | **compute-01 is not enrolled.** No `cynic*` units run there; 8080 and 8088 are open. | ssh `cynic@192.168.0.25` | Decide the role. Firewall the open ports. |

---

## C1 — Authority & registries

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C1-1 | 🔴 | **The kernel's authority file is writable by the desktop user.** On cortex, `/etc/cynic/registry.json` is owned `user:user` with mode `664`. The kernel loads it through `CYNIC_REGISTRY`, so any process running as `user` (every agent included) can rewrite the topology the kernel believes. This violates `infra/runtime-authority.json` → `installed_state`. | `stat /etc/cynic/registry.json` | Set `root:cynic 0640`. Make only the release or projection path write it. |
| C1-2 | 🔴 | **Registry divergence across the fleet.** Deployed copies — cortex `2ec7efaf…` (17/10), CT102 `a08af481…` (17/10), CT105 `3843e811…` (14/10) — match **none of the last 80 committed versions** on `origin/main`. CT103 and CT106 have no registry at all. Cortex's copy contains fields newer than the repo. Decision D1 in `infra/registry_authority.json` (repo vs runtime authority) is still `draft`. | `sha256sum /etc/cynic/registry.json` on each host, compared to `git log origin/main -- infra/registry.json` | Ratify D1. Then generate deployed registries only from a committed SHA, and stamp that SHA in the file. |
| C1-3 | 🟠 | **The overlay network has three contradictory definitions.** The registry plane `mesh` is `100.64.0.0/10` on `wg0`. The real `wg0` is `10.20.0.0/24`. Code uses plane name `"synapse"` (81 non-test references), which the registry does not declare, and hardcodes `MeshTopology::new("10.20.0.0/16")` in `crates/cynic-node/src/usecases/mesh_sync_uc.rs:38`. | registry `network_policy.planes`; `wg show`; grep | Pick one truth, likely `10.20.0.0/24`. Declare it in the registry and read it from the registry in `mesh_sync_uc`. |
| C1-4 | 🟠 | **The project's own validators fail on main.** `infra_registry_validate` → `valid:false`, `nodes[3] missing role` (`local-builder`). `network_ipam_check` → `coherent:false`: forgejo uses an undeclared plane `local`. The compute-01 duplicate is fixed on main. | MCP `cynic_*` tools | Fix the data, then make both validators a pre-merge gate (see C2-3). |
| C1-5 | 🟠 | **The registry reserves the phantom 10.10.10.100** with DNS `kernel.service.cynic` and `local-kernel.service.cynic`. | `network_policy.reservations` | See C0-4. |
| C1-6 | 🟡 | **Dangling authority references.** `runtime-authority.json` assigns `proof_history` to `crates/cynic-askesis`, which does not exist. `ownership.json` cites `crates/cynic-intel-escore`, which does not exist. There are 44 dead path references across 8 governance files, 31 of them in `infra/cynicpedia.json`. | path-existence scan over `contracts/*.json` and `infra/*.json` | Reassign the owners or delete the entries. |
| C1-7 | 🟡 | **The authority order differs** between AGENTS.md and `runtime-authority.json`, which adds `directory-tree.json` and `agent-exposure.json`. Of 73 governance JSON files, 21 have no `status`, 7 are `draft` and 3 are proposals. | `authority_order` | Align AGENTS.md with the JSON. Give every file a status. |
| C1-8 | 🔵 | **The retired "organ" vocabulary survives** in registry aliases and DNS (7), LXC hostnames (100, 101) and 31 Rust files, although `deleted_authorities` lists it. | grep | Low priority rename. |

**What holds in C1:** the crate census is consistent (directories ↔ `Cargo.toml` members ↔ topology), except for C3-3. Production code has almost no hardcoded IPs; nearly all hits are test fixtures.

---

## C2 — Build & delivery chain

**Measured on the builder against `d88f1312`:**
- `make audit` → `status: blocked`.
  - crates 41/42 ok, 1 blocked (the report does not name it).
  - engine: 17/17 rules evaluated, 121 critical findings.
  - secrets: 2,618 files scanned, 0 findings.
- `cargo deny check`:
  - advisories: 0 vulnerabilities, 2 unmaintained (`instant` RUSTSEC-2024-0384, `paste` RUSTSEC-2024-0436), 1 yanked (`tokio 1.54.0`).
  - licenses: 0 rejected.
  - sources: 0 problems.
  - bans: 38 duplicate-version warnings.

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C2-1 | 🔴 | **Production runs stale debug binaries off-path.** On cortex, `cynic-kernel` reports `26.6.12 (build 26.08.11-c26e4d7, profile debug)`, **3,049 commits behind main**. The cortex `cynic` CLI is a debug build of `0bbbccb7`, 1,743 behind, with empty `build_proof_sha`. `cynic-node` on 102 and 103 was copied straight into `/usr/local/bin` on 2026-09-01. Only CT105 uses a `releases/v0.8.0` symlink. | `cynic version --json`; `readlink -f`; `git rev-list --count <sha>..origin/main` | Do a release build on CT105, then `make release.deploy` per host. Gate the result with `make audit.deployed` on each host. |
| C2-2 | 🔴 | **No glibc target policy.** Builds happen on Ubuntu 24.04 (glibc 2.39) and deploy to Debian 12 LXCs (glibc 2.36); this causes C0-11. No contract mentions glibc, musl or bookworm. CT104 `cynic-build-bookworm` exists with cargo and rustc but has no declared role. | grep in `infra/` and `contracts/` | Declare a target per `host_family` in build policy: either build Debian targets on CT104, or ship `x86_64-unknown-linux-musl`. Add a deploy-time check that the binary's max GLIBC version is ≤ the host's. |
| C2-3 | 🔴 | **CI is not a gate.** `.github/workflows/ci.yml` runs only on push to `main` or `sync/*`, i.e. after the Forgejo merge. It runs `cargo check --workspace` and tests for **4 of 42 crates**. `contracts/release-gates.json` requires `cargo test --workspace`, `cynic secret lint`, an architecture boundary check and a build proof. `ci-sync.yml` failed on 2026-10-21. | `ci.yml`; `gh run list -R zeyxx/CYNIC` | Run `cynic audit all --json` and `cargo deny check` as **required Forgejo PR checks** on the builder runner, not on cortex. |
| C2-4 | 🟠 | **The builder SSOT is dirty.** `/home/user/CYNIC` on CT105 has 168 modified files and HEAD `9d2f3cf0` from 18/10. Four other clones exist: `verify`, `verify2`, `probe-clone` (dirty) and `agent-runtime-test`. None is at `origin/main`. | `git status` in each | Build only from per-SHA checkouts (`/var/lib/cynic/checkouts/cynic-<sha>`). Retire the ad-hoc clones. |
| C2-5 | 🟠 | **`cargo-deny` and `cargo-audit` never run.** They are installed only on cortex (where builds are forbidden) and are absent from the builder. No Makefile target or CI step invokes them. | `ls ~/.cargo/bin` | Install them on CT105 and add `make deny` plus the CI step. Fix the 2 unmaintained crates and the yanked tokio. |
| C2-6 | 🟠 | **`infra/binary-locations.json` is false.** It claims cortex, the builder and 102 are `current` at sha256 `944753f2…`; the observed hashes differ. It also lists the cortex path `/home/user/.local/bin/cynic`, which violates `installed_state`. | `sha256sum` | Generate this file from observation, or delete it. |
| C2-7 | 🟡 | **Versions are not traceable.** Four different binaries all claim `cynic 0.8.0`. `cynic-node --version` does not exist. The kernel uses a calendar version unrelated to the `v0.x` tags. | `--version` | Embed the git SHA and build profile in every binary, and expose `version --json` everywhere. |
| C2-8 | 🟡 | **Gate naming is muddled.** `make check` is deprecated. `contracts/build-placement.json` cites `make:build` and `make:build-release`, which do not exist. Test and WIP tags (`test-artifact-probe`, `wip/…`, `audit/…`) are mixed with release tags. | Makefile; `git tag` | Fix the contract text. Namespace or prune the non-release tags. |
| C2-9 | 🔴 | **`main` fails its own canonical gate.** There are 69 new undeclared debts: 50 `ARCH-001-HOTSPOT`, 6 `SEC-001-ABSOLUTE-PATH`, the **undeclared crate `cynic-supervisor`** (already depended on by cli, inference and node), and 3 systemd units missing from the registry (`cynic-kernel-proxy`, `cynic-supervise-soak-node`, `cynic-supervise-watch-soak-no…`). There are also 52 stale ledger entries for debts already paid. | `make audit` on clean `d88f1312` | Reconcile `infra/audit-debt-ledger.json` and `infra/crate-topology.json` in one PR. Then make the gate required (C2-3). |
| C2-10 | 🟠 | **The builder working repo is corrupt.** `/home/user/CYNIC` on CT105 is missing tree `ab5f9445…` and blob `ab1bd0a9…`, which `.git/index` references through its cache-tree; uncommitted state only. The cause is not established; the native build-artefact GC landed on 2026-10-21, which may be relevant. | `git fsck --no-dangling` | Do not build from it. Recover the needed uncommitted work, then re-clone. Check whether the GC can touch `.git`. |
| C2-11 | 🟡 | **The audit report contradicts itself.** It shows `maturity_score_percent: 100` and `debt: 0` next to `status: blocked` with 121 criticals, and counts one blocked crate without naming it. | `make audit` JSON | Derive the score from the findings. Name the blocked crate. |
| C2-12 | 🔵 | `tokio 1.54.0` is yanked; `instant` and `paste` are unmaintained. | `cargo deny` | Run `cargo update -p tokio`. Replace `instant`/`paste` or track them as declared debt. |

**What holds in C2:**
- `rust-toolchain.toml` pins 1.95.0, documented in `infra/rust-toolchain.json`.
- `.cargo/config.toml` justifies `jobs` with measurements.
- `deny.toml` is well-formed.
- `contracts/runtime-update.json` (gate → stage → atomic switch → rollback) and `make audit.deployed` are well designed; neither is used.

**Unexplained (not a finding):** a `git bundle` of `origin/main` created on cortex could not be fetched on CT105 (`missing blob b8baead2…`). Cortex `git fsck --connectivity-only` is clean. The audit used `git archive` instead.

---

## C3 — Code architecture (on `d88f1312`)

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C3-1 | 🟠 | **14 upward DAG edges.** `cynic-host` (L1) depends on 9 L2 crates: build, eval, finance, inference, intel, node-protocol, storage, vcs, vcs-providers. `cynic-lifecycle` (L1) depends on build and storage. `cynic-network` (L1) depends on finance. `cynic-workload` (cross-cutting) depends on consensus and finance. These are declared in the debt ledger, but `cynic-host` behaves as an L3 orchestrator, so its *classification* is wrong, not only its edges. | Cargo.toml deps vs `infra/crate-topology.json` layers | Either reclassify `cynic-host`, or split it into a thin L1 host-port crate and move the orchestration up. |
| C3-2 | 🟡 | **FCIS violations, with I/O inside `domain/`:** `cynic-host/src/domain/storage/{apply,local_adapter,fleet_observer}.rs` (`std::fs::write` and read-back), `cynic-knowledge/src/domain/{vector_cache,synthesis}.rs`, and `cynic-node/src/domain/observation_plan.rs`. | grep for `std::fs\|Command::new\|tokio::(fs\|net\|process)` in `*/src/domain` | Move these into `adapters/` behind ports. |
| C3-3 | 🟡 | **`cynic-supervisor` is missing from `infra/crate-topology.json`** (42 crates on disk, 41 in the topology). | topology vs `crates/` | Add it with its layer. Fixes part of C2-9. |

**What holds in C3:**
- **L0 is pure.** `cynic-protocol` and `cynic-types` have no I/O outside tests.
- **File sizes are reasonable.** The largest file has 1,624 lines (`cynic-cli/src/agent_surface/coordination.rs`).
- **Production panics are rare.** There are 138 `unwrap`/`expect`/`panic` sites in about 374k LOC, mostly Mutex locks, and zero `todo!`/`unimplemented!`.

---

## C4 — Security

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C4-1 | 🔴 | **The desktop/agent user is root.** `/etc/sudoers.d/temp-claude` contains `user ALL=(ALL) NOPASSWD: ALL`, dated **2026-04-23** ("temp", 6 months old). `cynic-tools` (NOPASSWD `find` and `cat`) and `claude-code` (NOPASSWD `systemctl` and `apt-get`) are each a root escalation on their own. `user` is also in group `docker` (root-equivalent). Every agent session, Hermes and Claude included, runs as `user`. | `sudo -n true` succeeds; `ls /etc/sudoers.d` | Remove `temp-claude` and `cynic-tools`. Narrow `claude-code` to specific units. Remove `user` from `docker`. Keep a root console open while doing it. |
| C4-2 | 🔴 | **There is no blast-radius boundary.** Cortex holds root SSH to pve (`cynic@cortex` key in pve's `authorized_keys`) and to CT105 (`root@10.10.10.119` in `~/.ssh/config`). From pve, `pct exec` reaches every CT, including CT107's signing key. | the audit's own SSH passes | Use a dedicated, non-root, forced-command identity per actuator. Remove the cortex root key from pve or restrict it with `from=` and `command=`. |
| C4-3 | 🔴 | **Secrets are readable by the desktop user.** `user` is in group `cynic`, so it reads every `root:cynic 0640` secret (`forgejo-token`, `headscale-api-key`, `domain.env`, `kernel-runtime.env`). `user` also **owns** the fleet SSH private keys in `/etc/cynic/secrets/`: `agent-host-ssh`, `compute-01-ssh`, `headscale-ssh`, `laptop-ssh-authority`. This violates `installed_state`. | `ls -la /etc/cynic/secrets`; `id` | Remove `user` from group `cynic`, and give agents an `cynic-agent` socket-only path instead. `chown cynic:cynic` the keys. Rotate every secret that `user` could read. |
| C4-4 | 🟠 | **A mainnet payment relay runs on an unregistered DHCP CT.** CT107 runs `kora --rpc-url https://api.mainnet-beta.solana.com` and `gasdf-relay-gateway` (index and facilitator), and holds the hot signing key `/etc/cynic/secrets/gasdf/relay/upto-authorizer.json` (`root:640`), on a flat, unfirewalled network. `npm audit` shows 0 vulnerabilities: the code is fine, the placement is not. | `ps` in CT107 | Register CT107 with a static IP. Isolate it by firewall (egress to the Solana RPC, ingress only from the gateway). Hold the key with the least privilege possible. |
| C4-5 | 🟠 | **Inference endpoints are unauthenticated.** Cortex `0.0.0.0:8081/v1/models` returns 200 without a key, and `/completion` returns 500 (not 401) without one. CT106 `:8080/v1/embeddings` returns 200 without a key. CT103 SurrealDB `/health` is reachable fleet-wide. `/etc/cynic/llama-api-key` exists, but these responses show it is not enforced everywhere. | `curl` with no auth header | Enforce `--api-key` on every llama-server. Bind to localhost or wg. |
| C4-6 | 🟠 | **`programs/cynic-anchor` is unsafe.** It is not deployed: the program ID is the placeholder `CYNiC111…`, with localnet and devnet only. It must not be deployed as is: (a) **any signer** can call `resolve_futarchy_market`, which has no `has_one` or oracle constraint; (b) `place_futarchy_bet` **moves no tokens**, so pools can be inflated for free; (c) in `execute_ignis_burn`, `token_program` is an unchecked `AccountInfo`, which allows **program substitution**: a fake program inflates `total_burned_units` without any burn. `AccountInfo` fields have no `/// CHECK`, and `+=` is unchecked. | `programs/cynic-anchor/src/lib.rs:51-196` | Add an oracle/authority constraint and status/time guards. Do a real token transfer into an escrow PDA. Use `Interface<TokenInterface>` / `Program<Token>` and typed `InterfaceAccount<Mint/TokenAccount>`. Use `checked_add`. Have it reviewed before any devnet deploy. |
| C4-7 | 🟡 | **C0-10 security view.** The ed25519 check lowers the risk, but this is a root, sourceless, internet-facing legacy binary. | see C0-10 | Retire it. |
| C4-8 | 🟡 | **sudo grants `network-actuate` with unrestricted arguments.** `/etc/sudoers.d/cynic-host-exec` grants `cynic` the command `cynic-host-exec network-actuate` with no argument list; sudo treats that as *any arguments*. | sudoers file | Pin the exact argv, or make the binary reject anything outside its policy file. |
| C4-9 | 🔵 | **1,753 distinct external IPs** in pve nginx logs; this is normal scanner background noise. | `access.log` | Covered by C0-3. |

**What holds in C4:**
- `cynic secret lint` found 0 hits in 2,618 files.
- MCP runs over stdio only, with read-only tools.
- The relay-gateway has 0 npm vulnerabilities and uses an HMAC-authenticated Kora client.
- `principal-seed` and the Freebox and Porkbun tokens are `cynic 0600`.
- `cynic` sudoers are subcommand-scoped, except C4-8.

---

## C5 — Operations, supervision, observability

| ID | Sev | Finding | Evidence | Remediation |
|---|---|---|---|---|
| C5-1 | 🔴 | **There is no alerting.** Cortex Prometheus has **0 rule groups** and **no Alertmanager**. Its only CYNIC target, `http://100.74.31.10:3030/metrics` (a dead Tailscale IP), is `down`. Only `node_exporter` and Prometheus self-metrics are collected. | `curl 127.0.0.1:9090/api/v1/{targets,rules,alertmanagers}` | Fix the target to `127.0.0.1:3030`. Add a node_exporter on pve and each CT. Add minimum rules: `up == 0`, a systemd restart count over a threshold, disk > 85 %, and the newest backup older than 26 h. Add an Alertmanager with one real channel. |
| C5-2 | 🔴 | **The SurrealDB backup is written but not installed.** `infra/os/systemd/surrealdb-backup.{service,timer}` (daily, 7/14-day retention) exists in the repo but is **absent on CT103**, whose `/var/backups` holds only dpkg files. Together with C0-1, the database has **zero backups at any level**. | `systemctl list-timers --all` on 103 | Install and enable the unit. Run it once. Prove it with a restore into a scratch DB. |
| C5-3 | 🟠 | **Health timers are installed but never run:** `cynic-healthcheck.timer`, `cynic-monitor.timer` and `cynic-dog-monitor.timer` on cortex show no last or next run. | `systemctl list-timers --all` | Enable them, or delete them if superseded. |
| C5-4 | 🟠 | **The build GC is merged but not deployed.** PR #224 (`f611d873`, *"arm the native build artefact GC on a daily timer"*) is on main, but CT105 has no GC timer. Checkouts use 2 GiB; the rest of the disk usage is unexplained (C0-16). | `list-timers` on 105 | Deploy through the release path. This is the C2-1 pattern again. |
| C5-5 | 🟠 | **Watchdogs check liveness, not function.** `surreal-watchdog` (CT103) checks only SurrealDB `/health` and ignores the crash-looping `cynic-node` on the same CT. `cynic-main-watchdog.sh` (cortex cron) writes `AGREE <sha>` every 15 minutes and nothing reads it. | watchdog logs | Feed the watchdog output into Prometheus (textfile collector) so that C5-1's rules see it. |
| C5-6 | 🟡 | **Supervision lives outside the principal.** The crons `cynic-main-watchdog.sh` and `cynic-ram-guard.sh` run from `/home/user/.local/bin`. The latter starts and stops a **Forgejo runner on cortex**, although `contracts/build-placement.json` forbids building on cortex (cortex also has faulty RAM). | `crontab -l` | Move the runner off cortex. Move the supervision into `cynic`-owned systemd units. |
| C5-7 | 🟡 | **An unbounded self-healing action runs on the hypervisor.** `cynic-hypervisor-watchdog.sh` runs every 5 minutes; on detection it does `kill -9 pmxcfs`, `rm -rf /dev/shm/qb-*` and `restart pve-cluster`, with no rate limit or lease. It has fired 4 times, all on 2026-10-06. | `/var/log/cynic-watchdog.log` | Add a max-N-per-hour limit with escalation (an alert) instead of an endless restart loop. |
| C5-8 | 🔵 | **Logs are unbounded and unused.** journald uses 0.75–1.5 GiB per host. `/var/log/cynic` holds only 8 KiB, so components do not log there. The pve watchdog log is 470 KiB with no rotation. | `journalctl --disk-usage` | Set `SystemMaxUse`. Add logrotate for the watchdog log. |

---

## 8. Remediation order (recommended)

Each step is a separate scoped claim and branch (`agent/hermes/<finding-id>`), needs operator approval, and ends with read-back proof.

1. **Access (cortex)** — C4-1, C4-3, C1-1: remove `temp-claude` and `cynic-tools`, remove `user` from `cynic` and `docker`, fix `/etc/cynic` ownership, rotate the exposed secrets. *Keep a root console open.*
2. **Data safety** — C0-1, C5-2: vzdump jobs off `sda`, and the SurrealDB backup timer. Prove each with a restore.
3. **Hypervisor exposure** — C0-2, C0-3, C0-10, C0-13: sshd hardening, pve-firewall on `wlp1s0`, retire `cynic-cloud-api`, fix the headscale vhost.
4. **Broken production** — C0-11, C2-2: stop the crash loop on CT103, declare the glibc/musl target, rebuild and redeploy `cynic-node`.
5. **Close the loop** — C2-3, C2-5, C2-9, C5-1: reconcile the debt ledger and topology, make `cynic audit all` and `cargo deny` required Forgejo PR checks, add the minimum Prometheus alert rules.
6. **Release convergence** — C2-1, C2-6, C2-7, C5-4: release builds through `make release.deploy`, `make audit.deployed` green on every host, regenerate `binary-locations.json`.
7. **Registry truth** — C1-2 to C1-5, C0-4, C0-5, C4-4: ratify D1, register or destroy CT104, CT107 and CT113, settle the overlay CIDR and the kernel placement.
8. **Hardware** — C0-12: replace the NVMe, and add redundant storage.
9. **Architecture debt** — C3-1 to C3-3, C4-6, C1-6 to C1-8.

---

## 9. Re-verification checklist (exit criteria)

- [ ] `sudo -n true` as `user` on cortex **fails**.
- [ ] `id user` contains neither `cynic` nor `docker`.
- [ ] `/etc/pve/jobs.cfg` contains a vzdump job; the newest dump is < 26 h old; a test restore succeeded.
- [ ] CT103 has `surrealdb-backup.timer` active; the newest dump is < 26 h old.
- [ ] `sshd -T` on pve shows `passwordauthentication no`.
- [ ] `pve-firewall status` shows `enabled/running`; an external scan of 192.168.0.37 shows only the intended ports.
- [ ] `cynic-cloud.service` is inactive and `/api/deploy` returns 404 publicly.
- [ ] `systemctl show cynic-node -p NRestarts` on CT103 stays stable over 1 h.
- [ ] `make audit` on a clean `origin/main` returns `status != blocked`.
- [ ] A Forgejo PR cannot merge without the audit and deny checks.
- [ ] `make audit.deployed` passes on every deploy target.
- [ ] Prometheus has ≥ 1 rule group, an active Alertmanager, and every `cynic` target is `up`.
- [ ] `infra_registry_validate` → `valid:true`; `network_ipam_check` → `coherent:true`.
- [ ] Every deployed `/etc/cynic/registry.json` hash equals a committed version.

---

## 10. Evidence index (commands used, all read-only)

```bash
# C0 hypervisor (as root@192.168.100.1)
pveversion; pct list; pct config <id>; pvesm status; lsblk -d -o NAME,SIZE,MODEL,ROTA
smartctl -H /dev/sda; cat /etc/pve/jobs.cfg; ls -la /var/lib/vz/dump
ip -4 -br a; ip route; pve-firewall status; iptables -S; ss -tlnp
sshd -T | grep -E '^(permitrootlogin|passwordauthentication)'
wg show wg0   # keys redacted in the report
grep -hE 'server_name|proxy_pass|listen' /etc/nginx/sites-enabled/*
systemctl show cynic-cloud -p User,ExecStart,NoNewPrivileges,ProtectSystem
# C0 inside CTs
pct exec <id> -- sh -c 'ss -tlnH; systemctl --failed; systemctl list-units "cynic*"; sshd -T'
pct exec 103 -- journalctl -u cynic-node -n 8; systemctl show cynic-node -p NRestarts
# C1
sha256sum /etc/cynic/registry.json   # per host, compared to git log origin/main -- infra/registry.json
# MCP: cynic_infra_registry_validate, cynic_network_ipam_check
# C2 (builder, ephemeral snapshot of d88f1312, deleted afterwards)
make audit; cargo-deny --format json check advisories licenses bans sources
git -C /home/user/CYNIC fsck --no-dangling
cynic version --json; cynic-kernel --version; git rev-list --count <sha>..origin/main
# C4
sudo -n true; ls -la /etc/sudoers.d /etc/cynic/secrets; id
curl -s -o /dev/null -w '%{http_code}' <endpoint>   # no auth header
# C5
curl -s 127.0.0.1:9090/api/v1/{targets,rules,alertmanagers}
systemctl list-timers --all; crontab -l
```
