# Proxmox / Fleet Cartography

This document maps the current fragmentation around Proxmox, fleet discovery, and VM activation in CYNIC.

## Scope

The subject is everything that currently:
- reads or writes Proxmox state
- discovers VM or container inventory
- maps VM state into kernel observations
- drives lifecycle readiness for `cynic-vm-anvil` and `cynic-vm-core`

The goal here is not to refactor yet. The goal is to make the fragmentation explicit and assign the correct responsibility boundary.

## Current Layers

### 1. Registry layer

Source of truth for declared topology:
- [infra/registry.json](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/registry.json:191)

What it declares today:
- `cynic-host-proxmox` as `hypervisor`
- `cynic-vm-anvil` as `build_runner`
- `cynic-vm-core` as `kernel_primary`
- Tailscale placeholders for `TAILSCALE_HOST_PROXMOX`, `TAILSCALE_ANVIL`, `TAILSCALE_CORE`
- lifecycle workflow and activation evidence rules

This is the declarative layer. It does not probe reality.

### 2. MCP access layer

Remote Proxmox access is already exposed through MCP:
- `cynic_proxmox_list`
- `cynic_proxmox_action`

Reference:
- [crates/cynic-mcp/src/proxy.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-mcp/src/proxy.rs:647)

This layer is an adapter. It talks to Proxmox. It should not own lifecycle policy.

### 3. Script/bootstrap layer

Current shell surfaces:
- [infra/monitoring/monitor_vms.sh](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/monitoring/monitor_vms.sh:1)
- [infra/monitoring/provision_cynic.sh](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/monitoring/provision_cynic.sh:1)
- [scripts/fleet-sync.sh](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/fleet-sync.sh:1)
- [infra/systemd/cynic-monitor.service](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/systemd/cynic-monitor.service:1)
- [infra/systemd/cynic-monitor.timer](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/systemd/cynic-monitor.timer:1)

These are bootstrap or operational bridges. They prove that the concern is not yet centralized.

### 4. Kernel sensing layer

The kernel already has the right primitive for fleet sensing:
- probe trait and typed `ProbeDetails`
- fleet probe in Rust
- probe scheduler in the runtime loop
- main boot sequence wiring probes into the environment snapshot

Relevant files:
- [crates/cynic-kernel/src/domain/probe.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-kernel/src/domain/probe.rs:1)
- [crates/cynic-kernel/src/infra/probes/fleet.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-kernel/src/infra/probes/fleet.rs:1)
- [crates/cynic-kernel/src/infra/probes/mod.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-kernel/src/infra/probes/mod.rs:1)
- [crates/cynic-kernel/src/infra/tasks/runtime_loops.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-kernel/src/infra/tasks/runtime_loops.rs:1)
- [crates/cynic-kernel/src/main.rs](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/crates/cynic-kernel/src/main.rs:620)

This is the correct place for sensed fleet facts to end up.

### 5. Lifecycle orchestration layer

`organ-forge` owns lifecycle policy and readiness, not Proxmox access:
- [organs/forge/README.md](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/organs/forge/README.md:1)
- [organs/forge/src/core/lifecycle_readiness.py](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/organs/forge/src/core/lifecycle_readiness.py:1)

It consumes activation evidence and fleet status. It should not become the Proxmox client.

### 6. Anvil surface

`organs/anvil` still contains topology assumptions that are no longer aligned with the registry:
- [organs/anvil/src/entrypoints/kernel_worker.py](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/organs/anvil/src/entrypoints/kernel_worker.py:32)
- [organs/anvil/src/core/entities.py](/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/organs/anvil/src/core/entities.py:1)

The naming drift is visible:
- code still looks for `proxmox_host_kernel`
- the registry now models `hypervisor`

This is a clear SoC break between historical naming and current declared topology.

## Fragmentation Inventory

### A. Inventory source duplication

Current inventory exists in at least three forms:
- registry JSON
- MCP Proxmox list tool
- shell monitoring script

The registry is declarative.
The MCP tool is operational.
The shell probe is bootstrap glue.

### B. Responsibility overlap

Proxmox-related behavior is spread across:
- MCP proxy
- monitoring shell
- systemd timer/service
- kernel fleet probe
- lifecycle readiness
- anvil worker code
- registry docs

This means no single layer currently owns:
- discovery
- observation
- action
- lifecycle interpretation

### C. Naming drift

Observed mismatch:
- `proxmox_host_kernel` remains in anvil code
- `hypervisor` is the registry role
- `cynic-host-proxmox` is the concrete node name

That drift is not cosmetic. It changes what code thinks the topology is.

### D. Boundary confusion

The kernel should not talk directly to Proxmox if Proxmox is an infrastructure concern.
The lifecycle organ should not own inventory collection.
Scripts should not be the permanent architecture.

## Recommended Soc Boundary

### Proxmox / fleet organ

This concern should live in a dedicated infra organ, conceptually:
- `organ-proxmox`
- or `organ-fleet`

Its job would be:
- discover Proxmox inventory
- resolve VM identities
- emit typed observations
- optionally perform explicit Proxmox actions
- never encode lifecycle policy

### Kernel

The kernel should:
- receive typed fleet observations
- expose them through `/observe` or equivalent storage
- keep probes as domain structs, not shell text

### Organ-forge

`organ-forge` should:
- read registry policy
- inspect readiness evidence
- decide whether release paths are blocked
- not become the Proxmox client

## What Is Canonical Today

Canonical pieces already exist, but they are split:
- registry declares the desired topology
- MCP exposes direct Proxmox access
- kernel has typed probe infrastructure
- organ-forge has lifecycle policy

What is missing is a single owner for the Proxmox/fleet concern.

## What This Means Operationally

For now:
- `infra/monitoring/monitor_vms.sh` is acceptable as bootstrap
- `cynic_proxmox_list` is acceptable as an operator access tool
- `crates/cynic-kernel` should be the consumer of typed fleet facts
- `organ-forge` should remain the policy gate

## K8S Migration Path (Kernel-First)

The architecture is already provider-agnostic at the probe level. To pilot a controlled migration:

- Keep `infra/registry.json` as the desired topology source.
- Run both providers in parallel (`CYNIC_FLEET_PROVIDERS=proxmox,k8s`) for read-only parity.
- Compare declared vs observed by role/node in `FleetInventory` probe payload.
- Only move `KernelRelease` lifecycle policy when readiness evidence converges.

Environment defaults:
- `CYNIC_FLEET_PROVIDERS` (default: `proxmox`, can include `k8s`)
- `CYNIC_FLEET_K8S_ENABLED=true`
- `CYNIC_KUBECTL_BIN` (optional, default: `kubectl`)
- `CYNIC_K8S_CONTEXT` (optional)
- `CYNIC_K8S_NAMESPACE` (optional)
- `CYNIC_K8S_DISCOVER_NODES` (optional, default: `true`)
- Fleet payload now includes a summary with:
  - declared/observed/matched/missing/unexpected counts
  - per-provider evidence
  - declaration gap lists (`declaration_missing_nodes` / `declaration_unexpected_nodes`)

The next architectural step is to lift Proxmox/fleet into one explicit organ or infra module, then retire the shell bridge once parity exists in Rust.
