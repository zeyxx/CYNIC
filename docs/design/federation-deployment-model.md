# CYNIC Federation Deployment Model — bottom-up design doc

Status: draft. Method: observed facts first (section 0), models derived from
facts (sections 1–3), agent navigation index (section 4), explicit unknowns
(section 5). Nothing in sections 1–3 is asserted beyond what section 0 proves.
All code paths are relative to the workspace root.

## 0. Observed facts (read, not inferred)

### 0.1 Network domain — `crates/cynic-network` (16 modules, ~3,667 LOC)

Modules (`src/lib.rs:6-21`): actuator, discovery, dns, finance_adapters,
headscale_client, firewall_policy, freebox, mesh_plan, port, probe,
registry_maintenance, telemetry, transport_readback, transport_security,
types, watchdog_alerter. No `adapters/` directory exists.

- Pure core: `types.rs` (683), `discovery.rs` (524), `firewall_policy.rs`
  (488), `probe.rs` (407), `mesh_plan.rs` (277), `dns.rs` (113).
- I/O inside the domain root (not quarantined): `headscale_client.rs`
  (206) calls `reqwest::blocking::Client::new()` three times (lines 85,
  98, 113); `Cargo.toml:16` declares `reqwest.workspace = true`.
- Its own guard forbids this: `tests/architecture_boundary.rs` asserts
  `Cargo.toml` contains neither `tokio` nor `reqwest`
  (`network_domain_has_zero_direct_io_network_calls`). By inspection this
  test is RED: the manifest contains `reqwest.workspace = true`.
- A canonical typed plane model exists and is dead: `plane.rs` in
  `cynic-protocol` defines `Plane { Service, Admin, Mesh }`; `grep -rn
  'Plane::'` outside that file returns nothing.
- Consumers of `cynic_network` (measured): `cynic-node` 15 use sites,
  `cynic-host` 9, `cynic-vcs` 2. Direction is downward-correct (L3/L1
  consume L1); no orchestrator is consumed back (boundary test
  `network_domain_has_no_orchestrator_dependencies` covers
  cynic-node/cynic-host/cynic-cli/cynic-kernel).
- In-flight modifications on this branch: `network_notify_surface.rs`
  (CLI) and `network_surfaces_uc.rs` (node) — both are reformatting-only
  diffs, no behavior change.

### 0.2 Registry vocabulary vs mesh-plan resolution (verified by reading)

`infra/registry.json`: `network_policy.planes = {service (10.10.10.0/24,
canonical, dns service.cynic), admin (192.168.100.0/24), mesh
(10.20.0.0/24)}`; `canonical_service_plane = "service"`. UPDATE
2026-10-06 (tranche network-vocabulary, commits test+A1+A2+B, all proven
on builder): node `network` maps now speak the trio (`service`,
`admin`, `local` alternate retained) — zero non-trio keys, enforced by
`crates/cynic-network/tests/mesh_plan_registry_proof.rs`. Before the
migration the maps used `{synapse, management, local}` and the mesh plan
resolved zero peers silently. Lab fixtures (`infra/registries/*.json`)
intentionally keep their local dialects and exercise the documented
legacy-alias arm of `resolve_plane_address`.

`mesh_plan.rs:generate_mesh_plan` resolves the address plane to
`"service"` (policy contains that key). Before the migration,
`peer_from_node` did `node.network.get("service")` directly, so every
peer was silently dropped (`insufficient_peers`, empty list — proven red
on builder, EXIT=101). Since tranche network-vocabulary the lookup goes
through `resolve_plane_address` (trio key first, documented legacy alias
second) and the canonical file resolves via the primary path.

Sibling registries each speak a local dialect (measured):
`lab.json` canonical `labmesh`, planes `[synapse, proxmox_guest]`;
`single-node.json` canonical `loopback`, planes `[synapse,
proxmox_guest]`; `home-fiber.json` canonical `synapse`, planes
`[management, synapse]`, node keys include `public_ingress`.

### 0.3 Federation + enrollment code — where it actually lives

