# Single Binary, Capability-Selected — Consolidation Design

**Status:** PROPOSED
**Schema:** `cynic.single_binary.v1`
**Authority:** directive. Changes no gate, no ledger, no threshold.
**Depends on:** `docs/design/proxmox-consolidation-plan.md`

---

## 0. The decision

CYNIC collapses to **one binary** — `cynic` — with a **capability catalog
selected at install time**. Some capabilities are mandatory; the rest are
opt-in. Privilege is granted per capability, not per binary.

This replaces eleven binaries and five sudoers membranes with one entry point
and one generated sudoers file.

---

## 1. Measured problem (2026-10-06)

### 1.1 Binaries built

| # | binary | crate | main LOC | audience declared | sudoers |
|---|---|---|---|---|---|
| 1 | `cynic` | cynic-cli | 11 | operator | — |
| 2 | `cynic-node` | cynic-node | 121 | service | `/etc/sudoers.d/cynic-node` |
| 3 | `cynic-kernel` | cynic-kernel | 33 | service | — |
| 4 | `cynic-mcp` | cynic-mcp | 10 | service | — |
| 5 | `cynic-lxc-exec-bridge` | cynic-lxc | 89 | privileged | `/etc/sudoers.d/cynic-lxc-exec-bridge` |
| 6 | `cynic-host-exec` | cynic-host | 12 | privileged | `/etc/sudoers.d/cynic-host-exec` |
| 7 | `cynic-network-actuator` | cynic-node | 37 | tooling | `/etc/sudoers.d/cynic-network-actuator` |
| 8 | `cynic-node-enroll` | cynic-node | 379 | tooling | — |
| 9 | `cynic-topology` | cynic-audit-engine | 176 | tooling | — |
| 10 | `cynic-pool` | cynic-pool | **missing** | tooling | — |
| 11 | `zeroclaw-solana-demo` | cynic-zeroclaw-solana | 148 | tooling | — |

**Two measured discrepancies:**

1. `infra/runtime-basket.json` declares `privileged: 2` but lists only 1
   privileged artifact. `cynic-network-actuator` is declared `tooling` (with a
   note calling it `DEAD: zero external consumers`) yet it carries a sudoers
   membrane (`cynic-host::runtime_layout:299`). The basket drifted from reality.
2. `cynic-pool/Cargo.toml` declares `[[bin]] path = "src/main.rs"` and that file
   **does not exist**. The binary is phantom: the manifest promises a target the
   tree cannot build. (`required-features = ["cli"]` may mask it.)

### 1.2 Privilege membranes (the real count)

Five distinct sudoers targets, three different installation mechanisms:

| sudoers target | granted binary | how installed |
|---|---|---|
| `/etc/sudoers.d/cynic-lxc-exec-bridge` | `cynic-lxc-exec-bridge` | Rust `cynic-lxc::bundle` |
| `/etc/sudoers.d/cynic-pve-pct` | `pct-bounded-exec` (**shell**) | Rust `cynic-host::pve_helper_install` |
| `/etc/sudoers.d/cynic-host-exec` | `cynic-host-exec` | shell in `framework_bootstrap/scripts.rs` |
| `/etc/sudoers.d/cynic-network-actuator` | `cynic-network-actuator` | Rust `cynic-host::runtime_layout` |
| `/etc/sudoers.d/cynic-node` | `cynic-node` | `runtime-basket.json` |

### 1.3 Duplicated policy (the actual debt)

Three independent implementations build the same `pct` argv:

| implementation | language | LOC |
|---|---|---|
| `cynic-lxc::admit` + `execute` | Rust | 1 086 |
| `pct-bounded-exec.sh` | **shell** | 75 |
| `cynic-proxmox/proxmox_clone.rs` | Rust | 233 |

Three implementations of one policy diverge silently. The shell cannot share
`admit()` with Rust: it is mechanically a second source of truth.

**This violates the standing doctrine** (supply chain 100% Rust, shell wrapper
eradication) in `infra/host-bootstrap/pve/pct-bounded-exec.sh`.

---

## 2. Why the current split does not buy safety

The declared justification (`runtime-basket.json` → `audiences.privileged`):

> *"Kept separate because one binary that can run as root must not expose every
> other subcommand at root."*

**This is false as stated.** The privilege boundary is the **sudoers argv
allowlist**, not the binary separation. Two binaries with the same sudoers
grant are equally dangerous; one binary with a strict allowlist is safer than
five with sloppy ones. Separation without admission strictness is theatre.

What actually protects the system:
1. The sudoers file names **exact argv** (`/path/bin <capability-id>`, fixed args).
2. The binary refuses any capability not in its **installed catalog**.
3. Every invocation is **journaled** with its capability id.
4. The catalog is **generated from declared policy data**, never hand-written.

All four survive the collapse. None of them require a separate binary.

---

## 3. Target architecture

### 3.1 One binary

```
/usr/local/bin/cynic        # THE binary. One. Everywhere.
```

Installed once, in one place, by one mechanism. `systemd` units invoke it
with a subcommand (`cynic node control`, `cynic kernel run`,
`cynic mcp serve`).

