# CYNIC Agent CLI Framework — Classification & Placement

Status: normative foundation.

Purpose: disambiguate agent CLI surfaces and persistent agents that touch the
CYNIC organism, so operators never conflate a local inference CLI with an
autonomous organ, and never mistake an external scheduler framework for a
CYNIC authority.

This is the single source for the question: "which agents can act on CYNIC,
and how is each bounded?" All other mentions of agent/classification names in
`docs/`, `crates/`, or prompts must defer to this hierarchy.

## 1. Scope

Covers three disjoint namespaces that are routinely conflated:

1. Cortex reasoning clients — interactive, human-in-the-loop, invoked per
   session (Codex, Claude Code, **Antigravity**).
2. Autonomous organ agents — persistent daemons owned by the CYNIC executor and
   subject to the same coordination guarantees as an interactive cortex
   (`hermes-agent`).
3. External scheduler frameworks — infra tooling that runs jobs/sandboxes but is
   NOT a CYNIC component and must never be granted organism authority
   (`mercury-agent`).

Out: build/proposal surfaces (`organ-anvil`, `organ-forge`), validation dogs,
and kernel-internal scheduling primitives. Those are organs/tasks, not agent
CLIs.

## 2. Status of `gemini-cli`

`gemini-cli` is **deprecated and unsupported** as a CYNIC cortex client. It was
never a declared CYNIC agent surface: the only historical mention in the repo
(`crates/cynic-kernel/domains/wallet-judgment-dogs.md:231`) used it as a
**pattern-matching heuristic** for Sybil/wallet clustering, not as an execution
adapter. No config, provider pin, or exposure manifest references `gemini-cli`.

Replacement: `antigravity-cli`.

Rule: never add `gemini-cli` to `infra/agent-exposure.json`,
`infra/ecosystem-pins.json`, or any `cynic-*` launch recipe. Quarantine the old
mention as a deprecated heuristic, not an active surface.

## 3. Canonical classification

| Name | Kind | Source / repo | Scale (per policy matrix) | Layer | Authority class | Declared in |
|------|------|---------------|---------------------------|-------|-----------------|-------------|
| `antigravity-cli` | cortex reasoning client (CLI) | agentic-cli/antigravity | Cortex | Runtime adapter (local) | Human-supervised cortex | `infra/ecosystem-pins.json:antigravity-cli`, `infra/agent-projections/antigravity-mcp.json` |
| `Codex CLI` (`codex`) | cortex reasoning client (CLI) | OpenAI | Cortex | Runtime adapter | Human-supervised cortex | `AGENTS.md`, agent adapter docs |
| `Claude Code` (`claude`) | cortex reasoning client (CLI) | Anthropic | Cortex | Runtime adapter | Human-supervised cortex | `AGENTS.md`, agent adapter docs |
| `cynic` CLI / `cynic-mcp` | portable surface (projection only) | this repo | Kernel / Adapter | MCP surface | Projection only | `crates/cynic-cli`, `crates/cynic-mcp` |
| `hermes-agent` | autonomous organ agent (persistent) | NousResearch/hermes-agent | Organ | Runtime adapter | Autonomous organ agent | `docs/organs/HERMES_AGENT.md`, `AGENTS.md`, `infra/agent-exposure.json` |
| `mercury-agent` | external scheduler framework | cosmicstack-labs/mercury-agent | External | N/A (not a CYNIC layer) | None (external) | NOT part of CYNIC; no references in source |

### 3.1 Why `hermes-agent` and `mercury-agent` are not interchangeable

- `hermes-agent` (NousResearch) is CYNIC's own persistent autonomous organ: it
  claims tasks from the kernel mempool, emits proposals/proofs, and obeys the
  `/coord/register` + `claim-before-edit` guarantees from
  `docs/CYNIC_ORGANISM_POLICY_MATRIX.md:130`.
