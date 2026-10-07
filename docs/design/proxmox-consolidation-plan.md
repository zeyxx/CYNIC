# Hypervisor + Build Consolidation Plan

**Status:** IN PROGRESS
**Schema:** `cynic.hypervisor_consolidation.v1`
**Authority:** directive, not contract. Changes no gate, no ledger, no threshold.

---

## 0. Why this exists

The Proxmox/hypervisor control surface is fragmented across three crates with
overlapping traits, inverted dependencies, and no single path for credentials.
The builder is a SPOF that crashes under load. The hypervisor IPC corrupts
repeatedly with no watchdog. The scheduler creates workspaces but never purges
them. Secrets are fragmented with no capability-based vault.

Measured 2026-10-06. Confirmed by live incidents.

---

## 1. Incident data — 2026-10-06

### 1.1 Symptom

`ssh cynic-builder` → `Connection timed out during banner exchange` (recurrent)

### 1.2 Root cause chain

| layer | what happened | how it manifested |
|---|---|---|
| `pmxcfs` (stale PID) | old process held IPC files in `/dev/shm/qb-*` | new `pmxcfs` could not create its own IPC |
| `pvedaemon` | could not connect to `pmxcfs` | `qm list` / `pct list` returned `ipcc_send_rec` errors |
| `sshd` on builder | could not authenticate (PAM depends on `pmxcfs`) | TCP SYN accepted, banner never sent |
| `sudo` on PVE | `pmxcfs` lockfile stale | `systemctl restart pve-cluster` failed with timeout |
| builder disk | `vm-105-disk-0` at 99.95% | I/O hang, D-state processes, load 27 |
| builder RAM | 8 GiB, no swap headroom | OOM killer, sshd starved |

### 1.3 Watchdog lessons (data-collected, 2026-10-06)

**Real corruption signals** (from the three genuine incidents):
1. `pmxcfs` unmounted (`mount` shows no `/etc/pve` fuse entry)
2. `pvedaemon` inactive
3. Functional probe fails: `pct list` emits `ipcc_send_rec` errors
4. Stale `pmxcfs` process holding IPC files while a new one cannot start

**False signals (v1 watchdog killed a healthy pmxcfs every 5 min):**
- `.pmxcfs.lockfile` presence — this is a NORMAL pmxcfs runtime artifact,
  recreated on start. Its presence is not corruption.
- `ipcc_send_rec` failures from SSH as user `cynic` (uid 1001) — pmxcfs
  rejects IPC connections from non-root users BY DESIGN
  (`[ipcs] crit: connection from bad user 1001! - rejected`). The daemon
  was healthy; the unprivileged caller was refused.

**Proven recovery** (for a genuine stale-pmxcfs incident):
```bash
kill -9 <stale_pmxcfs_pid>
rm -rf /dev/shm/qb-*
systemctl restart pve-cluster pvedaemon
```

**Deployed**: watchdog v2 at `/usr/local/bin/cynic-hypervisor-watchdog.sh`
on pve (systemd timer, 5 min interval) — probes functionally, distinguishes
real corruption from the uid-1001 IPC refusal, logs to
`/var/log/cynic-watchdog.log`. First ticks: `probe_ipcc_err=0`, stable.

**Consequence**: agent-side hypervisor mutation needs a root membrane
(sudoers for `pct`/`qm`/`pvesh` or a PVE API token) — tracked in Phase 7.
Until then, hypervisor mutations go through the operator root relay.

### 1.4 What was NOT needed

- Full node reboot (tried first, did not help)
- Reinstalling Proxmox packages
- Restoring from backup

### 1.5 What CYNIC could NOT do

- Diagnose the failure (no hypervisor control surface)
- Recover the builder (no API access without `pvedaemon`)
- Execute privileged commands (sudo requires password, no capability-based auth)
- Observe build progress (no log route in HTTP API)
- Purge stale workspaces (no GC mechanism)

---

## 2. Current fragmentation

### 2.1 Traits (3 overlapping)

