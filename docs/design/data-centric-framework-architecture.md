# Data-Centric Framework — Hexagonal / DDD Architecture (proposal)

Status: proposed. Awaiting validation before any code moves.

## 0. Why: the current shape is the defect

`infra/scripts/data_system/` = **13 913 LOC, ~35 scripts each with `__main__`**,
5 normalizer generations (v1–v5), 5 pipelines (v1–v5), 3 schemas (v1, v3, v4),
~20 curators. Every script opens SQLite directly, writes JSONL directly, and
re-implements its own parsing. There is no boundary, so there is no place to put
an invariant — which is why the same bug (lost graph, lost human prompt) recurs
across versions.

Evidence: `docs/design/essence-loss-audit.md`, `docs/design/raw-to-silver-fidelity.md`.

## 1. The four layers as bounded contexts

Named by contract, not by metaphor (no raw/silver/gold).

| # | Layer | Authority | Guarantee | Declared loss |
|---|-------|-----------|-----------|---------------|
| 1 | **Capture** | authoritative | exact bytes emitted by a harness; immutable; content-addressed | none |
| 2 | **Canonical** | authoritative | typed harness-agnostic facts (turns, tools, reasoning, **edges**, **state**, human signal) | none semantic |
| 3 | **Model** | derived | the trajectory hypergraph, materialized and queryable | structural only |
| 4 | **Projection** | derived | consumer views (SFT, DPO, PRM, diffs, compactions) | **declared, reconstructible** |

Law: a layer may only read from the layer above it. Capture and Canonical are
never overwritten. Model and Projection are disposable — their corruption is never
a loss of truth, only a rebuild.

## 2. Hexagon per bounded context

Each context is a hexagon; dependencies point inward (domain knows nothing of I/O).

```
<context>/
├── domain/      pure: entities, value objects, invariants   (no I/O, no driver import)
├── ports/       abstract interfaces (Protocol) the domain needs
├── adapters/    concrete: sqlite / surrealdb / parquet / harness readers
└── usecases/    application orchestration (imports domain + ports, not adapters)
```

Enforced by an architecture test (no `sqlite3`/`requests`/`pathlib` writes reachable
from any `domain/` module — same spirit as the Rust `architecture_boundary.rs`).

## 3. Proposed tree

```
data_system/                              # the framework (offline instrument)
├── pyproject.toml
├── src/cynic_data/
│   ├── kernel/                           # cross-cutting contracts, no domain logic
│   │   ├── contract.py                   # LayerContract: guarantee + declared loss
│   │   ├── ids.py                        # content-addressing (sha256), uid scheme
│   │   ├── registry.py                   # dynamic registry: harnesses, adapters
│   │   └── errors.py
│   │
│   ├── capture/                          # LAYER 1
│   │   ├── domain/                       # CaptureRecord, SourceLocator
│   │   ├── ports.py                      # CaptureStorePort, SourceReaderPort
│   │   ├── adapters/                     # content_addressed_fs.py, harness sources
│   │   └── usecases/capture_source.py
│   │
│   ├── canonical/                        # LAYER 2
│   │   ├── domain/                       # Session, Turn, ToolCall, Reasoning,
│   │   │                                 # Edge, StateSnapshot, HumanSignal
│   │   ├── ports.py                      # CanonicalStorePort, NormalizerPort
│   │   ├── adapters/
│   │   │   ├── store_sqlite.py
│   │   │   └── normalizers/              # one module per harness, registered
│   │   │       ├── claude.py  omp.py  codex.py
│   │   │       ├── hermes.py  antigravity.py  windows_gemini.py
│   │   └── usecases/normalize.py
│   │
│   ├── model/                            # LAYER 3
│   │   ├── domain/                       # Trajectory, Hypergraph, invariants
│   │   ├── ports.py                      # ModelStorePort, GraphQueryPort
│   │   ├── adapters/                     # store_surrealdb.py, store_sqlite_graph.py
│   │   └── usecases/build_model.py, query_model.py
│   │
│   ├── projection/                       # LAYER 4
│   │   ├── domain/                       # ProjectionSpec, LossDeclaration
│   │   ├── ports.py                      # ProjectionWriterPort
│   │   ├── adapters/                     # jsonl_writer.py, parquet_writer.py
│   │   └── usecases/                     # project_sft.py, project_dpo.py,
│   │                                     # project_diffs.py, project_compactions.py
│   │
│   └── cli/main.py                       # thin facade: capture|normalize|model|project|audit
└── tests/
    ├── test_architecture_boundary.py     # hexagonal guards
    ├── test_capture_contract.py
    ├── test_canonical_contract.py
    ├── test_model_contract.py
    └── test_projection_loss.py
```

