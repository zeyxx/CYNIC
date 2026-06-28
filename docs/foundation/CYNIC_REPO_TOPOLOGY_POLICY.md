# CYNIC Repo Topology Policy

Status: normative foundation.

Machine contract: `axioms/repo-topology-v1.json`.
Thematic ownership contract: `axioms/organism-theme-topology-v1.json`.

This policy defines where organism concerns live in the repository. It exists
because durable policy written only in prompts, journals or one-off docs
disappears; durable policy written in the wrong directory creates competing
truths.

When the question is not just "which directory owns this file?" but "which
domain owns this claim?", use `docs/foundation/CYNIC_THEME_TOPOLOGY_POLICY.md`
and `axioms/organism-theme-topology-v1.json`.

## 1. Core Rule

Every file must belong to one of these classes:

| Class | Primary path | Meaning |
|-------|--------------|---------|
| Policy | `docs/foundation/` | Cross-cutting laws and governance rules |
| Semantic contract | `axioms/` | Machine-readable payloads, APIs and profiles |
| Architecture map | `docs/architecture/` | How current implementation maps to target design |
| DNA / desired state | `infra/registry.json`, `infra/agent-exposure.json` | Organism-wide desired state and exposed capabilities |
| Kernel | `crates/cynic-kernel/` | Bus, memory, coordination, health and storage |
| MCP surface | `crates/cynic-mcp/` | Portable agent-facing tool surface |
| Organ | `organs/<role>/` | Persistent bounded subsystem |
| Shared runtime framework | `services/` | Framework code reused by multiple organs or agents |
| Deployment projection | `infra/systemd/`, `infra/docker/`, `infra/firewall/` | Concrete incarnation of desired state |
| Transitional wrapper | `scripts/`, `infra/scripts/` | Operator wrapper or diagnostic only |
| Local runtime state | `.cynic/`, `organs/<role>/state/` | Non-canonical state, logs, caches |

If a file does not fit one class, the concern is not understood well enough to
extend safely.

Every root class in `repo-topology-v1.root_classes` must declare:

- `class`: the stable concern category.
- `owns`: what the path is allowed to define.
- `must_not_own`: concerns that must move to another layer.

Top-level directories are organism species. A new root directory is forbidden
until it is either:

- declared in `axioms/repo-topology-v1.json:root_classes`; or
- explicitly ignored by `top_level_root_policy.ignored_roots` because it is
  local build/runtime noise such as `.git`, `.venv`, `target` or
  `node_modules`.

This is the first organic correction layer: classify the body before moving
organs. It prevents hidden organs, hidden policy and hidden runtime state from
appearing as accidental root folders.

The second organic correction layer is organ maturity. A directory under
`organs/` is not simply "present" or "absent"; it must be in one declared state:

| Maturity | Meaning |
|----------|---------|
| `canonical` | Manifest-backed and structurally complete for its runtime |
| `canonical_partial` | Bounded organ with explicit missing part, owner and expiry |
| `transitional` | Tolerated while being migrated; no new canonical behavior |
| `embryonic` | Undifferentiated organ seed; must gain a boundary or be absorbed |
| `quarantined` | Read-only migration/deletion surface |
| `legacy_blocked` | Fails closed except guarded emergency fallback |
| `burn_candidate` | No durable role is proven; deletion/absorption preferred |

`axioms/repo-topology-v1.json:organ_maturity_policy` is the machine vocabulary.
Organ manifests and organ exceptions must use that vocabulary.

The third organic correction layer is the organ maturity inventory.
`axioms/organ-maturity-inventory-v1.json` must list every immediate
`organs/<role>/` directory exactly once. The inventory is not a backlog; it is a
machine projection of the current body:

- canonical organs are visible instead of implied by absence from exceptions.
- partial, transitional, embryonic, quarantined and blocked organs are visible
  in one scan.
- adding or deleting an organ requires updating the inventory, which makes
  organism shape drift fail in the topology gate.

## 2. Dependency Direction

Allowed direction is top-down:

```text
policy -> contract -> registry/exposure -> kernel/MCP -> organ core
       -> organ adapter -> organ entrypoint -> deployment projection -> wrapper
```

A lower layer may implement or project a higher layer. It must not redefine it.

Examples:

- `infra/systemd/*.service` may project registry placement; it must not decide
  organ identity.
- `crates/cynic-mcp/` may expose `network.ipam.check`; it must not invent IPAM
  policy or hardcode project paths.
- `organs/wireguard/src/adapters/` may write CoreDNS or WireGuard config; it
  must not decide which service owns which canonical address.
- `scripts/` may diagnose or wrap; it must not become the canonical supply
  chain or provisioning path.

## 3. Canonical Organ Shape

Every persistent organ uses this boundary:

```text
organs/<role>/
  MANIFEST.yaml
  README.md
  src/
    core/
    adapters/
    entrypoints/
  tests/ or src/tests/
  systemd/
  state/
```

Rules:

0. `MANIFEST.yaml` declares a stable identity (`id` or `name`), lifecycle
   `status`, and `description`.
1. `src/core/` contains entities, ports and use cases; no external I/O.
2. `src/adapters/` contains provider, filesystem, network, kernel and database
   implementations.
3. `src/entrypoints/` wires adapters to use cases.
4. `state/` is local runtime state and is not committed by default.
5. `scripts/` inside an organ is transitional unless the organ README declares
   owner, expiry condition and replacement path.

## 4. Runtime State

Runtime state is not organism memory.

Allowed runtime locations:

- Kernel storage and mempool for durable dynamic memory.
- `.cynic/` for local cache/logs.
- `organs/<role>/state/` for organ-local state.
- explicit fixtures under test directories.