| trait | crate | layer | consumers |
|---|---|---|---|
| `HypervisorObserver` + `HypervisorController` | `cynic-protocol` | L0 | **zero** (orphan) |
| `HypervisorPort` | `cynic-node` | L3 | **removed** (orphan, zero impl) |
| `RemoteExecutionPort` | `cynic-host` | L1 | `ProxmoxSshCloneAdapter` |

### 2.2 Adapters (9 files, 2 crates)

| adapter | crate | layer | LOC | protocol |
|---|---|---|---|---|
| `ProxmoxSshCloneAdapter` | `cynic-host` | L1 | 233 | SSH |
| `ProxmoxDiskExtensionAdapter` | `cynic-host` | L1 | 129 | SSH |
| `ProxmoxSubstrateAdapter` | `cynic-host` | L1 | 166 | SSH |
| `ProxmoxRestAgentHostObserver` | `cynic-host` | L1 | 809 | REST |
| `ProxmoxSshAgentHostObserver` | `cynic-host` | L1 | 335 | SSH |
| `ProxmoxControlAdapter` | `cynic-node` | **L3** | 879 | REST |
| `ProxmoxStorageAdapter` | `cynic-node` | **L3** | 204 | REST |
| `ProxmoxLxcBridge` | `cynic-node` | **L3** | ? | REST |
| `ProxmoxFleetInventory` | `cynic-node` | **L3** | ? | REST |

**Inversion:** 4 adapters (1 083+ LOC) are in L3 (`cynic-node`), not L1.

### 2.3 Type duplication

| type | location 1 | location 2 | status |
|---|---|---|---|
| `VmState` | `cynic-protocol/src/host_substrate/hypervisor.rs:31` | `cynic-protocol/src/host_substrate/proxmox.rs:7` | **fixed** (Stage 1) |
| `DiskController` | `cynic-protocol/src/host_substrate/hypervisor.rs:52` | `cynic-protocol/src/host_substrate/proxmox.rs:17` | **fixed** (Stage 1) |

### 2.4 Credential paths (fragmented)

| path | mechanism | problem |
|---|---|---|
| `CYNIC_PROXMOX_TOKEN_SECRET` | env var | exposed in process list |
| `/etc/cynic/secrets/proxmox-token` | file | correct, but no rotation |
| `sudo` on PVE | password | no capability-based auth |
| `pvesh` NOPASSWD | sudoers | only one command, no `systemctl` |

### 2.5 Secrets (`cynic-secrets`, 339 LOC)

| component | LOC | role |
|---|---|---|
| `ports.rs` | 86 | `SecretProviderPort`, `SecretStorePort`, `SecretUseGatePort` |
| `protocol.rs` | 106 | `SecretStatus`, `SecretEntry`, `SecretError`, `SecretUseHandle` |
| `composite.rs` | 82 | `CompositeSecretProvider` (env → file → keyring) |
| `lib.rs` | 65 | re-exports + helpers |
| `adapters/` | — | env, file, keyring, memory |

**Problems:**
- No vault centralisé capability-based
- Credentials fragmentés (env, fichier, keyring) sans rotation automatique
- `SecretUseGatePort` exists but scheduler doesn't use it
- No unified audit of secret access

### 2.6 Identity (`cynic-identity`, 185 LOC)

| component | LOC | role |
|---|---|---|
| `types.rs` | 138 | `NodeId`, `NodeIdentity`, `SigningMaterial` |
| `error.rs` | 24 | `IdentityError` |
| `lib.rs` | 23 | re-exports, `no_std` compatible |

**Problems:**
- `SigningMaterial` exposes ed25519 private key without rotation
- No binding between identity and capabilities
- `no_std` but no vault for keys

### 2.7 Builder state

| metric | value |
|---|---|
| Disk | 118G, 52G free (55%) |
| RAM | 8 GiB, no swap headroom |
| Worktrees | 1 active, 3 prunable |
| Workspaces `build-*` | 8+ residual (~26 Mo each) |
| Branches local | 5 |
| Branches remote | 13 |
| `target/` | root-owned (permission denied for cynic) |