## 4. What the framework enforces (the point)

1. **Every layer declares a `LayerContract`** — its guarantee and its declared loss.
   A projection that drops the human prompt must say so; the audit then checks the
   declaration against reality. Loss stops being invisible.
2. **The domain model is written once** — `Edge`, `StateSnapshot`, `HumanSignal` are
   entities in `canonical/domain/`, not ad-hoc columns per script. Graph fidelity
   becomes an invariant, not a per-version accident.
3. **Harness parsers are adapters behind `NormalizerPort`** — one module per harness,
   registered in the dynamic registry. Adding a harness = adding an adapter, not a v6.
4. **Stores are adapters behind ports** — SQLite today, SurrealDB when the model needs
   it; the use cases do not change.
5. **The CLI is a facade** — `capture / normalize / model / project / audit`, each a
   thin call into a use case. No logic in the CLI.

## 5. Migration (no big-bang)

- **Slice 1 — kernel + canonical (the model first).** Define the entities (Turn, Edge,
  StateSnapshot, HumanSignal) and the `NormalizerPort`. Write ONE harness normalizer
  (OMP — explicit `parentId`) end-to-end as the reference. Prove graph fidelity.
  **DONE** — but the reference adapter shipped `hermes.py` (the hardest shape: no
  parent pointer), not OMP. The design's assumption was inverted; see §7.
- **Slice 2 — capture.** Content-addressed immutable store; wire the OMP source.
  **DONE** — `SqliteSourceCapturer` + `DirectorySourceCapturer`, content-addressed
  frozen copies, manifest with content hash / size / method.
- **Slice 3 — remaining normalizers** (claude, codex, hermes, antigravity, windows_gemini),
  each as an adapter with a contract test.
  **PARTIAL** — `hermes.py`, `claude_code.py`, `codex.py` landed, each with a
  contract test. `omp.py` landed with three proofs no earlier adapter has:
  a record-type conservation audit (`skipped_record_types` closes 39 491 raw
  records with zero unknown), a field-level closed world (`FIELD_FATE` + a test
  that fails on any unmapped raw field), and a frozen-capture gate
  (`capture_omp.py` + `run_omp_canonical.py`, which refuses live reads).
- `windows_gemini.py` landed under the same bar: session = `sessionId` group
  (disjoint chat files merged in timestamp order + inventoried
  `tool-outputs/session-<id>/` dir, never content-linked — no reliable per-call
  key exists, and counts do not align). Inline results are masked/truncated as
  the harness recorded them (`<tool_output_masked>`, `[TRUNCATED]` preserved,
  never resolved). One 13.5 MB chat is corrupt on disk; `_load_chat` applies
  two documented repairs (978 messages recovered, `repaired: true` in meta).
  Conservation: 6 255 turns = 6 255 messages, 5 818 tools, 4 489 reasoning,
  5 626 states; the closed world caught `functionResponse.parts` (base64
  binaries) before landing.
