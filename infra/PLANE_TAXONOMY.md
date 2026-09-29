# CYNIC — Plane Taxonomy v1

> **Status**: `normative`
> **Authority**: `contracts/foundation.json` (hard boundary `strict_dag_and_single_responsibility`)
> **Companion**: `infra/network/topology.json`, `infra/registry.json`, `crates/cynic-network/src/firewall_policy.rs`
> **Defined**: 2026-09-28

---

## 1. PURPOSE

This document defines the canonical naming and contract for CYNIC network planes.
It replaces the biological metaphor "synapse" with functional, production-grade names.

**Naming rule**: Plane names are functional, English, non-metaphorical. The name describes
**what the plane does**, not what it evokes.

---

## 2. CANONICAL PLANES

| Plane | CIDR | Federation Scope | Purpose | DNS Zone |
|-------|------|------------------|---------|----------|
| `service` | 10.10.10.0/24 | `canonical_service_plane` | CYNIC service endpoints — kernel, builder, storage, forgejo | `service.cynic` |
| `admin` | 192.168.100.0/24 | `administration_only` | Bare-metal management and hypervisor control | — |
| `mesh` | 10.20.0.0/24 | `encrypted_transport_plane` | WireGuard encrypted full-mesh between nodes | — |

### 2.1 Service Plane

- **CIDR**: 10.10.10.0/24
- **Federation scope**: `canonical_service_plane`
- **DNS zone**: `service.cynic`
- **Realization**: `proxmox_bridge_and_lxc_veth` for CTs; `host_interface_or_route` for cortex
- **State**: `declared`
- **Registry key**: `network_policy.canonical_service_plane`
- **Registry pointer**: `/network_policy/planes/service/cidr`

### 2.2 Admin Plane

- **CIDR**: 192.168.100.0/24
- **Federation scope**: `administration_only`
- **Purpose**: Bare-metal management and hypervisor control
- **Realization**: `host_interface_or_route`
- **State**: `observed`
- **Registry pointer**: `/network_policy/planes/admin/cidr`

### 2.3 Mesh Plane

- **CIDR**: 10.20.0.0/24
- **Federation scope**: `encrypted_transport_plane`
- **Transport**: WireGuard
- **Realization**: `wireguard_l3_overlay`
- **Listen port**: 44196
- **Watchdog**: stale 150s, dead 200s
- **State**: `observed`
- **Registry pointer**: `/network_policy/planes/mesh/cidr`

---

## 3. RENAMING MAP

| Old (metaphor) | New (functional) | Rationale |
|-----------------|-------------------|-----------|
| `synapse` | `service` | It is the service plane — nodes connect to it for CYNIC services |
| `management` | `admin` | It is the administration plane — bare-metal and hypervisor control |
| `overlay` | `mesh` | It is the mesh plane — WireGuard full-mesh between nodes |
| `synapse.cynic` | `service.cynic` | DNS zone follows plane name |
| `SynapseOnly` | `ServiceOnly` | Rust enum variant follows plane name |
| `synapse_cidr` | `service_cidr` | Rust field follows plane name |

---

## 4. GOVERNANCE RULES

### R1. Functional Naming
Plane names MUST be functional, English, non-metaphorical. No biological, mythological, or
brand-name metaphors.

### R2. Single Source of Truth
Plane definitions live in `infra/network/topology.json`. Other files reference planes by
name, never redefine them.

### R3. Registry-Driven Configuration
CIDRs, ports, and interface names MUST come from `infra/registry.json` or
`infra/network/topology.json`. Hardcoded values in Rust source are forbidden.

### R4. DNS Zone Naming
DNS zones follow the pattern `<plane>.cynic`. The service plane zone is `service.cynic`.

### R5. Rust Identifier Mapping
Rust identifiers follow the plane name:
- Enum variants: `ServiceOnly`, `AdminOnly`, `MeshOnly`, `ClusterMesh`
- Struct fields: `service_cidr`, `admin_cidr`, `mesh_interface`
- Match arms: `"service" => NetworkPlaneScope::ServiceOnly`

---

## 5. MIGRATION SLICES

### Slice 1: Contract Definition (this document)
- [x] Define `infra/PLANE_TAXONOMY.md`
- [ ] Update `infra/network/topology.json` with new plane names
- [ ] Update `infra/registry.json` with new plane names

### Slice 2: Rust Types (cynic-network)
- [ ] Rename `NetworkPlaneScope::SynapseOnly` → `ServiceOnly`
- [ ] Rename `DynamicFirewallPlanes::synapse_cidr` → `service_cidr`
- [ ] Rename `DynamicFirewallPlanes::management_cidr` → `admin_cidr`
- [ ] Rename `DynamicFirewallPlanes::wireguard_interface` → `mesh_interface`
- [ ] Update `derive_firewall_policy_from_registry` to read new registry keys
- [ ] Update tests

