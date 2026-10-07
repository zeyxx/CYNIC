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
- **Slice 2 — capture.** Content-addressed immutable store; wire the OMP source.
- **Slice 3 — remaining normalizers** (claude, codex, hermes, antigravity, windows_gemini),
  each as an adapter with a contract test.
- **Slice 4 — model.** Materialize the hypergraph (SQLite graph adapter first).
- **Slice 5 — projection.** Rebuild SFT/DPO/diffs/compactions from the model, each
  with a `LossDeclaration`.
- The old scripts stay until each slice's contract test passes; then they are deleted
  (destructive removal, per cleanup doctrine).

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