### 3.2 Capability catalog (data, not code)

```jsonc
// infra/capabilities/catalog.json
{
  "schema": "cynic.capability_catalog.v1",
  "capabilities": [
    {
      "id": "pct-list",
      "domain": "hypervisor",
      "argv": ["pct", "list"],
      "requires_root": true,
      "mandatory": false,
      "purpose": "CT inventory for fleet observation (read-only)"
    },
    {
      "id": "pct-clone",
      "domain": "hypervisor",
      "argv": ["pct", "clone", "{source}", "{target}", "--full"],
      "requires_root": true,
      "mandatory": false,
      "purpose": "ephemeral build container provisioning"
    },
    {
      "id": "systemctl-restart-pve-cluster",
      "domain": "system",
      "argv": ["systemctl", "restart", "pve-cluster"],
      "requires_root": true,
      "mandatory": false,
      "purpose": "hypervisor watchdog recovery"
    },
    {
      "id": "node-control",
      "domain": "runtime",
      "argv": ["node", "control"],
      "requires_root": false,
      "mandatory": true,
      "purpose": "supervised node runtime"
    }
  ]
}
```

### 3.3 Install-time selection

```bash
# Mandatory capabilities are always installed.
# Optional capabilities are chosen per node role.
cynic install --profile builder \
  --capabilities pct-list,pct-clone,systemctl-restart-pve-cluster

# Renders exactly one sudoers file:
#   /etc/sudoers.d/cynic
#   cynic ALL=(root) NOPASSWD: /usr/local/bin/cynic privilege exec pct-list
#   cynic ALL=(root) NOPASSWD: /usr/local/bin/cynic privilege exec pct-clone *
#   cynic ALL=(root) NOPASSWD: /usr/local/bin/cynic privilege exec systemctl-restart-pve-cluster
```

### 3.4 Invocation

```bash
# Unprivileged operator surface
cynic hypervisor list
cynic node control

# Privileged capability (only if installed)
sudo -n /usr/local/bin/cynic privilege exec pct-clone --source 105 --target 106
```

The `privilege exec` arm:
1. Looks up the capability id in the **compiled-in catalog** (not argv).
2. Refuses if the id is not in the **installed catalog** (written at install).
3. Builds argv from the **catalog template** + validated parameters.
4. Journals the capability id, the resolved argv, and the exit code.
5. Executes. Never a shell.

### 3.5 The admission library

`admit()` is one Rust function, shared by every domain:

```
cynic-protocol/privilege/catalog.rs     # pure: capability types, validation
cynic-host/privilege/admission.rs       # admission: id -> exact argv
cynic/privilege_exec.rs                 # the binary arm, journaling, exec
```

`cynic-lxc`, `cynic-proxmox` (→ `cynic-hypervisor`), and every future domain
**call `admit()`** instead of building argv themselves. The shell executor is
deleted.

### 3.6 What happens to each binary