### Slice 3: Dependent Crates
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-node/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-host/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-cli/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-mcp/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-protocol/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-lifecycle/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-build/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-inference/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-vcs/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-vcs-forgejo/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-consensus/`
- [ ] Update all `rg -l 'synapse'` files in `crates/cynic-node-protocol/`

### Slice 4: Registries and Contracts
- [ ] Update `infra/registries/home-fiber.json`
- [ ] Update `infra/registries/lab.json`
- [ ] Update `infra/registries/single-node.json`
- [ ] Update `infra/agent-source-protocol.json`
- [ ] Update `infra/verticals/node/intent.json`
- [ ] Update `infra/verticals/network/intent.json`
- [ ] Update `infra/verticals/provider_enrollment/intent.json`
- [ ] Update `contracts/relay-policy.json`
- [ ] Rename `infra/synapse-nodes.seed.json` → `infra/mesh-nodes.seed.json`

### Slice 5: Documentation
- [ ] Update `docs/specs/SYNAPSE_P2P_FEDERATION_SPEC.md` → `docs/specs/MESH_P2P_FEDERATION_SPEC.md`
- [ ] Update `docs/specs/SOVEREIGN_RELAY_AND_OFFLINE_OPERATION_SPEC.md`
- [ ] Update `docs/specs/HOST_HARDENING_ENGINEERING_SPEC.md`
- [ ] Update `docs/specs/RUNTIME_CONFORMANCE_AND_METRICS_SPEC.md`
- [ ] Update `docs/design/cynic-agent-isolation-plan.md`
- [ ] Update `docs/design/cynic-isolation-mapping.md`
- [ ] Update `docs/design/cynic-master-isolation-plan.md`
- [ ] Update `protocols/network-topology-design.md`
- [ ] Update `protocols/tls-fleet-design.md`
- [ ] Update `protocols/headscale-mesh-coordination.md`
- [ ] Update `protocols/machine-enrollment-federation.md`

### Slice 6: Verification
- [ ] `cargo check -p cynic-network --lib` passes
- [ ] `cargo check -p cynic-node --lib` passes
- [ ] `cargo check -p cynic-host --lib` passes
- [ ] `cargo check -p cynic-cli --lib` passes
- [ ] `cargo check -p cynic-mcp --lib` passes
- [ ] `cargo check -p cynic-protocol --lib` passes
- [ ] `cargo check -p cynic-lifecycle --lib` passes
- [ ] `cargo check -p cynic-build --lib` passes
- [ ] `cargo check -p cynic-inference --lib` passes
- [ ] `cargo check -p cynic-vcs --lib` passes
- [ ] `cargo check -p cynic-vcs-forgejo --lib` passes
- [ ] `cargo check -p cynic-consensus --lib` passes
- [ ] `cargo check -p cynic-node-protocol --lib` passes
- [ ] `rg -i 'synapse' crates/ --type rust` returns 0 hits
- [ ] `rg -i 'synapse' infra/ --type json` returns 0 hits
- [ ] `make check` passes

---

## 6. SYSTEMIC FINDING — Hardcoding & Portability Debt

The fact that ~115 files and ~680 occurrences reference a plane name is not a naming issue —
it is evidence of **chronic hardcoding and missing agnosticism**.

### 6.1 Root Cause

Plane names are **string literals** scattered across:
- Rust source (match arms, struct fields, enum variants)
- JSON contracts and registries
- Documentation and protocols

There is **no single abstraction** that owns the plane concept. Each crate re-implements
its own plane logic, leading to:
- Duplicated plane definitions
- Inconsistent naming (same plane, different names in different files)
- No runtime portability (plane config is compiled in, not injected)

### 6.2 Portability Violation

CYNIC's own governance rules are violated:
- **R3 (Registry-Driven Configuration)**: CIDRs, ports, and interface names are hardcoded
  in Rust source, not read from registry.
- **R5 (No Unused Dependencies)**: crates depend on concrete plane types instead of
  abstract ports.
- **R8 (No Hardcoded Runtime Values)**: `10.10.10.0/24`, `192.168.100.0/24`, `wg0`, `44196`
  appear in multiple files.

### 6.3 Required Abstraction

A plane should be a **type** in `cynic-protocol` (or `cynic-network`), not a string.
Crates should depend on the type, not on the literal.

```rust
// cynic-protocol/src/plane.rs
pub enum Plane {
    Service,
    Admin,
    Mesh,
}

impl Plane {
    pub fn cidr(&self, registry: &Registry) -> String { ... }
    pub fn dns_zone(&self) -> Option<&str> { ... }
}
```

This would:
- Eliminate string literals from match arms
- Make plane config registry-driven
- Enable runtime portability (same binary, different plane configs)

### 6.4 Migration Debt

The renaming (synapse→service, management→admin, overlay→mesh) is a **symptom fix**.
The **root cause** is the missing abstraction. Until `Plane` exists as a type, any future
plane change will require another mass renaming.

**Recommendation**: After the renaming slices, create a `Plane` type in `cynic-protocol`
and migrate all crates to use it. This is a separate vertical (see `infra/PLANE_TAXONOMY.md` §7).

---

## 7. IMPACT SUMMARY

| Category | Files | Occurrences |
|----------|-------|-------------|
| Rust source | ~80 | ~500 |
| JSON contracts/registries | ~15 | ~80 |
| Docs/protocols | ~20 | ~100 |
| **Total** | **~115** | **~680** |

---

*v1 — 2026-09-28. Canonical plane taxonomy for CYNIC.*
*Referenced from `contracts/foundation.json`, `AGENTS.md`, and `infra/network/topology.json`.*
