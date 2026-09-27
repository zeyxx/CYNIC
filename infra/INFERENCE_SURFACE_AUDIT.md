# Inference Surface Audit — declared vs implemented vs observable

- **schema**: `cynic.audit.inference_surface.v1`
- **status**: `read_only_audit`
- **method**: observation only. No file mutated during the audit. Coordination was available and a single file claim was taken to write this report.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` > `infra/runtime-authority.json` > `infra/registry.json` > workspace `Cargo.toml` > canonical crates > `infra/agent-exposure.json`

## 0. Why this audit exists

The operator asked for a working local inference path exposed on the CLI. The audit was to
establish why the surface is currently a black box before writing any code. The answer is
not fragmentation. It is that the declared surfaces are largely **unreachable**, and the
mechanisms that should detect that **do not compile**.

### 0.1 One root cause, two symptoms

`RuntimeCommand` is a complete 44-variant enum with a full dispatcher. It is **not wired into
the clap root enum**, so no `cynic runtime …` string can reach it. This single omission
simultaneously explains:

- the inference black box (`cynic runtime inference-*`: 15 implemented, 0 reachable), and
- the missing storage triage (`cynic runtime disk` / `disk-clean`: implemented, dispatched,
  declared as canonical entrypoints in `infra/surface-registry.json`, still unreachable).

91 of 165 advertised CLI commands are in this state.

## 1. Observed runtime state (the data-centric baseline)

Observed on the operator host, 2026-09-27.

| Signal | Observed value |
|---|---|
| `llama-server` | `/usr/local/bin/llama-server`, present, `--list-devices` works |
| Vulkan devices | `AMD Radeon Graphics (RADV RENOIR)`, 22265 MiB shared — **integrated, no discrete GPU** |
| RAM | 27 GiB total, 17 GiB available |
| Disk `/` | 350 G total, 325 G used, **7.5 G free (98%)** |
| Backends declared in `/etc/cynic/inference.toml` | **13**, ports 8080–8091 |
| Backends actually listening | **0** (only 127.0.0.1:8000 surreal is bound) |
| `.gguf` files in `~/.config/cynic/models` | **0** — the vault directory is empty |
| HF cache | 1 partial blob, 15.9 MB, `.incomplete` |
| HF auth | authenticated (`hf auth whoami` succeeds) |
| `CYNIC_*` inference env vars | none set |
| `target/debug/cynic` mtime | 2026-09-17 18:58 — **9 days older than `cli_args.rs` (2026-09-26 12:25)** |

### 1.1 Consequences

1. **The SSOT is a wish-list, not observed state.** 13 declared backends, 0 alive. Under the
   authority order, observed state outranks `inference.toml`; the file is currently fiction.
2. **The stale binary is a success-shaped artifact.** `target/debug/cynic` still accepts
   `cynic runtime inference-status` that the source can no longer parse. An operator invoking
   it receives a plausible report for a surface that does not exist in HEAD.
3. **Model identity is unprovable by construction.** `inference.toml:6` stores
   `model = "Qwen3.5-2B-Q4_K_M"` — a guessed *filename*, with no `repo_id`, no `revision`, no
   digest. The real repository is `unsloth/Qwen3.5-2B-GGUF`. Nothing in the system can verify
   which artifact is loaded. This is the definition of a black box.

### 1.2 Measured model sizes (contradicts a plausible plan)

`hf download prism-ml/Ternary-Bonsai-2-27B-gguf --dry-run` → **68.5 G across 11 files**.

| File | Size |
|---|---|
| `Ternary-Bonsai-2-27B-F16.gguf` | 53.8 G |
| `Ternary-Bonsai-2-27B-PQ2_0.gguf` | 7.2 G |
| `Ternary-Bonsai-2-27B-PTQ1_0.gguf` | 5.9 G |
| `Ternary-Bonsai-2-27B-mmproj-BF16.gguf` | 931 M |
| `Ternary-Bonsai-2-27B-mmproj-Q8_0.gguf` | 629 M |

The smallest usable artifact is 5.9 G against 7.5 G free, leaving ~1.6 G of headroom on a
filesystem already at 98 %. **Not viable.** A 2B Q4_K_M (~1.6 G) fits and is the artifact the
SSOT already names.

## 2. Structural blockers

### X1 — `cynic runtime` is not a clap subcommand

`crates/cynic-cli/src/cli_args.rs:31-191` defines 25 top-level `Command` variants. **None is
`Runtime`.** `RuntimeCommand` (`crates/cynic-cli/src/runtime_surface.rs:38`) is dispatched by
`execute_runtime` (`runtime_surface.rs:529`), whose only callers are
`crates/cynic-cli/src/mask_surface.rs:80,91,143`.

All 16 `inference-*` variants are real implementations:

| Variant | Declared | Dispatcher | argv-reachable |
|---|---|---|---|
| `Inference` | `runtime_surface.rs:201` | `inference_dispatch.rs:7` | no |
| `InferenceVault` | `:208` | `:8` | no |
| `InferenceStatus` | `:216` | `:11` | no |
| `InferenceStart` | `:223` | `:14` | only via `cynic mask` |
| `InferenceStop` | `:238` | `:25` | only via `cynic mask` |
| `InferenceForge` | `:243` | `:103` | only via `cynic mask` |
| `InferenceDiscover` | `:250` | `:28` | no |
| `InferenceModels` | `:261` | `:35` | no |
| `InferenceModelsConsolidate` | `:268` | `:44` | no |
| `InferenceHfStatus` | `:272` | `:47` | no |
| `InferenceHfDownload` | `:277` | `:50` | no |
| `InferenceHfCategories` | `:301` | `:74` | no |
| `InferenceHardware` | `:305` | `:77` | no |
| `InferenceHfSearch` | `:309` | `:80` | no |
| `InferenceHfSuggest` | `:322` | `:93` | no |
| `InferencePlan` | `:330` | `:100` | no |

`inference_dispatch.rs:103-110` also silently drops `alias` for `InferenceForge`, forwarding a
hardcoded `""` as `source_note`.

### X2 — `cynic-mcp` compiles 9 of 88 authored tools

`crates/cynic-mcp/src/lib.rs:8` declares `pub mod server;` — **that is the only module
declaration in the crate.** `proxy.rs`, `handler.rs`, `routers/`, `helpers_split/`,
`types_split/`, `local_tools.rs` are outside the compilation graph.

`infra/agent-exposure.json` declares 109 tools. **9 are servable.** Both inference tools
(`cynic_inference_backends`, `cynic_inference_discover`) are authored but not compiled.

`crates/cynic-mcp/Cargo.toml` has no `[features]` table, so
`#[cfg_attr(feature = "legacy_infra_tools", tool(...))]` on 4 proxmox tools
(`routers/local.rs:1313,1419,1474,1535`) is permanently inert.

