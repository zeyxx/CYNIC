# Hermes-CYNIC Sovereign Bridge Specification

**Document Status**: Implementation Baseline (v1.0)
**Scope**: Zero-Coupling MCP Bridge, CLI Projections, and Specialized Hermes Skills
**Authority**: Human Operator (Constituent Power)
**Language Invariant**: 100% English (code, schemas, docstrings, commits)

---

## 1. Architectural Philosophy: Inversion of Control & Zero Coupling

Hermes-Agent is the operator's primary daily interface (TUI/CLI, subagent orchestration, tool loop). CYNIC is the sovereign epistemic kernel (Popperian falsification, immutable memory, sandbox restoration, canonical data center).

**Core Principle**: *Neither system imports the other's code. Communication is strictly out-of-process via standard JSON-RPC 2.0 (Model Context Protocol / stdio) and standard CLI execution.*

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                 HERMES-AGENT (Host / TUI)                              │
│                                                                                        │
│  • Primary conversational loop with operator                                           │
│  • Tool executor (`terminal`, `read_file`, `patch`, `execute_code`)                    │
│  • Subagent spawner (`delegate_task`)                                                  │
│  • Local SQLite database (`~/.hermes/state.db`)                                        │
│  • Special Skills: `cynic-epistemic-protocol`, `cynic-memory-recall`, etc.            │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                       MCP JSON-RPC Stdio   │   Out-of-process, typed contracts,
                       (cynic-mcp binary)   │   zero shared libraries
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              CYNIC-MCP (Gateway Projection)                            │
│                                                                                        │
│  • `cynic_epistemic_probe` : Popperian hypothesis falsification test                   │
│  • `cynic_memory_query`    : Immutable Fact Registry and winning subgraph lookup       │
│  • `cynic_sandbox_run`     : Hermetic execution with auto-snapshot & rollback          │
│  • `cynic_data_observe`    : Streams verified execution receipts to Data Center        │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            │   Direct in-process Rust / CLI calls
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                         CYNIC CORE & RUNTIME (Sovereign Engine)                        │
│                                                                                        │
│  • `cynic epistemic` : Null hypothesis generation, destructive probe runner            │
│  • `cynic memory`    : Multi-scale memory & Fact Registry (SurrealKV / SQLite)         │
│  • `cynic harness`   : Hermetic sandbox (`bwrap`) with zero-copy snapshot/rollback    │
│  • `cynic data`      : Frozen corpus processing (Capture -> Canonical -> Model)       │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. MCP Tool Interface Specifications

The `cynic-mcp` binary exposes 4 dedicated tools to Hermes-Agent.

### 2.1 Tool: `cynic_epistemic_probe`

Submits an empirical claim or hypothesis to Popperian falsification testing. The engine formulates the null hypothesis $H_0$, runs destructive physical checks, and returns a verified verdict.

```json
{
  "name": "cynic_epistemic_probe",
  "description": "Subject an empirical claim or design hypothesis to Popperian falsification against the frozen corpus or live environment. Returns a strict REFUTED or UNREFUTED verdict backed by an immutable physical machine receipt.",
  "parameters": {
    "type": "object",
    "properties": {
      "hypothesis": {
        "type": "string",
        "description": "The empirical claim to test (e.g. 'all 1079 crons are passive one-shot monitors')."
      },
      "null_hypothesis": {
        "type": "string",
        "description": "Optional explicit H0 formulation designed to disprove the claim."
      },
      "probe_scope": {
        "type": "string",
        "enum": ["frozen_corpus", "live_worktree", "git_history"],
        "default": "frozen_corpus",
        "description": "Target environment for the probe."
      },
      "destructive_query": {
        "type": "string",
        "description": "Optional SQL or shell script probe designed to search for counter-examples."
      }
    },
    "required": ["hypothesis"]
  }
}
```

**Output Schema**:
```json
{
  "verdict": "REFUTED",
  "evidence_summary": "Found 320 code mutations (patch, write_file) inside cron sessions. 79.4% of crons have between 10 and 122 messages.",
  "receipt": {
    "sha256": "4b82d9...",
    "exit_code": 0,
    "rows_counter_evidence": 320,
    "target_dataset": "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"
  },
  "facts_recorded": [
    {
      "id": "fact_cron_mutations_observed",
      "statement": "Cron sessions in hermes_union.sqlite executed 320 real code mutations.",
      "grade": "L0_physical"
    }
  ]
}
```

### 2.2 Tool: `cynic_memory_query`

Queries the immutable Fact Registry and winning trajectory subgraphs before embarking on non-trivial refactorings or designs.

```json
{
  "name": "cynic_memory_query",
  "description": "Retrieve verified facts, winning trajectory subgraphs, or historical failure precedents from CYNIC's immutable memory registry.",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Keyword or semantic description of the problem or task."
      },
      "mode": {
        "type": "string",
        "enum": ["fts_lexical", "semantic_graph", "hybrid"],
        "default": "hybrid",
        "description": "Retrieval mode: lexical exact for symbols/errors, semantic for concepts."
      },
      "limit": {
        "type": "integer",
        "default": 5,
        "description": "Maximum number of verified precedents to return."
      },
      "min_grade": {
        "type": "string",
        "enum": ["L0_physical", "L1_curated", "L2_provisional"],
        "default": "L0_physical",
        "description": "Minimum verification grade required."
      }
    },
    "required": ["query"]
  }
}
```