Committed runtime JSON/JSONL is blocked by default unless it is explicitly a
fixture, schema, desired state or operator contract.

## 5. Transitional Paths

These paths are tolerated but must not receive new canonical behavior:

| Path | Status | Replacement |
|------|--------|-------------|
| `scripts/` | transitional | organ entrypoint or MCP operation |
| `infra/scripts/` | transitional | infra contract + organ adapter |
| `organs/lifecycle/` legacy script surface | burned | registry/MCP fleet and storage operations through bounded lifecycle adapters |
| `services/cynic-python/organs/core/` | boundary under clarification | shared Hermes/Ouroboros runtime framework |

Adding a new file to a transitional path requires fallback metadata:

```json
{
  "owner": "...",
  "allowed_file_kinds": ["operator_wrapper", "diagnostic"],
  "forbidden_concerns": ["canonical_lifecycle", "durable_policy"],
  "scope": "...",
  "expires_when": "...",
  "replacement": "...",
  "observability": "..."
}
```

At the root level, every entry in
`repo-topology-v1.current_transitional_paths` must declare:

- `path`: the tolerated transitional root.
- `owner`: who is accountable for burning or migrating it.
- `allowed_file_kinds`: the only classes of files tolerated there.
- `forbidden_concerns`: concerns that must move to contracts, registry, organs
  or MCP.
- `replacement`: the target canonical layer.
- `expiry_condition`: the objective condition that removes the exception.

Organ-level exceptions are not informal. If a directory under `organs/` lacks
`MANIFEST.yaml`, it must be declared in
`repo-topology-v1.organ_boundary_exceptions` with owner, status, replacement and
expiry condition. If a manifest-backed Python organ lacks `src/core`,
`src/adapters` or `src/entrypoints`, it must be declared in
`repo-topology-v1.organ_shape_exceptions` with the exact missing paths. Stale
exceptions fail the topology gate.

Runtime-state and provider-mutation exceptions follow the same rule. Tracked
runtime JSON/JSONL under organ state paths must match
`repo-topology-v1.runtime_state_commit_exceptions`; new committed runtime state
without that declaration fails the topology gate. Provider mutation scripts in
transitional roots must match
`repo-topology-v1.provider_mutation_script_exceptions` with owner, guard,
replacement and expiry condition.

New wrappers in transitional roots must match
`repo-topology-v1.transitional_wrapper_file_exceptions` before they can become
part of the source tree. This applies even when they are read-only diagnostics:
`scripts/` is not an unbounded API namespace. Each exception must declare path,
status, owner, classification, replacement and expiry condition.

A `MANIFEST.yaml` under `scripts/` is not a wrapper; it is an organ hiding in a
transitional root. Such paths are forbidden unless declared in
`repo-topology-v1.misplaced_organ_exceptions` with owner, target organ,
replacement and expiry condition. The exception is a migration fence, not a
canonical home.

The contract namespace follows the same boundary discipline. Files under
`axioms/` are `.json` machine contracts by default, with top-level
`schema` and `status`. Subdirectories under `axioms/` must be declared
in `repo-topology-v1.contract_file_policy.allowed_subdirectories` with class,
owner and reason. This keeps consumer bindings such as
`axioms/consumer-profiles/` explicit instead of letting contracts become
a mixed documentation drawer.

## 6. File Move Policy

Do not mass-move the repo while core organs are unhealthy. Migration is
progressive:

1. Classify the path in `repo-topology-v1`.
2. Mark it canonical, transitional, quarantined or legacy blocked.
3. Add or update the owning manifest.
4. Move only when tests and runtime references prove the path is not live.
5. Delete obsolete paths instead of leaving parallel implementations.

Git is the archive. The repo should not keep dead files as emotional backup.

## 7. Enforcement Target

The minimum gate must eventually verify:

- JSON contracts parse.
- contracts under `axioms/` remain machine contracts: `.json` by
  default, top-level `schema`, top-level `status`, declared status values, and
  declared subdirectories.
- no new hardcoded absolute paths in portable MCP proxy code.
- no new provider mutation script without fallback metadata.
- no new committed runtime state without manifest exception.
- Python organs preserve `core/`, `adapters/`, `entrypoints/` separation.

The `lint-repo-topology-contract` gate currently enforces:

- JSON contracts parse.
- JSON contracts under `axioms/` use allowed extensions and declare
  top-level `schema` and `status` fields matching
  `repo-topology-v1.contract_file_policy`.
- subdirectories under `axioms/` are declared in
  `repo-topology-v1.contract_file_policy.allowed_subdirectories`.
- topology authorities exist.
- topology root classes declare `class`, `owns` and `must_not_own`.
- every immediate `organs/<role>/` directory is represented exactly once in
  `axioms/organ-maturity-inventory-v1.json` with a declared maturity status.
- portable MCP Rust code does not contain host/project-specific absolute paths.
- every immediate `organs/<role>/` directory has `MANIFEST.yaml` or a declared
  boundary exception.
- manifest-backed organs declare identity, lifecycle status and description
  according to `repo-topology-v1.organ_shape.manifest_policy`.
- manifest `scripts[].name` entries that look like file paths are audited as
  non-blocking findings when they point to missing or untracked files; this is
  currently a visibility layer while historical manifests are being corrected.
- manifest-backed Python organs either expose `src/core`, `src/adapters` and
  `src/entrypoints`, or carry an exact, non-stale shape exception.
- tracked runtime state is either quarantined in the topology contract or
  blocked from becoming source of truth.
- provider mutation scripts in transitional roots are either declared
  legacy-blocked with guards or blocked from becoming canonical paths.
