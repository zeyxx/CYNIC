# Spec: Rust rewrite of the session-trace ingestion pipeline (`cynic-traces`)

## 1. Goal

Give the Capture → Canonical layers of the data pipeline a normative Rust home
with machine-checked conservation, while keeping the Python package under
`apps/cynic-data` as the iteration companion. Reference: `apps/cynic-data/docs/
{ POLICY,SPEC,DESIGN }.md` (merged via #149), skills `cynic-data-centric-pipeline`
and `cynic-data-centric-ssot`.

Non-goals: embeddings / vector index / RAG ranking (downstream of canonical,
owned by `cynic-knowledge`); model training; touching the live Python suite
signature (164/164 must keep passing unmodified).

## 2. Why a new crate (pruning-policy justification)

- **Different change schedule**: the pipeline evolves with producer harnesses
  (Hermes/Claude/Codex/OMP/Antigravity/Gemini schemas), not with the kernel.
- **Heavy dependency isolation**: SQLite + JSONL streaming belong nowhere near
  `no_std` or the CLI binary.
- **Independent test reason**: conservation proofs (L1/L2/L3, FIELD_FATE,
  non-vacuity) are a self-contained proof bundle.
- **Name**: `cynic-traces`, not `cynic-data` — avoids collision with the Python
  distribution name and states the responsibility (session traces, not "data"
  in general). One responsibility: *ingest agent session traces into a
  canonical store with conservation proof*.

## 3. Layer map (what moves, what stays)

| Layer | Rust (`cynic-traces`) | Python (`apps/cynic-data`, stays) |
|-------|----------------------|-----------------------------------|
| Capture | frozen-copy + manifest (content hash), per-harness readers | `capture_*.py` kept as probe/iteration tools |
| Canonical | entities (turn/tool/reasoning/state/human/edge), FIELD_FATE per adapter, L1/L2/L3 proofs | normalizers kept until parity, then retired per-harness |
| Model | trajectory DAG (α10–α14 invariants as tests) | — (new, no Python equivalent) |
| Projection | SFT/DPO renderers with declared losses | `bridge_to_teich.py` stays (downstream reuse, do not rewrite) |

Hexagonal discipline: `domain/` pure zero-I/O, `ports/` traits, `adapters/`
(sqlite/jsonl), `usecases/` orchestration, `tests/architecture_boundary.rs`
per workspace custom.

## 4. Proof-debt transfer (from #149 triptyque — all become spec requirements)

| # | Debt | Requirement |
|---|------|-------------|
| D1 | 5 adapters UNCONTRACTED (omp, antigravity, windows_gemini, claude_code, codex) | each adapter ships FIELD_FATE + L1/L2/L3 before CONTRACTED status; UNCONTRACTED list may only shrink |
| D2 | conservation/round-trip circularity (same normalizer both sides) | independent oracle per harness (producer counters, FTS index, integrity_check) |
| D3 | `verify_conservation.py` points at absent DB + stale SRC | frozen-capture manifest is a required input; runners refuse live reads |
| D4 | `check_nonvacuity.py` GUARDS pre-move unresolvable | re-anchor guards to post-move commits; ship the checker as a Rust test harness, not a script |
| D5 | triad α10/α12/α13 declared, untested | Model-layer tests: strict DAG acyclicity, subagent resolution 100%, tool-concurrency poset |
| D6 | capture-layer non-redaction unproven | `capture_does_not_redact` test per harness; redaction stays a Projection concern |
| D7 | SQL f-strings (`VACUUM INTO`, `PRAGMA table_info`) | parameterize identifiers via whitelist; fail closed on unknown table |
| D8 | hardcoded absolute paths | `CynicPaths`-style resolver (env override → canonical → fail closed) |

## 5. Migration stages (one harness vertical first)

1. Freeze: check in a frozen capture manifest (content hashes) for the Hermes
   corpus; all proof tests read the frozen copy, never the live store.
2. Port the Hermes vertical (richest harness) with α1–α9 gates; reproduce the
   Python suite's *behavioral* signature (same counts, same fates).
3. Extract alphas as shared contracts; fan out remaining harnesses, each
   shrinking the UNCONTRACTED list.
4. Retire each Python normalizer only after its Rust twin proves L1+L2+L3 on
   the frozen capture. The Python package keeps probes, bridges, and iteration
   scripts permanently.

## 6. Acceptance

- `cargo check --workspace` green; `cynic audit all` green.
- New crate registered in `infra/crate-topology.json` with layer + justification.
- Hermes vertical: FIELD_FATE closed-world test green, L1+L2+L3 green on frozen
  capture, non-vacuity harness green (mutation-verified).
- Python suite still 164/164 untouched.
- No live-store reads in any proof test (mechanical grep gate).