### 2.3 Tool: `cynic_sandbox_run`

Executes high-risk commands (destructive file edits, compiler checks, migrations) in an isolated sandbox with automated pre-action snapshot and instant rollback on regression.

```json
{
  "name": "cynic_sandbox_run",
  "description": "Execute a shell command within a zero-cost snapshot sandbox. If the command fails, exits non-zero, or corrupts repository invariants, an automatic rollback is performed.",
  "parameters": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string",
        "description": "The exact shell command to execute."
      },
      "timeout_seconds": {
        "type": "integer",
        "default": 120,
        "description": "Execution timeout limit."
      },
      "snapshot_mode": {
        "type": "string",
        "enum": ["git_worktree_ephemeral", "bwrap_overlay"],
        "default": "git_worktree_ephemeral",
        "description": "Snapshot mechanism."
      },
      "rollback_on_failure": {
        "type": "boolean",
        "default": true,
        "description": "Whether to instantly restore previous state if exit code != 0."
      }
    },
    "required": ["command"]
  }
}
```

### 2.4 Tool: `cynic_data_observe`

Allows Hermes (or a subagent) to stream an execution event, test result, or human steering signal directly to CYNIC's Canonical Data Center.

```json
{
  "name": "cynic_data_observe",
  "description": "Stream an event or turn outcome to the CYNIC Data Center, anchoring it to the active repository Git commit SHA and session graph.",
  "parameters": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string" },
      "turn_index": { "type": "integer" },
      "event_type": {
        "type": "string",
        "enum": ["tool_success", "tool_failure", "human_steer", "host_fault", "milestone_commit"]
      },
      "receipt_data": {
        "type": "object",
        "description": "Arbitrary typed JSON payload conforming to cynic.canonical.event.v1."
      }
    },
    "required": ["session_id", "turn_index", "event_type", "receipt_data"]
  }
}
```

---

## 3. CYNIC CLI Native Commands Specification

The MCP gateway is a thin projection of native CLI commands implemented in `crates/cynic-cli`:

| Family | Subcommand | Purpose | Live Read-back Output |
|---|---|---|---|
| `cynic epistemic` | `test --hypothesis "..."` | Executes falsification against frozen corpus or worktree | `{"verdict": "REFUTED", "receipt": {...}}` |
| `cynic epistemic` | `registry list` | Displays verified facts with machine receipts | Table / JSON list of facts and SHA-256s |
| `cynic memory` | `query --query "..."` | Searches Fact Registry and winning subgraphs | Matches with relevance score and proof link |
| `cynic memory` | `record --statement "..."` | Ingests an operator-verified fact into memory | `{"fact_id": "...", "status": "recorded"}` |
| `cynic harness` | `sandbox --cmd "..."` | Runs command under ephemeral worktree/bwrap | `{"exit_code": 0, "rolled_back": false}` |
| `cynic data` | `canonical --source hermes` | Normalizes SQLite captures into canonical store | Progress stream + validation event count |
| `cynic data` | `project --target sft` | Exports verified winning subgraphs to Parquet | File path, sample count, loss-mask metrics |

---

## 4. Specialized Hermes Skills Specifications

Three specialized skills are deployed into `~/.hermes/skills/cynic/`:

### 4.1 Skill: `cynic-epistemic-protocol`
* **Trigger**: *"Use when facing an unverified empirical assumption, an apparently clean surface-level finding, or an architectural hypothesis."*
* **Core Rule**:
  1. Never assert a factual property about the corpus or system without a machine receipt.
  2. If tempted to state a generalization (e.g. *"all crons are monitors"*, *"4 hours is the session pause"*), **stop immediately**.
  3. Formulate the null hypothesis $H_0$ and invoke `cynic_epistemic_probe`.
  4. Only proceed after receiving a formal `UNREFUTED` verdict.

### 4.2 Skill: `cynic-memory-recall`
* **Trigger**: *"Use at the beginning of any design session, architectural review, or complex debugging task."*
* **Core Rule**:
  1. Invoke `cynic_memory_query` with the key domain keywords.
  2. Check if a structurally identical bug or design trade-off was previously arbitrated.
  3. Present verified historical facts to the operator before proposing mutations.

### 4.3 Skill: `cynic-subagent-coordination`
* **Trigger**: *"Use when spawning subagents via delegate_task or coordinating parallel workstreams."*
* **Core Rule**:
  1. Pass the parent `session_id` and active Git commit SHA in the subagent context.
  2. Subagents must emit their milestone outputs via `cynic_data_observe`.
  3. The parent agent verifies subagent returns with physical oracles before adopting them.

---

## 5. Hermes-Agent Configuration Wiring

In Hermes's configuration (`~/.hermes/config.yaml`), `cynic-mcp` is registered as a local stdio MCP server:

```yaml
mcp_servers:
  cynic:
    command: "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/target/release/cynic-mcp"
    args: ["serve"]
    env:
      CYNIC_WORKSPACE_ROOT: "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"
      CYNIC_CORPUS_DB: "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"
```

**Graceful Degradation Guarantee**:
If `cynic-mcp` is absent, unbuilt, or disabled, Hermes-Agent logs a single non-fatal warning on startup and falls back to standard shell tools without interrupting operator interaction.