- `mercury-agent` (cosmicstack-labs) is a standalone infra scheduler
  (sandbox/container lifecycle). It has **no** standing in the CYNIC contract
  hierarchy: it is not wired into `infra/registry.json`, not declared in
  `infra/agent-exposure.json`, and not invoked by any `cynic-*` port. Importing
  its types or spawning it from a CYNIC surface is treated as a **non-canonical
  probe** that must be declared as a temporary exception (§7), never as a
  persistent runtime adapter.
- Do not alias `mercury` to the hermes organ identity (canonicalized by
  `cynic-protocol::canonicalize_dog_runtime_id` in the inference benchmark plan).
  The benchmark plan ships two scenarios: `hermes-read-only` (hermes organ) and
  `pi-read-only` (rival cortex client). There is no `mercury-read-only`.

## 4. Cortex client selection policy

Default cortex reasoning client is selected bottom-up:

1. `CYNIC_CORTEX_CLIENT` env if set and validated, else
2. `antigravity-cli` when present on PATH (local, open, no vendor API key), else
3. `Codex CLI` / `Claude Code` as operator fallback.

`gemini-cli` is never selected: see §2. A client is only "active" when it has
a row in `infra/agent-exposure.json` with a non-empty
`CYNIC_AGENT_TYPE` and a passing freshness probe.

## 5. Authority & boundary map

- **hermes-agent** (autonomous organ agent) may: claim `/agent-tasks`, register a
  runtime agent via `/coord/register`, emit proposals to the proposal queue, and
  emit proof bundles. It may **not** mutate `docs/` or `crates/` directly; it
  routes repo changes through `organ-anvil` as a declared target (policy matrix
  §6.7/§11).
- **antigravity-cli / Codex / Claude** (interactive cortex) may: register, read
  attention, claim before edit, observe material facts, release — manually via
  MCP/CLI, with every repo-affecting change gated by `make check`.
- **mercury-agent** (external) has **no** CYNIC authority. Any invocation of it
  from a CYNIC surface must be wrapped as a throwaway diagnostic behind an
  explicit exception (§7) and proved not to mutate organism state.
- `cynic` CLI and `cynic-mcp` are projections only: they never own domain policy
  tables (anti-monolith rule, AGENS.md: structural invariants §2).

## 6. Provider pinning & ecosystem pins

`infra/ecosystem-pins.json` is the machine-readable pin. Current relevant rows:

- `id: "antigravity-cli"`, `runtime_kind: "local"`, `is_open: true` — canonical
  open cortex CLI, selected above.
- No `gemini-cli` pin exists and must not be added.
- No `mercury-agent` pin exists; do not add one (it is not a CYNIC provider).

When adding a new cortex CLI, add a pin row (not a prompt/config), then a
freshness projection in `infra/agent-projections/*.mcp.json`.

## 7. Exception policy for external schedulers

If an operator must drive an external scheduler (e.g. to run a sandboxed build
or to pilot a probe), it is a temporary exception only:

```json
{
  "fallback": "external scheduler as diagnostic",
  "reason": "canonical layer unavailable for sandbox lifecycle",
  "owner": "human operator",
  "scope": "single task id, no CYNIC state mutation",
  "expires_when": "sandbox reachable via a CYNIC-native surface",
  "observability": "read-back proof on /observe",
  "replacement": "provider-local sandbox port under infra/registry.json"
}
```

`mercury-agent` used as an external scheduler is therefore permitted only under
§7, never as a hermes organ identity, and only while the above JSON is committed
and the exception owner is live.

## 8. Review checklist (agent-class additions)

Before introducing or renaming an agent surface:

1. Place it on the correct row of §3 (cortex client / organ agent / external).
2. Confirm it is NOT `gemini-cli`; if it was, add a deprecation note in §2.
3. Confirm it is NOT `mercury-agent`; if it looks like an external scheduler,
   gate it under §7 and never expose it as a hermes dog identity.
4. Add/update a pin in `infra/ecosystem-pins.json` and a projection in
   `infra/agent-projections/` if it is a cortex client or organ agent.