- `crates/cynic-node-protocol/src/`: `emergence.rs`, `local_autonomy.rs`,
  `machine_enrollment.rs` (`MachineEnrollment`, `validate()`,
  `enrollment_matches_subject`), `node_enrollment.rs`,
  `node_activation.rs` (`plan_activation_action`,
  `evaluate_activation_proof`, `decide_node_activation`,
  `classify_activation_blocker`), `deployment_epoch.rs`
  (`DeploymentEpoch`, `DeploymentPhase`, `derive_deployment_epoch`,
  `plan_deployment_transition`), `peer_resolver.rs`,
  `bootstrap_federation.rs`, `node_control.rs`, `node_identity.rs`.
- `crates/cynic-protocol/src/`: `federation_membership.rs`
  (`MemberStatus` liveness vs authority, `MembershipPolicy`,
  `MemberLease`, `FederationDesiredState`), `fleet_discovery.rs`
  (`classify_node`), `fleet_heartbeat.rs`, `fleet_target.rs`.
- `crates/cynic-host/src/`: `federation_runtime.rs` (wires
  `decide_autonomy` + `elect_for_role` from `cynic-node-protocol`),
  `local_autonomy_runtime.rs` (tick observe → decide).
- Reference doctrine: `protocols/machine-enrollment-federation.md`
  (L0 voucher → L1 identity → L2 admission → L3 membership → L4
  transport). Skill-stored file layouts that place autonomy/election
  modules under `cynic-protocol` or `cynic-host` are stale; the table above
  is current.

### 0.4 Binary / compilation model (measured)