### 2.8 Scheduler (`cynic-node`)

| problem | evidence |
|---|---|
| `completion_not_admitted` | 18 revisions, no execution |
| Spawn silent | no cargo/make process after claim |
| `package` field ignored | all submissions become `make:check` |
| No log route | only `state`/`completion` in API |
| No workspace purge | 8+ residual workspaces |

---

## 3. Target architecture

### 3.1 Crate taxonomy

| crate | layer | responsibility |
|---|---|---|
| `cynic-protocol` | L0 | pure types, ports, schemas |
| `cynic-host` | L1 | local substrate: CPU, disk, network, processes, secrets |
| `cynic-proxmox` | L1 | all Proxmox: adapters, watchdog, recovery, credentials |
| `cynic-janitor` | L1 | GC artefacts: workspaces, build artefacts, disk |
| `cynic-node` | L3 | runtime orchestration, scheduler, workload lifecycle |
| `cynic-cli` | L4 | thin CLI projection |
| `cynic-secrets` | cross-cutting | secret management, vault, gatekeeper |
| `cynic-identity` | cross-cutting | node identity, signing material |

### 3.2 Taxonomy (agnostic, in `cynic-protocol`)

```rust
pub enum HypervisorKind {
    Proxmox,
    VMware,
    Cloud,
    Libvirt,
}

pub enum ContainerKind {
    Lxc,
    Qemu,
    Docker,
    Process,
}

pub enum VmState {
    Running,
    Stopped,
    Paused,
    Suspended,
    Unknown,
}
```

### 3.3 Single trait (in `cynic-protocol`)

```rust
pub trait HypervisorObserver: Send + Sync {
    fn observe_hypervisor(&self) -> Result<HypervisorObservation, String>;
    fn observe_vm(&self, vmid: u32) -> Result<VmObservation, String>;
}

pub trait HypervisorController: Send + Sync {
    fn control_vm(&self, op: &VmControlOp) -> Result<VmControlResult, String>;
    fn validate_control(&self, op: &VmControlOp) -> Vec<String>;
}
```

### 3.4 Single adapter (in `cynic-proxmox`, L1)

```rust
pub struct ProxmoxAdapter {
    client: reqwest::Client,
    base_url: String,
    token: String,  // from SecretUseGatePort, never in code
}
```

### 3.5 Credential flow

```
SecretUseGatePort → /etc/cynic/secrets/proxmox-token → ProxmoxAdapter
```

No env vars, no hardcoded paths, no sudo password.

### 3.6 CLI surface (agnostic)

```bash
cynic hypervisor list [--json]
cynic hypervisor status [--json]
cynic hypervisor start <id> [--yes]
cynic hypervisor stop <id> [--yes]
cynic hypervisor reboot <id> [--yes]
cynic hypervisor extend-disk <id> <size-gb> [--yes]
cynic hypervisor snapshot <id> <name> [--description <text>]
cynic hypervisor rollback <id> <name>
cynic hypervisor health [--json]
cynic hypervisor recover [--yes]
```

No "proxmox" in the CLI vocabulary. The adapter is injected, not named.

### 3.7 Watchdog (in `cynic-proxmox`)

```rust
pub trait HypervisorWatchdogPort: Send + Sync {
    fn observe_health(&self) -> Result<HypervisorHealthReport, String>;
    fn recover(&self, report: &HypervisorHealthReport) -> Result<HypervisorRecoveryResult, String>;
}
```

### 3.8 Janitor (in `cynic-janitor`)

```rust
pub trait JanitorObservationPort: Send + Sync {
    fn observe(&self) -> Result<JanitorReport, String>;
    fn plan(&self, report: &JanitorReport) -> JanitorPlan;
}

pub trait JanitorExecutionPort: Send + Sync {
    fn execute(&self, plan: &JanitorPlan) -> JanitorResult;
}
```

---

## 4. Consolidation plan

### Phase 0 — Hypervisor correction (immediate)