5. Never alias the external scheduler identity onto `canonicalize_*` runtime IDs.
6. Prove via `make check` and an exposure freshness probe.
7. Re-run `cynic dev agent-exposure --json` (see AGENTS.md: skills) to keep
   `infra/agent-exposure.json` green.

## 9. Read-back / proof

- `cynic-protocol::canonicalize_dog_runtime_id` must resolve `hermes-agent` and
  must NOT contain a `mercury-agent` arm (verified by grep:
  `canonicalize_hermes_runtime_id` has zero residual references and the
  inference benchmark plan contains no `mercury-read-only` scenario).
- `infra/ecosystem-pins.json` must contain `antigravity-cli` and must NOT
  contain `gemini-cli`.
- `docs/CYNIC_META_LAYER_CONTRACTS.md` must not list `mercury-agent` under
  autonomous organ agents.

## 10. Enforcement & current gate status

This framework is enforced at two levels:

- build admission (`make check` / `make check-strict`) — proves compile safety
  and surface conformance.
- exposure freshness (`cynic dev agent-exposure --json`) — proves agent surfaces
  are declared and live in `infra/agent-exposure.json`.

Current observed gate status (bottom-up read-back):

- ✅ `hermes-agent` vs `mercury-agent` classification is consistent across
  `docs/CYNIC_AGENT_CLI_FRAMEWORK.md` §3/§3.1 — `canonicalize_dog_runtime_id`
  has zero `mercury` arms, `infra/inference-benchmark-plan.json` ships only
  `hermes-read-only`, no `mercury-read-only`. Verified via grep + `cargo check
  -p cynic-protocol` green.
- ✅ `gemini-cli` is unpinned in `infra/ecosystem-pins.json` and absent from
  `infra/agent-exposure.json`; `antigravity-cli` is the canonical open cortex
  CLI.
- ✅ core Rust runtime crates compile green after the refactor:
  `cynic-protocol` (grep-clean, `cargo check -p cynic-protocol` EXIT 0),
  `cynic-host`, `cynic-node` (fleet_truth E0761 orphan resolved),
  `cynic-inference` (orphan `hermes_runtime_port`,
  `hermes_process_supervisor`, `hermes_run_executor`, and
  `harness_executor` modules declared in their `mod.rs`, obsolete
  `LocalRuntimeConvergenceReport` import removed, and
  `InferenceReply.text_response` field added to all initializers), and
  `cynic-observer` (orphan `fanout_runner`/`provider_probe`/`decode` type
  inference + move resolved) — verified `cargo check -p cynic-observer` green
  on `rustc 1.97.1 (stable)` to bypass the documented `rustc 1.96.0` ICE.
- 🔴 `make check` is currently blocked downstream by two residual conditions,
  both outside the hermes/mercury scope and preserved as explicit unknowns:
  1. **toolchain ICE** — `rust-toolchain.toml` pins `rustc 1.96.0`, which the
     Makefile itself notes "exhibited ICE/SIGSEGV here" (Makefile:24, 446, 455).
     The ICE is reproduced on the `1.96.0` toolchain and is bypassed by building
     the core crates on `rustc 1.97.1 (stable)` (use
     `RUSTUP_TOOLCHAIN=stable`).
  2. **cynic-mcp kernel-extraction debt** — `crates/cynic-mcp` carries a
     non-finished refactor (orphan `types_split/` directory with no
     `mod.rs`/`pub use`, a duplicate `tool_router_forward` definition — one via
     the `#[tool_router]` macro in `routers/forward.rs` and one hand-written in
     `proxy.rs` — and an obsolete `LocalRuntimeServiceAny::start_with_transition`
     call site in `lifecycle_inference.rs`). This is kernel-extraction debt
     (owner: MCP/kernel layer), not a classification error.

Per the policy matrix, unknown/blocked layers must be reported as residual
risk, not papered over. The hermes-vs-mercury refactor is therefore
compile-proven at its own layer even while `make check` remains blocked by the
upstream items in §10 (residual unknown).

