# Agent CLI / Host Surface Patterns (anti-monolith)

These rules are durable agent law for both Grok and Codex sessions. They exist
because large single-file "surfaces" reintroduce the patterns we are deleting.

## Structure

- Prefer layered modules over one file that owns types + policy + I/O + spawn +
  reports. Reference layouts:
  - `crates/cynic-host/src/domain/ecosystem_package/`
  - `crates/cynic-host/src/update_lifecycle/`
- Split at least: `types` (pure shapes), `policy`/`plan` (pure tables/logic),
  `port` (traits for execution/observation), adapters, and thin CLI glue.
- Do not grow a domain file past ~400 lines without extracting a port or a pure
  submodule. If a change would force that, split first.

## Execution ports

- Domain/host logic must **not** hard-wire `std::process::Command::new("cynic")`
  (or shell composition) for sibling CLI surfaces. Inject an argv runner port
  (`ArgvRunner` / local adapter / map stub for tests).
- CLI crates inject the production runner; unit tests inject fakes that return
  fixed JSON. Gate/repair/apply paths must be unit-testable without a real
  `cynic` binary on PATH.
- Mutable work still requires explicit `--yes` / `--apply`. No silent auto-repair,
  no fail-closed mock that pretends success when an endpoint is missing.

## CLI thinness

- `cynic-cli` surfaces: clap parse → inject ports → call host/crate API → print
  human or JSON. No phase policy tables, no finding classifiers, no state schema
  ownership in the CLI crate.
- One canonical command path; register it in `surface_registry` +
  `infra/agent-exposure.json` when the surface is agent-facing.
- Machine-readable reports are the proof surface; human text is a projection.

## Surface grammar

- Default grammar for non-trivial CLI/runtime surfaces:
  - facade/orchestrator
  - `types`
  - decision/plan/catalog or other pure tables
  - scan/observe/path discovery helpers
  - checks/predicates/classifiers
  - persistence/output helpers when the surface writes artifacts
- Do not collapse these roles back into one file once they have been split.
- If a surface needs more than one of:
  - state schema ownership
  - filesystem/report persistence
  - source/path discovery
  - rule evaluation
  - command dispatch families
  then it should already be a module tree, not a single `*.rs` file.
- Preferred thin-facade pattern in `cynic-cli`:
  - parse command
  - resolve injected context
  - delegate to family module
  - emit JSON/human projection
  - return exit code

## Audit grammar

- Default grammar for audit surfaces:
  - facade/assembly
  - `types`
  - `scan` or source/JSON discovery
  - `checks` / predicates / classifiers
  - shared path helpers where the audit walks source trees
- Audit surfaces must not each reinvent:
  - workspace root discovery
  - Rust file traversal
  - pretty JSON file writing
  - machine-local path detection
  when a shared helper already exists.
- When two audits need the same primitive, extract it on the second occurrence.

## Shared helpers

- Shared helper modules are preferred over copy-pasted local utilities once a
  pattern repeats across surfaces. Current canonical examples:
  - `cli_json` for stdout JSON projection
  - `fs_json` for pretty JSON file persistence
  - `rust_source` for workspace-relative Rust source discovery
- A shared helper must stay primitive. It should not quietly absorb domain
  policy.

## Growth thresholds

- Soft threshold: split before a CLI/audit facade grows past ~200-250 lines.
- Hard threshold: any single surface file above ~400 lines must be treated as
  extraction debt unless it is already a stable pure data table or test-heavy
  fixture file.
- If a file stays large on purpose, the reason should be obvious from its role
  (for example: pure registry table, generated schema projection, or dense test
  fixture data), not because unrelated responsibilities accumulated there.

## What not to reintroduce

- Monolith `*_lifecycle.rs` / `*_surface.rs` that re-implements half the runtime.
- Python organs or docs as runtime authority.
- Hardcoded operator machine paths, distro names, or principal homes in domain
  code when a layout/capability can be injected.
- Parallel "agent helper" shells that bypass argv-typed execution.
