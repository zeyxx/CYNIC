# MCP-0 Remote Agent Convergence

## Scope

MCP-0 is the first supervised remote-agent project for CYNIC Cloud. It
converges the current MCP artifact contract; it does not grant an agent the
authority to rebuild, promote, or self-authorize a general agent runtime.

## Observed baseline

- `cynic-agent-host` has an isolated CYNIC source snapshot with no Git remote.
- Hermes has no registered MCP server on the node.
- `crates/cynic-mcp` is absent from the workspace.
- the current artifact observation reports a historical incompatibility:
  contract revision 6 observed; revision 8 required.
- the node has an MCP runtime observer that produces a proof bundle, but the
  WebSocket federation client is not wired into the node runtime.
- agent session action/token ceilings are declared defaults, not empirical
  budgets.

## Desired state

A node-local `cynic-mcp` artifact proves, before Hermes consumes it:

1. contract JSON matches `infra/agent-exposure.json`;
2. stdio `initialize` and `tools/list` work;
3. the read-only bootstrap tools are present:
   `cynic_runtime_surface`, `cynic_coord_register`, `cynic_coord_who`;
4. `cynic-node` emits a satisfied MCP compatibility proof from that actual
   artifact.

## Authority

The node owns observation, worktree allocation, local admission, execution,
and retained completion evidence. A kernel-mediated WebSocket is a bootstrap
coordinator only: its outage must not prevent local observation, completion of
an already-admitted bounded workload, or later replay of retained proof.

## Baseline before budget policy

Each experiment records node/worktree/provider/model identity, declared
experiment ceiling, elapsed time, tool calls requested/executed/failed,
command outcome, token/cost when available, source and validation read-back,
human interventions, and explicit unknowns. These data are descriptive; they
are not enforcement policy.

## Stages

- M0: read-only inventory of desired contract, source gap, installed artifact,
  and test matrix.
- M1: pure compatibility fixture/tests for compatible, incompatible,
  unavailable, and invalid artifacts.
- M2: minimal source-owned stdio MCP projection, reusing existing protocol and
  session owners.
- M3: node-local artifact observation, compatibility read-back, then Hermes
  MCP registration.
- M4: one `mutates=false` observer workload delivered through node transport,
  including partition/reconnect completion replay proof.

Every stage requires separate human admission. Until M3 is proven, local
read-only CLI is the fallback. No M0-M4 step authorizes runtime, credential,
network, fleet, release, merge, push, or artifact-promotion mutation.
