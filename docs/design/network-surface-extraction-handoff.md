# Network surface extraction — handoff to the parallel session

**Status:** SPECIFIED, NOT STARTED. No code was written for this. The owning
session is running separately and owns `cynic-network`, `plane.rs`, the
`planes` / `nodes[].network` vocabulary in `infra/registry.json`, and the whole
network surface. Do not start this work in another thread.

Owner: the session currently reworking the network surface.
Blocked on: nothing in this spec. The measurements below are complete.

---

## What this session already did to the network surface

Three commits moved 16 CLI surfaces out of the `cynic-node` daemon and into
`cynic-cli`. This was the composition-root work, not a network redesign.

| commit | what |
|---|---|
| `5eeac52c` | deleted `cynic-node/src/facade.rs` (187 LOC, 8 of 13 functions dead) |
| `f3ccc81a` | daemon owns only `control`; 16 subcommands return explicit error |
| `f331fa53` | `git mv` of `cli.rs`, `cli_dispatch.rs`, `network_surfaces_uc.rs` into `cynic-cli/src/node_surface/` |
| `49636d41` | merged `node_activation_surface.rs` + `node_surface/cli.rs` into one unified `NodeCommand` (24 variants, 7 submodules) |
| `d3868a58` | reconciled the ledgers after those moves |

The 15 module inversions these moves created were paid and removed from
`infra/module-inversion-ledger.json` in `d3868a58`.

---

## The one violation this session could not close

```
ARCH-001-HOTSPOT|crates/cynic-cli/src/node_surface/network_surfaces_uc.rs
```

1 417 lines against a 400-line hard threshold. It is the only remaining
critical in `make audit`.

**It was not declared in the ledger.** Declaring it would make the gate green
by absorbing new debt silently, which is what the ledger exists to prevent. It
is new debt, and the honest state is red.

### Measured anatomy — this is two objects welded together

| block | lines | what it is |
|---|---|---|
| `NetworkCommands` enum | 10–471 | 461 lines, 29 variants. Mostly doc-comments and `#[arg]` attributes; median 16 lines per variant |
| `execute_network_command` | 531–1340 | 809 lines of dispatch |
| helpers + tests | 1342–1417 | 76 lines |

### The dispatch is flat, not deep

- median arm: **5 lines**
- **78 of 99** arms are ≤ 10 lines
- only **2** arms exceed 40 lines: `SecurityAudit` (41), `MeshCoordination` (39)

Do not factor the 78 short arms. That would be noise. A 5-line dispatch arm is
readable; abstracting it produces indirection with no reduction.

---

## Spec, in dependency order

### Step 1 — move the enum out (461 lines, zero risk)

`NetworkCommands` is a **type**, not logic. Moving it changes no behaviour.

Target: `cynic-cli/src/node_surface/network_commands.rs`, or fold it into the
existing `network_surface.rs` enum if the network session has already unified
those.

Effect: 1417 → ~950 lines.

Do this first because it is provably behaviour-neutral. Every later step moves
logic, and a reviewer comparing behaviour should be looking at a diff that is
only the enum.

### Step 2 — the 2 deep arms become use cases

`SecurityAudit` (41 lines) and `MeshCoordination` (39 lines) are use cases
wearing a dispatch arm's clothes. They call:

- `cynic_node::usecases::network_watchdog_uc`
- `cynic_node::network_repair`
- registry reads for headscale / wireguard / allowed services

Move them behind an explicit port into `cynic-node`, where their adapters
already are. This is the same correction applied to the 15 inversions that
were already paid: a lower layer calling a concrete module means the boundary
is missing.

Effect: hotspot under 400, and `cynic-cli`'s remaining reach into
`cynic_node::` (28 direct symbol references at measurement time) drops.

### Step 3 — what is deliberately NOT done

- **Do not merge `network_surfaces_uc.rs` into `network_surface.rs` blindly.**
  Measured: `network_surface.rs` (456 LOC) and its `network_surface/`
  submodules (`base.rs`, `dns.rs`, `mesh.rs`, `provider_fallback.rs`,
  `actuator.rs`, `domain.rs`) are **thin wrappers** that construct a
  `NetworkCommands` value and call `execute_network_command`. They are not
  duplicate implementations — they are adapters. Merging them naively would
  inline the adapters back into the dispatch and re-create the thing step 2
  removes.
- **Do not resolve the vocabulary conflict by renaming in Rust.**
  `infra/registry.json` declares `planes = {service, admin, mesh}` while
  `nodes[].network` uses `{synapse, management, local}`. The intersection is
  empty. 38 `.or_else()` fallback sites guess which vocabulary they are reading;
  171 string literals hardcode the node vocabulary; 203 private IPs are
  hardcoded in production. This session deliberately left all of it alone —
  it is the network session's decision, and fixing the literals before the
  registry has one typed model would just re-add them.
- **Do not declare the hotspot in `infra/audit-debt-ledger.json`** to turn the
  gate green. If the extraction above cannot complete, the red gate is the
  correct state and the debt belongs in the ledger with a deliberate entry, not
  with a silent one.

---

## Constraints that apply to whoever picks this up

- The gate now validates the source, not the install (`9cfa033e`). `make audit`
  runs the binary built from HEAD. A stale `/usr/local/bin/cynic` can no longer
  hide findings. Do not "fix" a red gate by reinstalling an old binary.
- `make audit.deployed` is the separate question — it checks whether the
  installed artifact describes the commit it claims. It currently reports
  `blocked_stale_installed_artifact`.
- Runtime plane vocabulary has a canonical type at `cynic-protocol/src/plane.rs`
  (`Plane::{Service, Admin, Mesh}`, `from_name`, `name`, `dns_zone`,
  `purpose`, `federation_scope`) with **zero consumers**. It is the obvious SSOT
  candidate for the two-vocabulary conflict above. It is not exported from
  `cynic-protocol/src/exports.rs` today.

---

## Verification the owning session must produce

```
make check                      # ok
make audit                      # must reach 0 critical, or declare the debt
cargo run -q -p cynic-host --bin cynic-topology -- --check   # up_to_date
```

`cynic-topology --write` regenerates `infra/crate-topology.json` from the
manifests. It refuses to write a document ARCH-004 would flag. Run it after any
dependency change — the file drifted twice in this session because nobody did.