### X3 — `cynic-kernel` does not compile

`crates/cynic-kernel/src/api/mcp/mod.rs:26` declares `mod observe_tools;`. **The file does not
exist.** `mod.rs:345` calls `Self::tool_router_observe()`. `judge_tools.rs` (12 `#[tool]`,
including a second `cynic_infer`) is an undeclared orphan.

## 3. Why the divergence is invisible

`crates/cynic-cli/tests/architecture_boundary.rs` contains **four committed merge conflicts**:

```
3:<<<<<<< HEAD     8:=======    15:>>>>>>> github/main
22:<<<<<<< HEAD    30:=======   56:>>>>>>> github/main
61:<<<<<<< HEAD    69:=======   1252:>>>>>>> github/main
1258:<<<<<<< HEAD  1270:======= 1527:>>>>>>> github/main
```

`rustc` reports unclosed delimiters. The file originates from commit `983b63b6`
("merge(github/main)"). **None of its ~35 tests has executed since.**

This is structurally invisible because the canonical gates never run tests:

- `Makefile:67-76` (`check`) and `Makefile:77-86` (`audit`) execute a **prebuilt binary** and
  invoke `cynic audit crate --json`. Neither calls `cargo test`.
- `Makefile:106-115` (`check-layers`) is the only target running `cargo test`, and it is not
  in the `AGENTS.md` canonical gate list.

