# cynic-proxmox — Design and Consolidation Plan

**Status:** SPECIFIED, NOT STARTED
**Schema:** `cynic.design.proxmox_consolidation.v1`
**Authority:** this document is a *directive*, not a contract. It changes no gate,
no ledger and no threshold.

---

## 0. Why this crate exists

The Proxmox/hypervisor control surface is fragmented across three crates with
overlapping traits, inverted dependencies, and no single path for credentials.
This was measured on 2026-10-06 and confirmed by a live incident where the
builder VM became unreachable due to `pmxcfs` IPC corruption — a failure mode
that no existing code could diagnose or recover from.

---

## 1. Incident data — 2026-10-06

### 1.1 Symptom

`ssh cynic-builder` → `Connection timed out during banner exchange`

### 1.2 Root cause chain

| layer | what happened | how it manifested |
|---|---|---|
| `pmxcfs` (PID 4671) | old process held IPC files in `/dev/shm/qb-4671-*` | new `pmxcfs` could not create its own IPC |
| `pvedaemon` | could not connect to `pmxcfs` | `qm list` / `pct list` returned `ipcc_send_rec` errors |
| `sshd` on builder | could not authenticate (PAM depends on `pmxcfs` for user lookup) | TCP SYN accepted, banner never sent |
| `sudo` on PVE | `pmxcfs` lockfile stale | `systemctl restart pve-cluster` failed with timeout |

### 1.3 Recovery procedure (proven, reproducible)

```bash
# 1. Kill the stale pmxcfs process
kill -9 <old_pmxcfs_pid>

# 2. Remove corrupted IPC files
rm -rf /dev/shm/qb-*

# 3. Remove stale lockfile
rm /var/lib/pve-cluster/.pmxcfs.lockfile

# 4. Restart pve-cluster (remounts pmxcfs)
systemctl restart pve-cluster
sleep 3

# 5. Restart pvedaemon (reconnects to pmxcfs)
systemctl restart pvedaemon
sleep 3

# 6. Verify
/usr/sbin/qm list
/usr/sbin/pct list
```

### 1.4 What was NOT needed

- Full node reboot (tried first, did not help)
- Reinstalling Proxmox packages
- Restoring from backup

### 1.5 What CYNIC could NOT do

- Diagnose the failure (no hypervisor control surface)
- Recover the builder (no API access without `pvedaemon`)
- Execute privileged commands (sudo requires password, no capability-based auth)

---

## 2. Current fragmentation

### 2.1 Traits (3 overlapping)

| trait | crate | layer | consumers |
|---|---|---|---|
| `HypervisorObserver` + `HypervisorController` | `cynic-protocol` | L0 | **zero** (orphan) |
| `HypervisorPort` | `cynic-node` | L3 | `homeostasis_uc` |
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

| type | location 1 | location 2 | problem |
|---|---|---|---|
| `VmState` | `cynic-protocol/src/host_substrate/hypervisor.rs:31` | `cynic-protocol/src/host_substrate/proxmox.rs:7` | **same crate, two definitions** |
| `LxcBridgeRequest/Response/Plan/Status` | `cynic-lxc/src/types.rs` | — | should be in `cynic-protocol` |

### 2.4 Credential paths (fragmented)

| path | mechanism | problem |
|---|---|---|
| `CYNIC_PROXMOX_TOKEN_SECRET` | env var | exposed in process list |
| `/etc/cynic/secrets/proxmox-token` | file | correct, but no rotation |
| `sudo` on PVE | password | no capability-based auth |
| `pvesh` NOPASSWD | sudoers | only one command, no `systemctl` |

---

## 3. Target architecture

### 3.1 Taxonomy (agnostic, in `cynic-protocol`)

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

### 3.2 Single trait (in `cynic-protocol`)

```rust
pub trait HypervisorController: Send + Sync {
    fn observe(&self) -> Result<HypervisorObservation, String>;
    fn observe_vm(&self, id: &str) -> Result<VmObservation, String>;
    fn control_vm(&self, op: &VmControlOp) -> Result<VmControlResult, String>;
    fn validate_control(&self, op: &VmControlOp) -> Vec<String>;
}
```

### 3.3 Single adapter (in `cynic-proxmox`, L1)

```rust
pub struct ProxmoxAdapter {
    client: reqwest::Client,
    base_url: String,
    token: String,  // from HostSecretPort, never in code
}
```

### 3.4 Credential flow

```
HostSecretPort → /etc/cynic/secrets/proxmox-token → ProxmoxAdapter
```

No env vars, no hardcoded paths, no sudo password.

### 3.5 CLI surface (agnostic)

```bash
cynic hypervisor list [--json]
cynic hypervisor status [--json]
cynic hypervisor start <id> [--yes]
cynic hypervisor stop <id> [--yes]
cynic hypervisor reboot <id> [--yes]
cynic hypervisor extend-disk <id> <size-gb> [--yes]
cynic hypervisor snapshot <id> <name> [--description <text>]
cynic hypervisor rollback <id> <name>
```

No "proxmox" in the CLI vocabulary. The adapter is injected, not named.

---

## 4. Consolidation plan

### Stage 1 — Assainir `cynic-protocol`

- Supprimer le doublon `VmState` dans `proxmox.rs`
- Introduire `ContainerKind`
- Étendre `HypervisorKind` avec `Libvirt`
- Déplacer les types LXC de `cynic-lxc` vers `cynic-protocol`
- Fusionner `HypervisorPort` → `HypervisorController` + `HypervisorObserver`

### Stage 2 — Créer `cynic-proxmox` (L1)

- Déplacer tous les adaptateurs Proxmox depuis `cynic-host` et `cynic-node`
- Un seul adaptateur API Proxmox
- Un seul chemin de credentials via `HostSecretPort`
- Tests contre un mock HTTP

### Stage 3 — CLI et MCP agnostiques

- `cynic-cli` appelle `HypervisorController` depuis `cynic-protocol`
- `cynic-mcp` appelle le même adaptateur
- Supprimer la logique Proxmox dupliquée dans `cynic-mcp`

### Stage 4 — Recovery automation

- `cynic hypervisor recover` — détecte et corrige les corruptions IPC/lockfile
- `cynic hypervisor health` — vérifie l'état de `pmxcfs`, `pvedaemon`, IPC
- Intégrer la procédure de récupération de §1.3 comme code

### Stage 5 — Vault capability-based (Stage 6 du goal)

- Vault centralisé pour les secrets (pas en clair dans l'env)
- Capabilities à court terme pour les actions privilégiées
- L'agent peut exécuter `systemctl restart pve-cluster` sans mot de passe
- Tout est audité (qui, quoi, quand, preuve)

---

## 5. Expected results

| metric | before | after |
|---|---|---|
| `cynic-host` LOC | 87 276 | ~74 000 |
| `cynic-node` LOC | 54 774 | ~53 700 |
| `cynic-proxmox` LOC | — | ~14 700 |
| traits | 3 overlapping | 1 unified |
| adapters | 9 files, 2 crates | 1 file, 1 crate |
| credential paths | 4 fragmented | 1 unified |
| CLI commands | `cynic hypervisor *` (proxmox-specific) | `cynic hypervisor *` (agnostic) |
| recovery | manual, undocumented | `cynic hypervisor recover` |

---

## 6. Explicitly out of scope

- **VMware/Cloud/Libvirt adapters** — the taxonomy supports them, but only Proxmox is implemented
- **Live migration** — not needed yet
- **Multi-hypervisor federation** — future work
- **Web UI** — CLI is the only surface