- [x] Clean IPC corrupted + lockfile stale (three genuine incidents recovered)
- [x] Verify `pmxcfs` mounted + `pvedaemon` connected
- [x] Prove `pct list` + `qm list` functional
- [x] Watchdog v2 deployed (systemd timer, 5 min): distinguishes real corruption
      from the uid-1001 IPC refusal; logs to `/var/log/cynic-watchdog.log`
- [x] Discovery: `ipcc_send_rec` from SSH-as-cynic is a BY-DESIGN refusal, not
      corruption — hypervisor mutations need a root membrane (Phase 7)

### Phase 1 — Ephemeral build container (provisioning, agnostic, formalized)

**Dogfooding rule**: the path being built must serve to build itself. The
full reference slice is `provision → enroll → build (cynic) → deploy →
prove → destroy`, executed with the `cynic` binary it produces. Every
domain below is a pure port + an adapter, anchored in a live incident from
this session.

Dogfooding checklist (each row is one port/adapter pair, validated by the
reference slice):

| # | domain | anchor (2026-10-06 session) |
|---|---|---|
| 1 | Provisioning VM/container | named — Phase 1 core |
| 2 | System/network administration | named — user account, SSH, net0, bridge |
| 3 | Standalone cynic binary deployment | named — the container runs `cynic` to steer itself |
| 4 | Dependency install on a virgin machine | named — pinned rustup, git, build-essential |
| 5 | Cold secret distribution | virgin container cloning forgejo — token/SSH key via `SecretUseGatePort`, never plaintext env |
| 6 | Identity enrollment | container receives identity (30-day enrollment grant, signing keys) — `cynic-identity` |
| 7 | Exact source materialization | clone exact SHA, transport resolved from registry (never hardcoded), dirty-worktree protection |
| 8 | Observed toolchain pin | `rustc 1.95.0` observed from inside the container, never assumed (ICE lesson: tool identity is read, not declared) |
| 9 | Native teardown/GC | destroy after read-back — the janitor is born here: 8+ residual workspaces exist because this is missing |
| 10 | Build observability from birth | log/progress route — the current scheduler is a black box (`claimed` with zero execution) |
| 11 | Container health watchdog | disk 99.95%, RAM 8 GiB, load 27 — same probes as the pmxcfs watchdog, applied to the guest |
| 12 | Bounded network policy | forgejo egress only, no internet — bwrap `--unshare-net` already exists (`cynic-host/sandbox`) |
| 13 | Registry-driven placement | which hypervisor, which template, which role — from `infra/registry.json` |
| 14 | Self-hosting loop | the `cynic` binary compiled in the ephemeral provisions the next one — CYNIC deploying CYNIC bootstrap ratchet |

- [x] `ProvisioningPort` (pure trait) + `ProvisioningSpec` types in `cynic-protocol`
      (proven: 8/8 tests, `cargo test -p cynic-protocol --lib
      host_substrate::provisioning` on builder checkout @ e50942da)
- [ ] `ProxmoxProvisioningAdapter` in `cynic-proxmox` — drives `pct create/start/destroy`
      through `ArgvExecutionPort` (`cynic-host`), never raw shell
- [ ] Provisioning from a CLEAN template (never clone a dirty container):
      Ubuntu LXC template + pinned rustc + forgejo clone + workspace layout
- [ ] Lifecycle: provision → build (targeted `cargo check -p <crate>`, never
      blanket `make check`) → read-back proof → destroy (or snapshot as template)
- [ ] Interconnects: `cynic-secrets` (clone credentials), `cynic-identity`
      (container identity), `cynic-janitor` (GC of dead containers/workspaces),
      `cynic-node` (scheduler integration)
- [ ] Prove on a real ephemeral container: targeted check green in < 2 min
- [ ] Prove the full reference slice end-to-end with the `cynic` binary it builds

### Phase 2 — Extract `cynic-proxmox` (L1)