Even if it compiled, the reconciliation test would fail. `command_manifest.rs:314`
`manifest_domains_match_clap_surface` compares only top-level domain names, with a hardcoded
skip list (`commands`, `advanced`) and allowlist (`models`, `doctor`, `errors`, `status`,
`version`). Current divergence:

- visible clap subcommands with no manifest domain: `audit`
- manifest domains with no clap subcommand: `fleet`, `runtime`, `dev`, `bootstrap`, `deps`,
  `lifecycle`

**No test in the repository can detect a command declared in `runtime_surface.rs` but absent
from `cli_args.rs`.** `commands_manifest_exposes_canonical_surface_boundaries`
(`architecture_boundary.rs:1157`) reads the generated `canonical_surfaces` and the hand-written
`domains`, then asserts only `surfaces.len() >= domains.len()` — a cardinality inequality
between 45 and 24 entries, structurally incapable of detecting drift.

## 4. The manifest is Rust prose, not a parseable contract

`crates/cynic-cli/src/command_manifest.rs` is hand-written. Its command strings are usage prose
with hand-copied flags:

```
:92  "cynic runtime inference-hf-download <repo> --json [--alias NAME] [--include <glob>]... [--no-profile] [--cache-only]"
```

`"cynic runtime deps [--json]  # alias of cynic deps"` is not parseable argv. **This is the
root cause of the drift: the manifest stores argv as free text, so there is nothing to compare
against the real tree.** 165 advertised leaf commands are never parsed by any test.

`crates/cynic-cli/build.rs` (424 LOC) generates **data, not code**: it embeds 45 JSON fragments
from `infra/surface-registry/` into `$OUT_DIR/surface_registry_fragment_index.rs`. No
`clap::Command` is ever generated. ~60% of the file is TOCTOU-hardened I/O (`openat`,
`NOFOLLOW`, `O_EXCL`+`renameat`), not codegen.

### 4.1 Declared-vs-reachable roll-up

| Domain | Advertised | clap root | Reachable | Unreachable |
|---|---|---|---|---|
| operator | 10 | n/a | 10 | 0 |
| **runtime** | **37** | **no** | 0 | **37** |
| **bootstrap** | **22** | **no** | 0 | **22** |
| **dev** | **12** | **no** | 0 | **12** |
| **lifecycle** | **9** | **no** | 0 | **9** |
| **fleet** | **3** | **no** | 0 | **3** |
| **deps** | **3** | **no** | 0 | **3** |
| **forensics** | **5** | yes | 0 | **5** (zero overlap with clap variants) |
| network, agent, node, source, git, judge, discovery, session, update, task, federation, mask, queue, court, build, chat | 61 | yes | 61 | 0 |
| **TOTAL** | **165** | | **74** | **91** |

Thirteen further CLI source files are orphans absent from `lib.rs`'s module list, masked by
`crates/cynic-cli/src/lib.rs:1` `#![allow(..., dead_code)]`.

## 5. MCP inference tool contract

`infra/agent-exposure.json:1026-1075` declares 7 `inference_tools`.

| Tool | line | kind | ssot | cli_equivalent | status |
|---|---|---|---|---|---|
| `cynic_inference_backends` | 1028 | `local_read` | `/etc/cynic/inference.toml` | `cynic runtime inference --json` | authored, **not compiled** |
| `cynic_inference_discover` | 1035 | `local_mutation` | `/etc/cynic/inference.toml` | `cynic runtime inference-discover --json --apply` | authored, **not compiled** |
| `cynic_inference_vault` | 1042 | `local_read` | `~/.config/cynic/models` | `cynic runtime inference-vault --json` | **no MCP tool**; CLI reachable as `cynic models list` |
| `cynic_inference_start` | 1049 | `local_mutation` | `~/.config/cynic/models` | `cynic runtime inference-start <alias> --yes --json` | **no MCP tool**; CLI `cynic models start` reachable |
| `cynic_inference_stop` | 1056 | `local_mutation` | `~/.config/cynic/models` | `cynic runtime inference-stop --yes --json` | **no MCP tool**; CLI `cynic models stop` reachable |
| `cynic_inference_switch` | 1063 | `local_mutation` | `~/.config/cynic/models` | `cynic runtime inference-start <target_alias> --yes --json` | **no MCP tool, no CLI** |
| `cynic_operational_benchmark` | 1070 | `read_only` | `infra/operational-benchmarks.json` | `cynic runtime inference-benchmark --json` | **no MCP tool, no CLI, no code**; `crates/cynic-cli/src/inference_benchmark.rs` (731 LOC) is an undeclared orphan |

