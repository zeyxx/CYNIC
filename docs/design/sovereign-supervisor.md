# Sovereign Supervisor — Contract-First Design

**Status:** DESIGN  
**Schema:** `cynic.supervisor_contract.v1`  
**Authority:** `contracts/supervisor-contract.json` is normative. This markdown document has zero execution authority; it is the human-facing rationale for that contract.

---

## 0. The decision

CYNIC gets an explicit **sovereign supervisor** for CYNIC-owned long-running processes.

The supervisor is:

- one named responsibility, not an accident of the operator CLI;
- contract-driven, with roles, argv, restart policy, signals, health, privilege, and observation declared in `contracts/supervisor-contract.json`;
- pure for decisions, imperative only for spawn/signal/socket effects;
- hardware/OS-agnostic in architecture, implemented per host only when a real use exists.

This is the first contract-first step. It does not authorize supervisor code, systemd changes, or runtime mutation.

---

## 1. Measured starting state

The workspace already contains fragmented supervision concepts:

- `crates/cynic-node/src/supervise.rs` and `crates/cynic-node/src/runtime_supervision.rs` supervise an inference backend with backoff, process groups, and shutdown handling.
- `crates/cynic-inference/src/adapters/local/process_supervisor.rs` and `docker_supervisor.rs` implement separate supervisor concepts.
- `crates/cynic-host/src/service_supervisor_observation.rs` observes service-manager providers: systemd, openrc, and process/port.
- `crates/cynic-host/src/process_convergence/types.rs` models process convergence decisions and records.
- Existing systemd units use separate binaries:
  - `/usr/local/bin/cynic-node`
  - `/usr/local/bin/cynic-kernel`

The debt is not that supervision exists. The debt is that there are several homes and no single declared authority.

---

## 2. Target architecture

```text
operator CLI
  → cynic supervise observe/start/stop
      → pure supervisor decision core
          → OS-gated imperative shell
              → CYNIC-owned child processes
```

### 2.1 One composition root

`cynic` remains the canonical binary. A role is selected by argv:

```bash
cynic daemon node
cynic daemon kernel
cynic mcp serve
```

The supervisor may parent these roles, but it does not require a second binary artifact.

### 2.2 Pure decision core

The decision core answers questions without I/O:

- may this role start?
- is this restart admissible?
- is this child failed or unknown?
- what is the next bounded action?

That core belongs in `cynic-protocol` when implementation begins. It must be testable without a host, socket, process table, or service manager.

### 2.3 Imperative shell

Spawn, signal, process-group kill, health probe, log routing, and service-manager read-back belong behind OS-gated adapters.

The shell must:

- use typed argv, never shell;
- use foreground processes, never SysV double-fork;
- enforce process groups where supported;
- require read-back after every bounded action;
- record capability id, argv, exit code, and evidence id.

### 2.4 Portability

The architecture is OS/hardware-agnostic, but implementation is justified per host.

Allowed boundaries:

- `cfg(unix)`
- `cfg(target_os = "linux")`
- `cfg(windows)`
- platform-specific adapters selected by capability or host family

Not allowed in the pure core:

- `std::process::Command`
- `tokio::spawn`
- sockets
- OS syscalls
- service-manager commands

Windows and macOS are not built now. The architecture must not make them impossible later.

---

## 3. External SOTA evidence

The design follows patterns observed in credible external projects.

| Pattern | Evidence | Consequence for CYNIC |
|---|---|---|
| Supervisor as distinct product | `runpulse`, `shep-daemon`, `immortal` | supervision gets its own responsibility and contract, not an implicit CLI side effect |
| Pure decision core + OS tier | `shep-daemon` | decisions are testable; spawn/signal/socket effects stay in adapters |
| Foreground new-style daemons | `systemd` daemon guidance | no double-fork; host supervisor owns PID 1 |
| Bounded drain and kill ladder | `runpulse`, `immortal` | SIGTERM → grace → SIGKILL, with process-group cleanup |
| Typed argv, no shell | supervisor ecosystem | spawn boundaries remain attestable |
| Agent OS process governance | AIOS, AIagentOS, AgenticOS | supervisor tracks role, identity, budget, health, and capability, not just PID |
| Cross-target portability | Rust `cfg`, musl static builds, Cosmopolitan experiments | portability is an architecture boundary, not a simultaneous build matrix |

### 3.1 What CYNIC should not copy blindly

- A parent supervisor must not become a second init system.
- It must not hide privilege widening behind a single binary.
- It must not turn an operator command into an implicit daemon launcher.
- It must not build Windows/macOS adapters before a real use exists.

---

## 4. Migration phases

1. **Contract-first — done in this step**
   - `contracts/supervisor-contract.json` is normative.
   - No runtime code is authorized by this document.

2. **Pure supervisor types**
   - Add pure types and decision functions to `cynic-protocol`.
   - Zero I/O, deterministic tests, no host dependency.

3. **Observation without mutation**
   - Expose `cynic supervise observe`.
   - Read process, service-manager, and health evidence.
   - Unknown stays unknown.

4. **Bounded lifecycle for one role**
   - Start with `node`.
   - Implement `start`, `stop`, `restart` behind explicit `--apply`-style boundaries if required by existing policy.
   - Require post-action read-back.

5. **Proof and rollback**
   - Record evidence for every bounded action.
   - Prove orphan adoption, crash backoff, drain, and residual unknowns.

6. **Remaining roles**
   - Migrate `kernel`, `mcp`, inference, and storage supervision only after `node` is proven.
   - Absorb duplicated supervisor concepts into the single authority.

---

## 5. Open decisions

1. **Parent scope**
   - Should the CYNIC supervisor spawn existing daemon roles first, or absorb their internal child supervision immediately?
   - Recommendation: spawn existing roles first, then absorb internal supervision after proof.

2. **Host supervisor relationship**
   - Should systemd remain the outer supervisor, or should CYNIC replace it?
   - Recommendation: systemd remains the outer supervisor; CYNIC owns CYNIC-owned children and observations.

These decisions are recorded in the contract as `open_decisions`. They are not silently resolved here.

---

## 6. Explicitly out of scope

- Rewriting crate boundaries.
- Changing systemd units in this step.
- Building Windows or macOS adapters now.
- Replacing the host init system.
- Turning the operator CLI into an implicit daemon launcher.
- Any runtime mutation without explicit approval and read-back proof.

---

## 7. Why contract-first is the organic step

CYNIC’s current debt is not a lack of supervisor code. It is a lack of one declared authority across several supervisor implementations.

Declaring the contract first:

- prevents implementation from inventing policy;
- makes the first role migration testable against declared policy;
- keeps pure decisions and OS effects separate;
- records open decisions instead of hiding them;
- avoids speculative Windows/macOS work while keeping the architecture portable.

The next implementation step is pure types in `cynic-protocol`, not a new runtime spawner.