- `antigravity.py` landed under the same bar: session = one trajectory DB,
  every `step_payload` walked as schemaless protobuf (field numbers kept,
  names never invented). Envelope: `f1` = type, `f4` = status (proven equal
  to the columns on all 58 092 steps), `f5.f1` = timestamp, `f5.f3` = native
  side marker, `f5.f4` = tool call (id/name/args). No call<->result id linkage
  exists (628 call ids, zero in any result step) -> all tool edges DERIVED.
  Conservation exact: 58 092 turns = 58 092 steps, 500/500 DBs, 0 failed,
  zero NATIVE edges. The closed world caught 12 fields sample probes missed
  (`f133`/`f147` shell plans, `f56`/`f154` user questions, `f148` subagent
  tasks...). One 22 MB DB is permission-locked (`600 cynic:cynic`); it walks
  fine once readable and is frozen with a hash-matched privileged copy.
  Remaining: nothing — Slice 3 (codex/cli, omp, windows_gemini, antigravity)
  is complete.
- **Slice 4 — model.** Materialize the hypergraph (SQLite graph adapter first). NOT STARTED.
- **Slice 5 — projection.** Rebuild SFT/DPO/diffs/compactions from the model, each
  with a `LossDeclaration`. NOT STARTED.
- The old scripts stay until each slice's contract test passes; then they are deleted
  (destructive removal, per cleanup doctrine).

### 5.1 The vertical + the alphas (method)

A single harness is taken **all the way down** before the next one starts, and the
invariants it proves are extracted as **alphas** — shared contracts every following
harness must satisfy. This avoids re-proving the same invariants six times and
avoids a contract that is only as strong as the easiest harness.

**Audit has three cumulative levels.** Level 1 alone is not enough: it catches a
*new* field but stays blind to a field mapped to the *wrong* home (fate declared,
value lost) or a *mutated* value in a correctly mapped field.

| Level | Proves | How |
|---|---|---|
| **L1 closure** | every producer field has exactly one fate | the observed field set is parsed from the producer's OWN DDL (`SCHEMA_SQL`), never hand-kept; a new column fails the test |
| **L1 anchor** | the contract cannot drift silently | pinned to the producer's `SCHEMA_VERSION`; a bump fails until re-derived |
| **L2 value** | a `preserved` field's value survives | for a RANDOM sample of rows, the value read back from the normalized session equals the raw value (or its declared transform) |
| **L3 re-derivability** | a `declared` loss is honest | the loss names a reason; the claim ("token_count unpopulated") is checked against the store |

**Random sampling is the point.** An average hides a rare shape. On Hermes, the
rare shapes are the whole story: a codex model whose CoT is encrypted, a
multi-model session, a compaction that rewrote 11 527 messages in place.

**Alphas extracted from the Hermes vertical** (promoted to every harness):

| α | Contract | Proven on Hermes |
|---|---|---|
| α1 | closure derived from the producer's DDL | 92 columns (30 messages + 62 sessions) |
| α2 | version anchor (`producer_schema_version`) | pinned to `SCHEMA_VERSION = 31` |
| α3 | concept resolution by model (one concept, N homes) | reasoning has 4 homes; resolved by model |
| α4 | value conservation (L2) on a random sample | turn content/role, reasoning home, active flag, session row |
| α5 | re-derivability of declared losses (L3) | `token_count` claim checked against the store |
| α6 | contract mandatory at registration | a normalizer without a full contract is refused |

### 5.2 Hermes vertical — the four reasoning homes (measured)

Hermes is the priority harness: it is the richest (297 988 messages, 1 744
sessions, **59 distinct models**, 67 multi-model sessions) and the most
heterogeneous *inside itself*. The decisive finding is that **reasoning has four
homes, chosen by the MODEL, not the harness**:

| Home | Carries | Example model |
|---|---|---|
| `reasoning_content` / `reasoning` | plain CoT (the two are a 1:1 duplicate) | poolside/laguna-s-2.1 |
| `reasoning_details` | structured text spans | gpt-5.6-luna-900k (partial) |
| `codex_reasoning_items` | **encrypted** CoT (`gAAAAA…`), presence + bytes only | gpt-5.3-codex-spark |

Reading only `reasoning_content` (the previous adapter) loses **8 462 messages
outright**; a model can also use *more than one* home in the same session
(`longcat-2.5-preview-free` was observed on both `reasoning_content` and
`codex_reasoning_items`), so the contract records the observed home per model and
treats a known model using an unexpected home as **drift**, never silence.

