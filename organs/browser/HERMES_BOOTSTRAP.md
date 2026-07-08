# Hermes Agent Bootstrap Contract

`NousResearch/hermes-agent` is the upstream bootstrap framework for
`organ-browser`.

This does not make Hermes Agent the CYNIC kernel. CYNIC remains the authority for
coordination, memory, evidence, human gates, and organism policy. Hermes Agent is
the upstream implementation substrate for browser-capable agent tooling.

## Upstream

- Repository: `https://github.com/NousResearch/hermes-agent`
- License at review time: MIT
- Primary browser modules to study and adapt:
  - `tools/browser_tool.py`
  - `tools/browser_supervisor.py`
  - `tools/browser_cdp_tool.py`
  - `tools/browser_camofox.py`
  - `toolsets.py`
  - `model_tools.py`
  - `mcp_serve.py`
  - `hermes_state.py`

## Strategic Decision

`organ-browser` must not evolve as a small ad-hoc Playwright wrapper. Its target
shape is a CYNIC-governed adaptation layer over Hermes Agent's mature browser and
tool registry model.

Hermes Agent provides the practical substrate:

- browser tool surface: navigate, snapshot, click, type, scroll, press, back,
  console, CDP, dialog handling, screenshots, image extraction, and vision
- browser backend model: local Chromium through `agent-browser`, Browserbase,
  Browser Use, Camofox, and raw CDP
- task/session isolation via task identifiers
- accessibility snapshots with stable element refs
- CDP supervisor for dialogs, frames, console events, and persistent browser
  state
- tool registry and toolset gating
- MCP server/client patterns and tool filtering
- SQLite/FTS session search patterns that can inform local organ state without
  replacing the kernel ledger

## CYNIC Boundaries

The following surfaces stay CYNIC-native and must not be delegated to Hermes
Agent internals:

- coordination and claims: CYNIC kernel
- durable organism memory: kernel mempool and `cynic_observe`
- evidence policy: `organ-browser` contracts and redaction rules
- mutation approval: explicit human gates
- revenue/opportunity routing: `organ-superteam` and kernel tasks
- secrets: `~/.cynic-env` and organ-local state only

Hermes adapters may execute browser actions, but they do not decide whether a
mutation is allowed.

## Native Integration Without Coupling

Hermes Agent is a native CYNIC execution substrate only at the adapter boundary.
It must not become a transitive dependency of CYNIC core, organ policy, kernel
memory, or prospect/business logic.

Allowed integration paths:

- external source checkout observed through `organ-framework-map` or
  `organ-contracts`, for example with `HERMES_AGENT_SOURCE_ROOT` or an operator
  supplied `--source-root`.
- process boundary through the `hermes` CLI, gateway, ACP, MCP, or a declared
  systemd executor.
- browser tool execution through `organ-browser` ports and policy adapters.
- capability discovery normalized into `cynic.agent_framework_capabilities.v1`.
- task execution normalized into `cynic.agent_task_result.v1` or another
  declared CYNIC schema.

Forbidden integration paths:

- importing Hermes Python modules from CYNIC core or organ core logic.
- reading Hermes memory as organism memory.
- letting Hermes toolsets decide CYNIC policy, claims, human gates, or evidence
  redaction.
- committing upstream Hermes runtime state, cookies, profiles, session logs, or
  generated skill state into the repo.
- building CYNIC features against raw Hermes internal classes instead of CYNIC
  ports/contracts.

Practical shape:

1. Keep `NousResearch/hermes-agent` as an external checkout or installed runtime.
2. Scan its capabilities with the framework/capability organs.
3. Select Hermes as a route candidate only through the execution envelope.
4. Execute through a process or browser adapter.
5. Normalize results back into Kernel tasks, observations, and evidence bundles.

This gives CYNIC first-class Hermes power while preserving substitutability:
another framework may implement the same CYNIC contracts without changing organ
business logic.

## Integration Phases

### Phase 0: Contract

Status: current.

- declare Hermes Agent as upstream bootstrap
- define adapter-oriented ports for Hermes-compatible browser tool calls
- keep existing minimal Hub operational for Codex MCP activation

### Phase 1: Local Agent-Browser Adapter

Status: started with `src/adapters/hermes_agent_browser_adapter.py`.

Add an adapter that maps CYNIC browser intents to Hermes-style browser calls:

- `browser_navigate`
- `browser_snapshot`
- `browser_click`
- `browser_type`
- `browser_scroll`
- `browser_press`
- `browser_console`
- `browser_cdp`
- `browser_dialog`

Default backend must be local/sovereign. Cloud backends are disabled unless an
operator explicitly configures them.

### Phase 2: CDP Supervisor

Replace the minimal Hub interaction model with a persistent per-task supervisor:

- one isolated browser session per `task_id`
- dialog/frame/console event capture
- accessibility snapshot as primary evidence
- screenshots as optional local evidence
- raw CDP as an escape hatch, still policy-gated

### Phase 3: Tool Registry

Adopt a registry pattern compatible with Hermes Agent toolsets:

- tools declare schema, risk level, and required policy layer
- MCP exposes only tools whose risk is allowed by the current policy
- mutation-class tools remain unavailable until human gate material is attached

### Phase 4: Superteam/RAG Loop

Use the browser organ to feed opportunity work:

- discover bounties/grants/leads
- extract requirements into structured JSON
- build evidence bundles
- draft submission packets
- block final submission/posting/wallet actions behind human gates

## Non-Goals

- Do not replace `cynic-kernel` with Hermes Agent memory.
- Do not make cloud Browserbase or Browser Use the default path.
- Do not expose submit/post/sign/trade/stake/deposit tools through MCP by
  default.
- Do not write browser cookies, raw authenticated HTML, or personal account data
  into Git-tracked files.

## Done Criteria

The bootstrap is real when:

- `organ-browser` has at least one Hermes-compatible local backend adapter
- browser evidence includes accessibility snapshots or CDP event summaries
- policy can list which Hermes-compatible tools are enabled/blocked
- MCP exposes only policy-approved tools
- Superteam opportunity tasks can run browser/RAG extraction through the organ
  without direct cortex browsing hacks