5 of 7 have zero implementation. 2 are authored but uncompilable. **0 are servable.**

### 5.1 Ungated SSOT mutation

`crates/cynic-mcp/src/routers/local.rs:1847-1865`:

```rust
async fn cynic_inference_discover(&self) -> Result<CallToolResult, McpError> {
    self.require_auth()?;
    self.rate_limit.check_other()?;
    let report = cynic_inference::peer_attach(cynic_inference::PeerAttachOptions {
        apply: true,          // :1856 — hardcoded
        open_webui: false,
        node_id: "local-node".into(),
    }).await;
```

Zero `Parameters<...>`: the agent cannot pass `apply`, `dry_run`, or `approve`. The write path is
`ods_discovery.rs:240-258` → `std::fs::write` on the resolved `inference.toml`
(`config_loader.rs:141-168`). **No read-back** re-parses or digests the file; `config_updated`
is a self-reported intent boolean.

This diverges from the CLI, which requires explicit `--apply`
(`inference_dispatch.rs:28-34`). Same operation, two consent policies by surface. Additionally
`ods_discovery.rs:231` writes discovered peers with `sovereign: true` and
`auth_style: AuthStyle::None` — a self-granting sovereignty flag.

## 6. Success-shaped reports for non-converged state

| id | location | non-converged state reported as |
|---|---|---|
| S1 | `cynic-inference/src/adapters/local/process_supervisor.rs:527` | `--no-wait` hardcodes `ready = true`; `ok: true`, `ExitCode::SUCCESS`, no probe ever runs |
| S2 | `process_supervisor.rs:121,125-127` | `state: Active` + `endpoint` derived from pidfile/config; **no HTTP, no `/v1/models`** |
| S3 | `process_supervisor.rs:274` | bidirectional `contains()` model match; a wrong model yields `ready=true` |
| S4 | `cynic-inference/src/adapters/inference/openai_compat.rs:234-255` | model identity read then discarded; mismatch is `tracing::warn!` but returns `BackendStatus::Healthy` |
| S5 | `process_supervisor.rs:600-612` | `stop()` on a non-running alias returns `ok: true`, `"process stopped"`, `SUCCESS` |
| S6 | `process_supervisor.rs:302-304` | missing `curl` is indistinguishable from "not ready"; burns the full 180–1200 s timeout |
| S7 | `cynic-cli/src/runtime_surface/inference_surface.rs:132,140` | absent `inference.toml` (`status: "not_found"`) with `ExitCode::SUCCESS` |
| S8 | `runtime_surface/inference_surface/local_runtime.rs:319-322` | `peer_attach` finding zero peers (`status == "empty"`) with `ExitCode::SUCCESS` |
| S9 | `cynic-mcp/src/routers/local.rs:1834` | `"status": "ok"` hardcoded while `sovereign_healthy_count` may be 0 |
| S10 | `cynic-cli/src/lib.rs:1` | crate-wide `dead_code` allow masks 13 orphan modules |
| S11 | `cynic-cli/src/command_manifest.rs:315` | the drift guard exists, is red, and cannot execute |
| S12 | `cynic-mcp/Cargo.toml` | no `[features]`, so 4 `#[tool]`s are permanently inert |
| S13 | `cynic-kernel/src/api/mcp/mod.rs:26` | `mod observe_tools;` → missing file |
| S14 | `cynic-mcp/src/lib.rs:8` | 74 authored `#[tool]`s outside the build graph |
| S15 | `cynic-inference-runtime/src/lib.rs` | doc-comment-only crate, 0 dependents, passes `make check` |
| S16 | `cynic-mcp/src/routers/local.rs:1856` | SSOT mutation with no consent parameter |