- [ ] Create empty crate `cynic-proxmox`
- [ ] Move Proxmox adapters from `cynic-host` and `cynic-node`:
  - `ProxmoxControlAdapter` (879 LOC)
  - `ProxmoxSshCloneAdapter` (233 LOC)
  - `ProxmoxDiskExtensionAdapter` (129 LOC)
  - `ProxmoxSubstrateAdapter` (166 LOC)
  - `ProxmoxRestAgentHostObserver` (809 LOC)
  - `ProxmoxSshAgentHostObserver` (335 LOC)
  - `ProxmoxStorageAdapter` (204 LOC)
- [ ] Unify credentials via `SecretUseGatePort`
- [ ] Watchdog hyperviseur: `observe → detect corruption → recover → alert`
- [ ] Tests mock HTTP for each operation
- [ ] `cynic-host` keeps only profiles + agent_host observation

### Phase 3 — Extract `cynic-janitor` (L1)

- [ ] Create empty crate `cynic-janitor`
- [ ] Move GC ports from `cynic-host`:
  - `DiskObservationPort`, `DiskCleanupExecutionPort`
  - `BuildArtefactObservationPort`, `BuildArtefactGcExecutionPort`
- [ ] Move GC adapters:
  - `LocalDiskAdapter`, `LocalBuildArtefactAdapter`, `RemoteBuildArtefactAdapter`
- [ ] Orchestrator: `observe → classify → plan → execute → read-back`
- [ ] `cynic-node` consumes `cynic-janitor` for auto-purge

### Phase 4 — Fix scheduler `cynic-node`

- [ ] Diagnose `completion_not_admitted` (budget, lineage, evidence)
- [ ] Fix silent spawn (log + detailed error)
- [ ] Honor `package` field in HTTP submit (granularity)
- [ ] Add route `GET /workloads/:id/log` (observability)
- [ ] Auto-purge workspaces via `cynic-janitor`

### Phase 5 — Convergence

- [ ] Push `cynic-audit-engine` from builder
- [ ] Delete `cynic-host/src/audit_engine/` local
- [ ] GC prunable worktrees + residual `build-*` workspaces
- [ ] Delete orphan branches (local + remote)
- [ ] Prove `make check` on ephemeral container

### Phase 6 — Industrialisation

- [ ] Systemd timer watchdog hyperviseur (detection + recovery auto)
- [ ] Systemd timer GC builder (purge workspaces + worktrees)
- [ ] Alert if corruption detected (log + event)
- [ ] Pattern `observe → classify → clean → read-back` replicated everywhere

### Phase 7 — `cynic-secrets` vault capability-based

- [ ] Centralized vault for secrets (not in env)
- [ ] Short-term capabilities for privileged actions
- [ ] Agent can execute `systemctl restart pve-cluster` without password
- [ ] Everything audited (who, what, when, proof)
- [ ] Rotation automatique des credentials
- [ ] `SecretUseGatePort` consumed by scheduler

### Phase 8 — `cynic-identity` binding

- [ ] Binding between identity and capabilities
- [ ] Key rotation for `SigningMaterial`
- [ ] Vault for keys (not in env or files)
- [ ] `no_std` compatible vault

---

## 5. Expected results

| metric | before | after |
|---|---|---|
| `cynic-host` LOC | 87 276 | ~74 000 |
| `cynic-node` LOC | 54 774 | ~53 700 |
| `cynic-proxmox` LOC | — | ~14 700 |
| `cynic-janitor` LOC | — | ~2 000 |
| traits | 3 overlapping | 1 unified |
| adapters | 9 files, 2 crates | 1 file, 1 crate |
| credential paths | 4 fragmented | 1 unified |
| CLI commands | `cynic hypervisor *` (proxmox-specific) | `cynic hypervisor *` (agnostic) |
| recovery | manual, undocumented | `cynic hypervisor recover` |
| watchdog | none | `cynic-proxmox` timer |
| GC | none | `cynic-janitor` timer |
| secrets | fragmented env/file/keyring | vault capability-based |
| identity | no rotation | bound to capabilities |

---

## 6. Explicitly out of scope

- **VMware/Cloud/Libvirt adapters** — the taxonomy supports them, but only Proxmox is implemented
- **Live migration** — not needed yet
- **Multi-hypervisor federation** — future work
- **Web UI** — CLI is the only surface
