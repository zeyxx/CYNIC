# Hermes X Legacy Surface Report (2026-06-27)

Scope: `organs/supervisor/controllers/validate_repo_topology.py` legacy Hermes-X
control points.

## Audit run

`python3 organs/supervisor/controllers/validate_repo_topology.py --json`

## Latest result

- `ok`: **True** (`python3 organs/supervisor/controllers/validate_repo_topology.py --json`)
- `critical`: **0** from topology checks after migration
- `medium`: **0** for new `legacy_hermes_x_systemd_*` findings

## Active-systemd migration status

All runtime service-file names and dependency edges using `hermes-x-*` were migrated to
canonical, non-legacy equivalents:

- `infra/systemd/hermes-x-ingest.service` -> `infra/systemd/hermes-ingest.service`
- `infra/systemd/hermes-x-ingest@.service` -> `infra/systemd/hermes-ingest@.service`
- `infra/systemd/hermes-x-login@.service` -> `infra/systemd/hermes-login@.service`
- `infra/systemd/hermes-x-recovery@.service` -> `infra/systemd/hermes-recovery@.service`

Dependency updates:

- `hermes-x-ingest.service` references now target `hermes-ingest.service`.
- Template unit links now target `hermes-ingest@%i.service`.
- `organs/twitter/systemd/organ-twitter-hermes-agent.service` now waits on
  `hermes-ingest.service`.

The canonical validator (`validate_repo_topology.py`) now treats this as the active
baseline for systemd migration checks.

## Contract debt snapshot (ground truth)

`axioms/repo-topology-v1.json` currently keeps explicit exception debt for
non-portable surfaces. The debt is acceptable **only because each item has a
replacement and expiry condition**.

### Organ-boundary / misplaced exceptions

- `organs/gasdf/` (`transitional`)  
  Replacement: manifest-backed organ or removal.  
  Expiry: the gasdf concern is either assigned an organ role or deleted.

- `organs/controllers/` (`legacy_blocked`)  
  Replacement: `organs/supervisor/controllers/`.  
  Expiry: all controller entrypoints are manifest-backed.

- `organs/freebox/`, `organs/rag/`, `organs/prompt-enricher/`,
  `organs/superteam/`, `organs/telemetry/` (`transitional`)  
  Replacement: explicit organ manifests or bounded adapters.  
  Expiry: explicit migration criteria in contract.

- `scripts/hermes-x/` (`legacy_blocked`)  
  Replacement: keep only compatibility shims then delete/migrate.  
  Expiry: no runtime implementation files remain and all callers migrated.

### Provider mutation / transitional script debt

- `scripts/proxmox_exec.py` (`legacy_blocked`)  
  Replacement: MCP `fleet_node_exec` with registry/provider preflight.

- `scripts/{audit_organism.py, kyon-goal-check.sh, portability-audit.py, scripts/enroll-mesh-windows.ps1, update_registry.py}` (`transitional`)  
  Replacement: bounded organ/supervisor entrypoints.

### Runtime state exceptions in source control

- `runtime_state_commit_exceptions` remains the mechanism used for historical state.
  It is intentional while migration and canonical ownership of state move to organ-local
  or `.cynic` boundaries.

## Follow-up controls

- Keep legacy file names (`hermes-x-*`) limited to historical/docs context.
- Any future rename or re-introduction should include `validate_repo_topology`
  update + service graph review before merge.
- Before each major change, refresh a concrete debt snapshot from
  `axioms/repo-topology-v1.json` and compare against active runtime.
- Any item with `legacy_blocked` but no active `status` proof must be treated as
  a burn candidate, not technical debt.

## Legacy surfaces retained by design

- `hermes-x-scripts` is intentionally retained with `legacy_blocked` status as a
  compatibility and evidence artifact in:
  - `scripts/hermes-x/MANIFEST.yaml`
  - generated topology modules (non-canonical)
- Any new runtime surface migration must remove this block status and provide a
  first-class `organs/*` manifest before canonical status is restored.

## Validator proof (now)

- `python3 organs/supervisor/controllers/validate_repo_topology.py --json`
- `ok`: true
- `findings`: none

This means the validator accepts the current debt set as explicitly declared and
currently bounded. It also means no additional hardcoded absolute deployment
projection path was detected in active `infra/systemd/*.service` or
`organs/*/systemd/*.service`.