S1 is the sharpest: the message field contains both `ready=true` and `ready_label = "deferred"`
(`process_supervisor.rs:563,578`). A machine parser and a human reader draw opposite conclusions
from the same field.

### 6.1 The tool loop is a two-tool stub

`cynic-inference/src/application/chat_service.rs` has a real port: `ToolInvoker`
(`:42-45`), injected at `:191-213`, invoked at `:347`. The domain is correctly pluggable.

But `BuiltinToolInvoker` (`:618-640`) matches only `echo` and `get_time`, and
`default_tool_specs()` (`:642-659`) advertises exactly those two. All four production call sites
hardcode it (`chat_surface/command.rs:128`, `chat_surface/runtime.rs:116,268,838`). Tool
advertising is off by default and requires `CYNIC_CHAT_TOOLS=1`
(`chat_surface/runtime.rs:807-817`).

### 6.2 The phantom crate

`crates/cynic-inference-runtime/`: 3 files, 28 LOC. `src/lib.rs` is seven doc-comment lines,
zero `pub` items, zero `mod`, zero impls. **Zero crates depend on it** — the only reference in
the workspace is `Cargo.toml:9` listing it as a member. It declares `reqwest`, `tokio`, `anyhow`,
`ulid` it never uses, and pins `thiserror = "1"` against the workspace's `2`. It is a green
crate in `make check` for a non-existent surface.

## 7. Canonical CLI architecture, from external study

The question "should this be schema-driven?" was settled by studying comparable systems.

### 7.1 What comparable systems actually do

| System | Mechanism | Reconciles docs against code? |
|---|---|---|
| **git** | `command-list.txt` at repo root: `command-name category [category]…`, plus `deprecated`. Reconciled by `Documentation/cmd-list.sh` (`exit 1` on missing man page / section / description / mismatch) and `lint-docs-meson` (`cmp` + `diff -u` + `exit 1`) | **Yes, bidirectionally, fails the build** |
| kubectl | Go source is the sole truth; docs generated by `gen-kubectldocs` via AST parsing (`gengo`), not runtime reflection. `kubectl explain` fetches OpenAPI from the apiserver — a *second* source | Regenerate-and-diff only; two doc sources coexist |
| cobra | Live `*cobra.Command` tree walk generates markdown; documented failure mode: *"may not work correctly if your command names have `-` in them"* | Generated, not audited |
| `clap_complete` / `clap_mangen` / `clap-markdown` | code → artifacts. `clap-markdown` docs say *"Save CommandLineHelp.md in git"* — no drift check | No |
| `bpaf` | `docgen` renders from the live `OptionParser`; help text **is** the source | No second artifact exists |
| cargo | `cargo --list` is a flat human-readable list, not machine-readable, not recursive. Reference docs are **hand-written** markdown | No |
| PowerShell | Closed approved-verb list with an explicit "synonyms to avoid" column; runtime rejects unapproved verbs | Machine-checkable governance |
| gcloud | Two-level groups; CRUD verbs per resource; **"Conventions are goals more than rules"**; parallel namespaces by stability (`alpha`/`beta`/ga) | Convention, not enforcement |

### 7.2 Negative results worth recording

- **`clap_schema` is exactly this idea, and it died.** v0.0.0, deprecated in favour of `argx`,
  no README. Treat as post-mortem.
- **No crate audits a clap command tree against a declared manifest.** `cargo --list` is flat
  and human-readable. There is no `cargo --list --json`.
- **The well-known `gum` is Go, not Rust.** Do not design against a Rust `gum`.
- **No published CLI surface-area limit or maximum nesting depth exists.** The de facto pattern
  is gcloud's two levels and git's deliberate flatness. Anyone quoting an "industry standard of
  N top-level commands" is inventing it.
- **No Nielsen Norman Group article on CLI command namespaces** was located. Do not accept such
  a citation.

### 7.3 The design lessons that transfer

1. **`argv` must be an array of strings, never a string.** git stores `git-add mainporcelain
   worktree` and **never** stores flags. Syntax lives in the code; the registry stores
   identity, category, and lifecycle only. This is the single most important correction, and it
   comes directly from the failure in §4.
2. **`clap::Command::get_subcommands()` is complete argv reflection** — already used in this
   repo's own test. No new crate is needed.