**Compaction rewrites history in place**: `active = 0` marks a message a
compaction replaced — **173 696 rows** on the live store (poolside/laguna-s-2.1
alone: 60 902). The previous adapter read them as live history.

Proof: `tests/test_hermes_conservation.py` — L1/L2/L3, 14 tests, random sample
seeded, all green. `src/cynic_data/canonical/domain/conservation.py` is the pure
alpha module; `hermes_contract.py` is the pinned instance.

### 5.3 Test infrastructure debt (fixed)

`pyproject.toml` declared no `pythonpath`, so `pytest` could not import `cynic_data`
and all 4 pre-existing test modules failed at collection. Tests only ran with a
manual `PYTHONPATH=src`. Fixed with `pythonpath = ["src"]` — the suite is now
self-contained.

## 6. Decisions needed

1. **Location & language**
   - (R) Python package `infra/data_system/src/cynic_data/` — offline instrument, fast
     iteration, model transposable to Rust later.
   - Rust crate `crates/cynic-data/` now — 100% Rust supply chain, but slow iteration on
     a model still being discovered.

2. **Model store for Layer 3**
   - (R) Start with a SQLite graph adapter (proves the model, zero new dependency),
     add a SurrealDB adapter behind the same port when traversal/vector queries need it.
   - SurrealDB from day one — matches the multi-model target but adds a moving part
     before the model is proven.

3. **Scope of this pass**
   - (R) Slice 1 only (kernel + canonical domain + OMP normalizer + boundary tests) —
     prove the skeleton on one harness, then fan out.
   - All four layers skeletoned at once — faster to see the whole, higher risk of
     designing the wrong contract before one harness proves it.

## 7. Observed divergence — the reference adapter was the wrong choice

The design named OMP (explicit `parentId`) as the reference harness on the theory that
the easiest shape validates the skeleton first. The implementation shipped **Hermes**
instead, which is the *hardest* shape: no parent pointer at all, so every causal edge is
`DERIVED` from row ordering.

This inversion turned out to be correct, and it is now a proven fact rather than a
preference:

| Harness | Parent pointer | Reply edges | Tool-result linkage |
|---|---|---|---|
| claude_code | native `parentUuid` | `NATIVE` (1187) | `NATIVE` via `tool_use_id` |
| codex | none | `DERIVED` (sequence) | `NATIVE` via `call_id` |
| hermes | none | `DERIVED` (sequence) | `NATIVE` via `tool_call_id` |

Building Hermes first forced the `EdgeProvenance.NATIVE` / `DERIVED` distinction to be
a real, load-bearing part of the model from the start. Had OMP come first, that
distinction would have looked decorative until Codex exposed it. A reference adapter
should be the shape that *breaks* the model early, not the one that flatters it.

## 8. Harness facts the canonical layer must not overwrite

Observed on the real archives (2026-10-05), recorded here because they look like bugs
and are not:

- **Claude Code persists no chain-of-thought** in the session JSONL (`reasoning: 0`).
  `ReasoningTrace` stays empty for this harness — that is the truth, not a loss.
- **Claude Code carries `message.usage` only on assistant turns.** A user turn yields
  `context_prompt_tokens = None`. Unobserved is not zero.
- **Codex emits `turn_context` *after* the first messages**, so `turns[0].native_model`
  is `None`. The model is known from that turn onward.
- **Codex names tool results `function_call_output`.** The canonical entity keeps the
  native name rather than forcing a uniform label — renaming would fabricate a shape
  the harness does not have.

## 9. Host instability is not a code failure

The suite intermittently dies with a core dump. `/var/log/syslog` shows kernel
`segfault ... error 4` on unrelated processes (chrome, hermes, python3.14 service
workers), on a different CPU each time. Root cause is the known unstable host
(Ryzen 5700G, no ECC, mixed DDR4-3200) — not this package. A core dump during
`pytest` is hardware debt; a *failing assertion* is real. Never debug one as the other.