Workspace: 49 members in root `Cargo.toml` (`members[2..50]`; the "48
crates" figure in AGENTS.md is stale by one). **10 binaries**, not 8: 4
`src/main.rs` (`cynic`, `cynic-node`, `cynic-kernel`, `cynic-mcp`) plus 6
`src/bin/*.rs` (`cynic-topology`, `cynic-host-exec`, `cynic-audit`,
`cynic-lxc-exec-bridge`, `enroll_grant` (cynic-node tool), `demo`
(cynic-zeroclaw-solana SOP worker). Only 8 carry an explicit `[[bin]]`
stanza; the other 2 exist through cargo's default `autobins` (any
`src/main.rs` / `src/bin/*.rs` is a binary unless opted out) — counting
`[[bin]]` stanzas undercounts. Library-first holds: 47 of 49 crates
expose `src/lib.rs`; binaries are thin argv facades over their own lib
(`cynic-cli/src/main.rs` is 10 lines delegating to `cynic_cli::run_main`;
`cynic-node/src/main.rs`, 115 lines, parses argv then either dispatches
one command or starts the tokio reconcile tasks). Canonical gates: `make check`, `make audit`,
`make release.deploy` (no `make build`; `make test` is non-canonical
while pinned rustc ICE paths persist). Build policy and ICE workarounds
are documented at the top of `Makefile` (lines 16–66): builder-gated
builds, `RUST_ENV`, per-layer check+test, no CLI surface runs `cargo
test`. Deployment-crate surface: `crates/cynic-release/src/` (`contracts,
ports, evaluator, types, adapters/`).

## 1. Federation deployment model (derived)

Three tiers, matching what the code already enforces:

- **Tier 0 — cold-start bootstrap (out-of-band, per node).** Static seed
  on the node filesystem (keys, one seed peer). Zero Git, zero DNS, zero
  discovery. Owner: node image / provisioning. Code anchor:
  `bootstrap_federation.rs`, `pre_os.rs` (node-protocol).
- **Tier 1 — desired state (Git SSOT, read-only at runtime).**
  `infra/registry.json` declares membership, addressing, planes, firewall
  policy, canonical DNS zone. Repository registry is authoritative for
  membership and addressing; it may declare nodes that do not exist yet
  (that is how deployment is planned). Code anchors: `mesh_plan.rs`,
  `dns.rs`, `firewall_policy.rs` (pure synthesis); readers in
  `cynic-node/src/network_mesh_plan.rs`,
  `adapters/local_network_mesh_registry`.
- **Tier 2 — observed state (live, never written back to Git).**
  Discovery reports, telemetry snapshots, membership views, heartbeat
  leases. Transport for Tier 2 between nodes is Tier-0/1-derived
  (WireGuard overlay, kernel HTTPS rendezvous) with pull-only diffusion;
  nodes that lose the kernel degrade to `Autonomous`/`LocalAutonomy` and
  reconcile via CAS on reconnection. Code anchors: `discovery.rs`,
  `telemetry.rs`, `federation_membership.rs`, `fleet_heartbeat.rs`,
  `federation_runtime.rs`.

Deployment mapping per node class (registry names): `cynic-host-proxmox`
(hypervisor + DNS authority candidate), `cynic-host-cortex` (mgmt-only
record; service reachability via overlay alias, not a physical plane),
`local-builder`/`cynic-builder` (sacrificial DMZ on overlay, egress
whitelist), `cynic-lxc-*` (guests announced via subnet-router pattern,
no per-guest WireGuard endpoint), `forgejo` (source transport),
`local-kernel` (rendezvous, optional at runtime — federation survives it).

Hard rule carried over from evidence: a declared peer/endpoint/DNS name
is Tier 1 desire; only a fresh handshake, listener bind, or resolver
answer is Tier 2 fact. The mesh-plan empty-peer finding (§0.2) is what
happens when Tier 1 vocabulary drifts: silent empty output, green tests.

## 2. Node enrollment / deployment model (derived)

Five decoupled layers (matches `protocols/machine-enrollment-federation.md`
and the modules in §0.3 — never couple them):

1. **L0 voucher (out-of-band).** Pull-only: the node requests, the
   federation judges. Single-use, dies after grant read-back. Steady
   state owes nothing to bootstrap.
2. **L1 identity.** ed25519 subject key + signed `MachineEnrollment`
   (`machine_enrollment.rs:25`, `validate()`).
3. **L2 admission.** Kernel REST enrollment → signed grant, monotonic
   generation, CAS store, mandatory read-back.
4. **L3 membership.** AP, volatile. Liveness (`Active/Suspect/Lapsed`)
   is refutable by ack and measured from the last ACKNOWLEDGED instant
   across persisted ticks — never from the newest measurement (a dead
   member with fresh unreachable probes must still lapse). Authority
   (`Suspended/Expelled`) is sticky; timers never set or clear it.
5. **L4 transport.** Replaceable read-back adapters (WireGuard
   handshake proofs, node-control TLS-or-loopback gate). Watchdog fields
   stay L4-only and never feed L3 policy.

Deployment progression for one node: voucher → identity → grant →
`decide_node_activation` verdict (`node_activation.rs:300`; blockers
classified to error codes at :422) → `DeploymentEpoch` phase plan
(`deployment_epoch.rs:106`) → service-plane evidence fresh →
**bootstrap withdrawal** (`Withdraw{evidence} | Retain{reason}`:
retention after convergence is declared debt, plan-not-action). A lapsed
member keeps its rescue path; withdrawal never strands a node without
transport.

## 3. Binary / compilation model (derived)

- **One workspace, 49 crates, 10 binaries (4 daemons/facades + 6 tools).**
  Every binary is a file-layout fact (`src/main.rs`, `src/bin/*.rs`),
  not only the 8 explicit `[[bin]]` stanzas — audit by `find`, never by
  stanza count. Domain crates expose `lib.rs` (47/49); binaries are
  argv-typed facades that render facts and delegate to usecases — no
  business logic in `main.rs`.
- **Strict DAG downward**, guarded per crate by
  `tests/architecture_boundary.rs`. Known RED guard: cynic-network's
  `reqwest` ban (§0.1) — first repair before any extraction (a failing
  guard is the baseline that proves the next change adds nothing).
- **Builds happen on the builder, not on dev hosts** (rustc ICE policy,
  `Makefile:16-66`). `RUSTFLAGS="-C debuginfo=0"` fast checks per crate;
  full gates only via `make check` / `make audit` / `make
  release.deploy`. Never run `cargo build/test` on an unstable host and
  call the result a verdict.
- **Release = versioned directory + atomic symlink flip**, never
  overwrite of a running binary (`/var/lib/cynic/releases/<hash>/` →
  `/var/lib/cynic/current`), with `/health` read-back before and after.
  `cynic-release` owns the contract (`contracts.rs`, `evaluator.rs`).

## 4. Agent navigation index

| I want to … | Read first | Prove with |
|---|---|---|
| Resolve an endpoint / plane | `infra/registry.json` `network_policy` + node `network` map; `mesh_plan.rs:73-84` fallback chain | `cynic network mesh-plan --json` → non-empty `desired_peers` |
| Add a node to the fleet | §2 L0→L4; `machine_enrollment.rs`, `node_activation.rs:130` | `decide_node_activation` verdict + grant read-back |
| Check fleet liveness | `federation_membership.rs:107` policy; `federation_runtime.rs` | `cynic fleet discover --json` from two peers |
| Change firewall / DNS output | `firewall_policy.rs`, `dns.rs` (pure) — never hand-edit generated projections | `audit_firewall_ruleset` comparison proof |
| Add a binary or crate | §3 + `cynic-crate-policy` skill; explicit `[[bin]]`, boundary test | `make check` + new `architecture_boundary.rs` |
| Build or test | Builder only; `Makefile:16-66` policy | `make check` (sole admitted runner) |
| Touch `cynic-network` | §0.1: core must stay Zero-I/O; I/O goes to `adapters/` (does not exist yet — create it) | boundary tests green, incl. the currently-red reqwest ban |

## 5. Explicit unknowns (do not guess; close with evidence)

1. `/etc/cynic/registry.json` (runtime registry) content — read 2026-10-06:
   legacy vocabulary (`canonical "synapse"`, planes `{management,
   synapse}`, node keys `{management, synapse, local}`). Redeploying it
   in trio is BLOCKED until trio-capable binaries are built from this
   tranche and redeployed blue-green: currently deployed binaries read
   legacy keys and would go blind on a trio file. Unblock sequence:
   merge tranche → release → redeploy binaries (prove `strings <bin>`
   carries the new resolver) → backup runtime file → flip → read back.
2. Live fleet state (listeners, handshakes, resolver answers) — no probe
   run; Tier 2 is unobserved in this doc.
3. `cynic-release` end-to-end deploy flow — file layout listed, sequence
   not traced.
4. Which CLI command owns `mesh-plan` vs `/etc/cynic/registry.json`
   default (`network_surface.rs:117` vs `:161`) — two defaults seen,
   ownership not traced.
5. Coordination claim for this doc edit — no `cynic agent claim` surface
   reachable from this session; operator's direct orders are the recorded
   authority.

## 6. Distributed-IaC reading grid (cloud vocabulary → CYNIC layers)

CYNIC is a private, on-premises cloud whose control plane is code: the
fleet owns its hardware (Proxmox, Cortex, Builder) and reconciles it from
declarative desired state, like Terraform apply running continuously and
supervised — except the "provider" is our own substrate, not an API.

| Cloud model | CYNIC equivalent | Code anchor |
|---|---|---|
| IaaS (shared responsibility: provider owns the metal) | `cynic-host` substrate: observe + actuate CPU/mem/disk/network per machine, local or via SSH | `domain/<family>`, `adapters/local_*`, `fleet_*_observer` |
| PaaS (platform agents consume) | builds, workload scheduling, inference runtime, lifecycle | `cynic-build`, `cynic-workload`, `cynic-inference-runtime`, `cynic-lifecycle` |
| SaaS (what users/agents touch) | argv-typed CLI + MCP projections, JSON read-back | `cynic-cli`, `cynic-mcp` |
| Private cloud (exclusive, on-prem) | the whole fleet: `infra/registry.json` as desired state, no external provider in the loop | Tier 0–2 (§1) |
| Community / hybrid (federation without central trust) | pull-only enrollment, deterministic emergence, autonomous degradation | §2 L0–L4 |
| IaC state file | `infra/registry.json` (Tier 1, Git, read-only at runtime) — drift is an event, never a silent edit | `registry_maintenance.rs` writes `observed` only |

## Next slices (in order; one commit each after operator go)

1. **S1 — close the mesh-plan vocabulary root**: typed plane model
   (revive `plane.rs`, re-export, delete ad-hoc traversal) so the
   `or_else` fallbacks become compile errors; add one test pointed at the
   REAL `infra/registry.json` asserting non-empty peers.
2. **S2 — quarantine domain I/O**: move `headscale_client.rs` (+ finance
   adapters) under `cynic-network/src/adapters/`, drop `reqwest` from the
   domain surface the boundary test names; boundary suite green.
3. **S3 — reconcile node readers**: census of `get("synapse")` /
   `get("management")` legacy-only readers (CLI + node usecases) onto the
   revived enum; fixtures must not mask the rename (one test per reader
   against the real file).
4. Only then: extraction slices (host fragments → network; node network
   domain → network or `cynic-network-node`).