| today | becomes |
|---|---|
| `cynic` | **the binary** (gains all arms) |
| `cynic-node` | `cynic node control` — systemd unit updated |
| `cynic-kernel` | `cynic kernel run` — systemd unit updated (boot lifecycle: see §5) |
| `cynic-mcp` | `cynic mcp serve` |
| `cynic-lxc-exec-bridge` | `cynic privilege exec <lxc-capability>` |
| `cynic-host-exec` | `cynic host <subcommand>` (staging) + `cynic privilege exec` (mutating) |
| `cynic-network-actuator` | `cynic network actuate` (or **deleted**: the basket calls it DEAD — decide, don't keep a dead binary with a sudoers grant) |
| `cynic-node-enroll` | `cynic node enroll` |
| `cynic-topology` | `cynic topology --write` (keeps read/write separation: §5) |
| `cynic-pool` | `cynic pool` |
| `zeroclaw-solana-demo` | `cynic zeroclaw demo` |

Crates keep their boundaries. They become **libraries**. `cynic-cli` (or a new
thin `cynic-bin` crate) links them and dispatches. The 27 existing CLI families
are already this shape.

---

## 4. Migration phases

### Phase A — catalog + admission — DONE 2026-10-06
- [x] `cynic-protocol/privilege/catalog.rs`: pure capability types + validation
      (25 tests green on the builder)
- [x] Port `pve-pct-helper.policy.json` + `transport-capabilities.policy.json`
      into one `infra/capabilities/catalog.json` (10 capabilities)
- [x] `cynic-host/privilege/capability_catalog.rs`: loader adapter (3 tests)
- [x] Prove: a capability not in the catalog is refused with a named blocker

### Phase B — `cynic privilege exec` arm — DONE 2026-10-06
- [x] Add the arm to `cynic-cli` (`privilege_surface.rs`)
- [x] Journaling (capability id, argv, requires_root) before the process starts
- [x] Prove: `catalog|admit|exec` run; an uninstalled capability refuses (exit 1);
      an injection attempt refuses; an absent catalog fails closed

### Phase C — absorb the privileged binaries
- [ ] `cynic-lxc-exec-bridge` → capability; delete the binary target
- [ ] `pct-bounded-exec.sh` → Rust capability; delete the shell script
- [ ] `cynic-host-exec` mutating subcommands → capabilities
- [ ] `cynic-network-actuator` → `cynic network actuate`
- [x] One generated `/etc/sudoers.d/cynic` (`cynic privilege install`);
      9/9 rules pass `visudo -c`. The five old files still exist — deleting
      them is the remaining step of this phase.

### Phase D — absorb the tooling + service binaries
- [x] `cynic-pool` phantom `[[bin]]` removed (the file never existed)
- [ ] `cynic-topology`, `cynic-node-enroll`, `zeroclaw-solana-demo`
- [ ] `cynic-node`, `cynic-mcp` → subcommands; update systemd units
- [x] `cynic-kernel` → decision made: stays separate (see §5.1)
- [ ] Update `infra/runtime-basket.json`: 11 artifacts → 1

### Phase E — prove
- [ ] One binary builds
- [ ] Every capability in the catalog has a test that exercises its argv
- [x] A capability absent from the installed catalog fails
- [ ] `make check` green; `cynic audit all` no new findings

---

## 5. Open decisions — MEASURED 2026-10-06

### 5.1 `cynic-kernel` — RESOLVED: stays a separate binary

**Measured.** `infra/systemd/cynic-kernel.service` runs `After=network.target`,
`WantedBy=default.target`, `MemoryMax=2G`, `Restart=on-failure`, and
`cynic-kernel/Cargo.toml:27` depends on `cynic-node`. Folding it into `cynic`
would give `cynic → cynic-kernel → cynic-node → …` and couple an OS boot
service to the operator CLI: a kernel panic would kill the operator surface.

**Decision:** keep `cynic-kernel` as a justified exception, recorded in the
basket with this reason. The kernel is the one binary that must run before and
independently of the operator.

### 5.2 `cynic-topology` — RESOLVED: stays a separate binary

**Measured.** The binary's own module doc (`bin/cynic-topology.rs:10-16`)
states the reason: *"A `--write` flag on the audit surface would make the
reader of the topology document capable of mutating it … a reader that can
also write cannot be trusted to only read."*

Folding it into `cynic topology --write` would put both paths in one binary,
which does **not** preserve the boundary. The separation must be at the binary
level, not the subcommand level.

**Decision:** keep `cynic-topology` as a justified exception, with the
read/write separation as the recorded reason.

### 5.3 Bootstrap circularity — CONFIRMED REAL, needs a decision

**Measured — the loop exists:**
- `install_standalone_cli` (in `cynic-host`) installs the `cynic` binary **by
  running `cynic`** (`cynic deps upgrade` → `install_standalone_cli`).
- The source binary comes from `discover_source_binary()` — it must **already
  exist**.
- The bootstrap scripts install `/usr/local/libexec/cynic-host-exec` **from an
  already-staged bundle**.
- **Nothing describes how the first binary reaches a virgin machine.**

So `cynic install` cannot be first contact. Two options:

| # | approach | trade-off |
|---|---|---|
| **(R)** | `cynic install --source <path>` for first contact: the operator supplies the release binary explicitly; one install path, no separate bootstrap script to maintain | the operator must obtain the signed artifact by some out-of-band means (as with any installer) |
| 2 | A separate signed bootstrap artifact + script | two install mechanisms to keep correct |

**Recommendation: option R.** `cynic install` stays the only install surface;
first contact is an explicit `--source`, never a guess. This is an open
decision for the operator, not silently chosen.

### 5.4 `cynic-pool` phantom binary — CONFIRMED, it is a bug

**Measured.** `cynic-pool/Cargo.toml:23-26` declares
`[[bin]] path = "src/main.rs"` with `required-features = ["cli"]`. That file
**does not exist** (7 source files, no `main.rs`). The `cli = []` feature
exists, so even `--features cli` cannot build it.

**Decision:** either delete the `[[bin]]` block (and the phantom from the
basket), or restore `main.rs`. Not silently resolved: a phantom target makes
every downstream binary count wrong, so it must be fixed where it is declared.

---

## 6. Expected results

| metric | before | after |
|---|---|---|
| binaries built | 11 | 1 (+ kernel if §5.1 keeps it) |
| sudoers files | 5 | 1 |
| shell executors | 1 (`pct-bounded-exec.sh`) | 0 |
| `pct` argv implementations | 3 | 1 (`admit()`) |
| policy sources | 2 JSON + code + shell | 1 catalog JSON |
| capability granularity | per binary | per capability, install-time |
| install location | 2 roots (`/usr/local/bin`, `/usr/local/libexec`, `/usr/local/sbin`) | 1 (`/usr/local/bin`) |
| basket drift | declared ≠ built | 1 artifact, trivially checked |

---

## 7. Explicitly out of scope

- Rewriting crate boundaries (they stay; crates become libraries)
- Changing the 27 CLI families (they are already the right shape)
- Multi-node distribution (one binary per node, same binary everywhere)