3. **git solves surface size with category, not nesting.** Its command tree is deliberately
   flat. The current `Advanced` escape hatch (`cli_args.rs:62`) is the right instinct and is
   underused; it should become load-bearing.
4. **Deprecation belongs in the registry, not the code.** git emits `(deprecated)` from the
   attribute while the route keeps working. This repo has no deprecation mechanism at all.
5. **`cynic status` re-exposes `doctor`/`models`/`errors`/`version`** (`cli_args.rs:194-215`),
   creating a second argv route to the same identity. The registry must express this
   (`aliases_of`) or the reverse check will false-positive permanently.
6. **Severity default decides fail vs warn.** `dependency-cruiser` warns by default and says so
   in its own tutorial. Spectral defaults to `error`. ArchUnit's violation-freeze store prevents
   grandfathering new violations. `ts-archunit` stages new rules at `warn` for a release, then
   promotes to `error` — a ratchet worth stealing.
7. **Do not baseline the 91 failures.** Advertising a command with no argv route is strictly
   worse than not advertising it. That is the bug, not debt to be grandfathered. A ratchet
   baseline preserves the lie.

### 7.4 Proposed mechanism (minimum)

Keep argv as hand-written clap code. Add a declared registry, reconciled by an audit that
**fails**.

```json
{
  "schema": "cynic.surface_registry.commands.v1",
  "commands": [
    { "argv": ["runtime", "inference-status"], "domain": "runtime", "owner_crate": "cynic-inference",
      "mutating": false, "visibility": "advanced", "aliases_of": null, "deprecated": null }
  ]
}
```

Audit as a pure function with three hard errors, each in both directions:

- `declared_not_routable` — every registered `argv` path resolves in the reflected clap tree.
  *(Catches the current 91.)*
- `routable_not_declared` — every non-hidden clap leaf is registered. *(Replaces the hardcoded
  allowlist.)*
- `consent_required` — every `mutating: true` entry must require `--yes` or `--apply` in the
  real clap command. *(New value: makes the constitutional consent invariant executable.
  Worth more than documentation accuracy.)*

Entry point `cynic audit surface-registry --json` returning non-zero, wired into `make check`,
plus a plain `#[test]` so `cargo test` catches it independently. Host placement reuses the
existing `infra/surface-registry/manifest.json` + fragment + `build.rs` pipeline.

### 7.5 Gaps in the authority itself

- `infra/surface-registry.json` **is not ranked in any `authority_order`**. Neither
  `contracts/foundation.json:13-21` nor `infra/runtime-authority.json:5-14` mentions it — yet it
  is where the real ownership decision for disk management lives.
- `infra/runtime-authority.json` contains **no** capacity, cleanup, retention, GC, quota, or
  cache entry. `host_substrate` (`:28-38`) says only `"filesystem projection"`.
- `cynic-inference` is flagged `experimental_workspace_crates`, `status: "extracting"`,
  `not_yet: ["canonical runtime authority surface", "make check core package gate", "agent
  exposure domain ownership"]` (`runtime-authority.json:193-201`).
- `infra/ARCHITECTURE_CONSTELLATIONS.md:110` declares
  `cynic-inference` uses `features = ["local-llama", "remote-api"]`. **`cynic-inference/Cargo.toml`
  has no `[features]` table.**

## 8. Storage triage ownership — decided by existing authority

The operator asked whether storage triage belongs to `cynic-storage` or `cynic-host`.

**`infra/surface-registry.json#runtime_disk_management` already decides it:**

```json
"owner_crate": "cynic-host",
"layer": "l1_host_apply_boundary",
"domain_owner": "cynic-host::domain::disk",
"owner_modules": ["domain::disk", "domain::disk::build_runner",
                  "adapters::local_disk", "adapters::local_build_cache"],
"mutation_policy": "explicit_apply_only",
"canonical_entrypoints": ["cynic runtime disk [--json]",
                          "cynic runtime disk-clean [--json] [--apply]", …],
"anti_responsibilities": ["silent auto-cleanup without --apply",
                          "delete user-created repositories or non-cache home files",
                          "reclaim a cache profile not declared by registry policy",
                          "follow symlinks or escape the injected state root",
                          "mutate host networking or service configuration"]
```

Corroborated by `infra/verticals/storage/intent.json` (`"owner_crate": "cynic-host"`,
`"layer": "l1_host"`). Canonical types live in `cynic-protocol::host_substrate::disk`
(`cynic-host/src/domain/disk/types.rs:1-11` is a re-export with that rationale).

**The capability already exists, is already dispatched, and is blocked by X1:**
`RuntimeCommand::Disk` (`runtime_surface.rs:397-401`) and `DiskClean` (`:417-430`), dispatched
by `runtime_surface/disk.rs:15,31`, implemented by `LocalDiskAdapter`
(`cynic-host/src/adapters/local_disk.rs`, 493 LOC) behind `DiskObservationPort` and
`DiskCleanupExecutionPort` (`cynic-host/src/domain/disk/port.rs`, 16 LOC).

### 8.1 Not `cynic-storage`

`cynic-storage` is a 4 077 LOC WAL and embedded-projection engine
(`rusqlite`, `WalFsAdapter`, `SqliteProjectionAdapter`). It has **no** capacity, usage,
cleanup, or retention concept — a grep for
`retention|gc|garbage|prune|vacuum|cleanup|sweep|reap|consolidate|expire|ttl` over
`cynic-storage/src` returns nothing. Its `StorageObservation`
(`src/lifecycle.rs:102-112`) is a provider-volume shape, and `StorageDecision`
(`:114-122`) is an **expansion** decision, never reclamation. It is also absent from
`infra/runtime-authority.json` entirely.

### 8.2 Not `cynic-host-probes`

That crate (1 151 LOC) does not probe disk — its only disk reference is prose, and it holds no
surface in `infra/runtime-authority.json`. `infra/CANONICAL_CRATE_MAP.md:341` reserves
`host_substrate/ (cpu/disk/memory)` for a future `cynic-host-substrate` built from
`cynic-protocol`. `contracts/foundation.json:36-38` requires host capabilities to be portable
ports before concrete adapters; `cynic-host-probes` has no port layer.

### 8.3 The capability is duplicated 16 times

Sixteen independent disk-usage computations exist across the workspace, including
`cynic-host/src/storage_census/growth.rs:7-31` (the only all-mounts **numeric** observer — and
it is a quarantined orphan per `cynic-host/tests/orphan_sources_ratchet.rs:22-51`),
`cynic-host/src/agent/cleanup.rs:30-49` (**hardcodes `ssh cynic-builder-ssh`**, a direct
violation of the dynamic-registry rule), and `cynic-host/src/space_gc/observe.rs:6-66`
(compiled, zero callers). A seventeenth must not be added.

### 8.4 The blocking defect for the operator's use case

`DiskFilesystem` (`cynic-protocol/src/host_substrate/disk.rs:11-19`) carries **only human
strings** — `size_human`, `used_human`, `avail_human`, `use_pct`. **No numeric bytes.** So
`cynic runtime disk --json` cannot express "98% full, 7.5 G free" to a machine, and cannot feed
a threshold decision. The numeric path exists only at
`cynic-host/src/adapters/local_build_cache.rs:132-142` (`sysinfo::Disk::available_space`).

This is an **additive field on an existing canonical protocol type**, not a new crate, and it
is the highest-leverage single fix in the storage surface.

### 8.5 Constraints on any reclamation

- `anti_responsibilities[1]` forbids *"delete user-created repositories or non-cache home
  files"*. The worktree is shared and carries other agents' `M`/`D`/`??` entries.
- `anti_responsibilities[4]` forbids following symlinks or escaping the injected state root.
- The only registry-declared reclaim policy is `infra/registry.json:403-412`
  (`canonical_cache_reclaim`, trigger `available_bytes_below_minimum`, `target_profile: debug`,
  `preserve_profiles: [release]`) — scoped to a `build_runner` node's cargo cache, not the
  operator root filesystem.
- `cynic-host/src/domain/disk/probe.rs:158-244` plans against six hardcoded cache labels
  (tmp-cynic-targets, journal-logs, rotated-syslogs, npm-cache, pip-cache, uv-cache). Anything
  unrecognised maps to `None` and is not cleaned.
- **Nothing in the repository authorizes unattended reclamation.** There is no scheduler and no
  daemon. "Automate" must mean registry-driven plan + explicit `--apply` + read-back, not a
  `du`-then-`rm` over `/home`.

## 9. Model vault retention — owner exists, policy does not

`infra/runtime-authority.json:92-106` (`inference_plane`, owner `crates/cynic-inference`) owns
`"model vault and layout under runtime principal"`. That is the only declaration naming a
vault-retention owner.

There is no policy. `cynic-inference/src/domain/model_retention.rs:30-73` **classifies only**
and hardcodes `deletion_authorized: false` at `:67` for every class. `assess_report` has zero
callers outside its own module. The only existing vault mutation is `vault_consolidate.rs`,
which **relocates** weights and already carries a capacity guard
(`vault_consolidate_capacity.rs:26-40`) returning `status="capacity_exceeded"` without mutating.

`infra/storage-layering.json:38-45` proposes LRU eviction for the L3 CAS plane, but that file is
`status: "proposal_v1"` and is not in any `authority_order`.

**Reclaiming vault space is currently unimplementable without a new authority decision.** That
decision belongs in `infra/registry.json` (a retention field on a `storage_class`, alongside the
existing `minimum_bytes`/`maximum_bytes`/`reclaim_policy`), per `no_contract_forest`
(`contracts/foundation.json:43-46`).

## 10. Bottom-up execution order

Derived state, not planned state. Each step produces proof the next step consumes.

| step | desired | read-back proof | authority touched |
|---|---|---|---|
| S0 | CLI reports declared vs observed, and fails on divergence | `cynic audit surface-registry --json` non-zero; a test that runs | new commands fragment; `Makefile` gate |
| S1 | `cynic runtime` reachable | `cynic runtime disk --json` and `inference-status --json` exit 0 with real data | none (pure wiring) |
| S2 | the guard actually runs | `cargo test -p cynic-cli` compiles and executes | resolve `architecture_boundary.rs` conflicts |
| S3 | MCP surface matches its contract | `cynic_mcp_surface` reports 9 declared-and-served, no fiction | `lib.rs` module list, then `agent-exposure.json` |
| S4 | observation is honest | `cynic models status --json` reports `declared:13, observed:0`, non-zero exit | fixes S1, S2, S5, S7 |
| S5 | model identity is provable | `GET /v1/models` id match; mismatch degrades status | `inference.toml` gains `repo_id`/`revision`/`sha256`; `openai_compat.rs`; exact match replaces `contains()` |
| S6 | disk capacity is machine-readable | numeric bytes in `cynic.host_disk_report.v1` | additive fields on `DiskFilesystem` |
| S7 | a real artifact exists | a ~1.6 G Q4_K_M in the vault with recorded digest | operator decision on space |
| S8 | a real token is rendered | `cynic chat` non-empty text, `prompt_tokens>0`, no mock | — |
| S9 | the phantom is gone | `cynic-inference-runtime` removed or populated | `Cargo.toml` members |
| S10 | the gate cannot drift | the audit is in `make check` and fails on a planted violation | `Makefile` |

**S1 before S2 is deliberate**: restoring argv reachability is a pure wiring fix with no
ambiguity, and it immediately unblocks the storage triage the operator also needs. S2 requires
resolving a 1 529-line conflicted test file and is a poor first move.

## 11. Explicit unknowns

These are `unknown`, not `blocked` and not converged:

- Whether the paged rustc ICE and the SIGBUS on `cargo test -p cynic-cli` share a root cause.
  `AGENTS.md` records pinned ICE paths; the observed failure here was `SIGBUS` during
  `native-tls` compilation. Not investigated — out of audit scope.
- Whether `cynic-inference-runtime` was ever intended to hold the orchestration its doc comment
  claims, or is pure accretion. No git history was traced.
- Which of the 16 duplicated disk observers is intended to survive. Requires an owner decision,
  not an audit.
- Whether `/var` (40 G) holds reclaimable state. Only `/home` was measured, to avoid touching
  the operator's system without a request.
