# PRD — Hermes harness mining (evolving)

Status: living document. Append-only. Never rewrite an entry — supersede it with a new one.
States per (sub-)problem: `proposed` → `sliced` → `in_progress` → `proof_pending` → `proven` | `blocked` | `dropped`.
A state change carries: date, evidence link, previous state. No evidence = no transition.

## 1. Objective

Turn repeated Hermes behaviors into maintained skills.
No invention. Every claim cites a count from `trace_report_hermes.json`.

Corpus (frozen, hermes harness only):

- gel `hermes.e3bc42cc9f7022ff.sqlite` (sha256 `e3bc42cc…46aee53`), 1.8 GB
- canonical `canonical_hermes.sqlite`, 3.3 GB — 2 234 / 2 235 sessions ok, 1 failed (malformed DB)
- 447 415 turns, 442 519 tools, 100 462 reasoning, 1 144 933 edges (699 376 native)
- 19 499 human signals, 7 343 corrections
- report: `/home/user/.hermes/cache/scratch/trace_report_hermes.json`

## 2. End-to-end pipeline (SOTA, local-only)

1. capture immutable, content-addressed (`capture_hermes.py` → frozen sqlite + manifest)
2. normalize hermes-only to canonical store (`run_hermes_canonical.py`)
3. conservation proof: source counts == canonical counts
4. mine counters only (tools, bigrams, error buckets, human kinds, overlap) — no raw text retention
5. generate skill candidates with dedup against `~/.hermes/skills/` (109 existing)
6. human review: create | merge into existing (preferred) | drop
7. merge evidence into the touched skill; candidate folder stays empty by policy

### 2.1 Skill Lifecycle: Emergence → Malleable Skill → Native Rust Distillation (Axiom BURN)

A skill does not mine solely from errors; it emerges from **the entirety of execution traces** (successes, human signals, frequent tool bigrams, structural invariants).

The lifecycle follows 5 rigorous stages:
1. **Whole-Trace Mining** : Surface recurrent behavioral patterns across all turns, not just failure modes.
2. **Malleable Skill Layer** : Maintained prompt/heuristic specification in `~/.hermes/skills/` during active iteration.
3. **Stagnation Gate** : When a skill's update frequency approaches zero (update delta < $\epsilon$), the heuristic has converged.
4. **Native Rust Distillation** : Distill the stabilized skill directly into pure, deterministic zero-I/O Rust domain logic in `crates/`.
5. **Algorithmic & Cognitive De-bloat (Axiom BURN)** : Deprecate the LLM skill. Replace prompt parsing with a native function, reducing total lines of code (LOC), runtime latency, and cognitive complexity.

## 3. PRD invariants

- I1: live reads refused — only the frozen gel is read (`CYNIC_HERMES_CAPTURE` required).
- I2: every gel carries hash + manifest; every number cites the report.
- I3: Rust == Python parity proven by machine before any new extraction.
- I4: no pattern asserted below threshold — below threshold stays `unknown`.
- I5: secrets never copied into skills (values stripped, rule kept).
- I6: candidates stay in review — never auto-injected into live skills.
- I7: every guard is non-vacuous — the proof fails without its code.

## 4. Problems (each sliced; states updated in place)

### P1 — Unbounded exploration loops [state: sliced]

- evidence: `read_file -> read_file` 8 609; `terminal <-> tool_result` > 100k each way
- [ ] P1a `proposed`: bound read phase (budget per question, then act)
- [ ] P1b `proposed`: detect Bash->Bash->Bash runs and force a read or patch step
- [ ] P1c `proposed`: skill candidate `bound-exploration` (trigger + steps + pitfalls)

### P2 — LOCKED / contention failures [state: sliced]

- evidence: `LOCKED` 906 + `locked` 35 + `busy` 1 of 4 744 error rows
- [ ] P2a `proposed`: classify LOCKED sources (sqlite writer vs file vs git)
- [ ] P2b `proposed`: retry-with-backoff rule vs fail-fast rule per source
- [ ] P2c `proposed`: overlap guard for 1–9 pattern (same branch/file/time)

### P3 — 12 156 unknown human signals [state: sliced]

- evidence: `human_kinds.unknown` 12 156 / 19 499; known: approval 3 370, constraint 2 366, rejection 1 105, redirect 502
- [ ] P3a `proposed`: cluster unknown texts into 5–10 intents (counters only)
- [ ] P3b `proposed`: promote intents above threshold to kinds; rest stays `unknown`
- [ ] P3c `proposed`: feed new kinds into rerank/synthesis weights

### P4 — One malformed session + one inconsistent snapshot [state: in_progress]

- evidence: `20260809_013853_b0b827: database disk image is malformed`; capture logged `Rowid 43975 out of order`
- [x] P4a `proven` 2026-10-23: quarantine the malformed session id, keep it out of canonical
  - cause found: corrupt b-tree row (messages rootpage 7, rowid 43975) in that session;
    two captures reproduce it → source-side defect. That session's messages carried live
    secrets in plain tool output — never mined, never copied; rotation is operator business.
  - resolution: CLI `.dump`+reload rebuilt the region clean; the 5 ghost rows read fine now.
- [x] P4b `proven` 2026-10-23: re-capture with integrity gate green before next mine
  - `dumped.sqlite`: integrity `ok`, 2 233 sessions + 2 orphans, 447 605 messages, zero loss.
  - `normalize_green.log`: 2 235/2 235 ok, 0 failed, EXIT 0 (303.6 s).
  - totals: 447 605 turns, 442 674 tools, 7 349 corrections, 19 518 human, 1 145 282 edges.
- [ ] P4c `proposed`: document fail-closed rule (malformed source ⇒ refused, never skipped silently)

### P5 — 1–9 multi-agent overlap model [state: in_progress]

- evidence: 853 sessions with parent; 803 on CYNIC cwd; branches `agent/hermes/*`
- [x] P5a `proven` 2026-10-23: true overlap from session windows, no fill (977 open/missing
  bounds stay `unknown`, never defaulted). Closed usable: 1 256. Proven peak: **19 simultaneous**.
  9–10 concurrent are frequent (267/253 window events) — the 1–9 pattern is real, ceiling higher.
  26 cwd+branch groups overlap (precise all-pairs count in P5b: 2 421). Report: `scratch/overlap_report.json`.
- [x] P5b `proven` 2026-10-23: collision index (same cwd + branch + overlapping window)
  - 2 421 paires en collision, recouvrement médian 234 s, p90 3 973 s
  - index (5 000 premières) : `scratch/collision_index.json`
- [x] P5c `proven` 2026-10-23: per-agent load (tools/min when N concurrent) → backpressure rule
  - 862 sessions mesurées, médiane outils/min par concurrence : 1→6.6, 2→15.7, 3→20.0,
    6→16.5, 9→8.4, 10→6.2, 14→8.6. Pas de courbe propre : le type de tâche domine
    (rafales CLI contre TUI idle). Report: `scratch/load_report.json`.
  - règle proposée (brouillon, non prouvée) : au-delà de 8 simultanées sur même cwd+branche,
    étaler les lancements; à valider par expérience dédiée avant de durcir.
- trap found 2026-10-23: `run_hermes_canonical.py --out` is IGNORED, the store uses
  `CYNIC_CANONICAL_DB`/default `scratch/canonical_hermes.sqlite` (it silently overwrote the
  first DB). No fix in Python (legacy chain); the Rust vertical must take `--out` as truth.

### P6 — 109 skills never deduped [state: proposed]

- evidence: `~/.hermes/skills/` count 109; 3 stale candidates from August run
- [ ] P6 `proposed`: dedup pass — map each candidate to existing skill or create; review folder ends empty

## 5. Checklist (gate before next harness)

- [ ] gel hash + manifest recorded above; integrity gate green (P4b)
- [ ] conservation counts equal (source == canonical)
- [ ] `trace_report_hermes.json` regenerated from the green gel
- [ ] every new claim cites a report count
- [ ] candidates reviewed (create/merge/drop); review folder empty
- [ ] touched skills updated with evidence lines
- [ ] builder proof green for any Rust change (never built on cortex)

## 6. Journal (append-only)

- 2026-10-23: created from fresh hermes-only mine (2 234 sessions). Prior August report (Windows, 3 436 sessions) retired as stale for hermes scope.
- 2026-10-23: dump fidelity proven — quarantined vs dumped gel identical: 65 indexes,
  43 tables, 13 triggers, 2 views; FTS 447 605 / 204 630; side tables equal
  (model_usage 2 884, delegations 51, obligations 16). Zero flattening loss: source holds
  no graph store (edges are derived downstream into canonical `edges`), so `.dump`+reload
  loses nothing. Re-study: `raw-to-silver-fidelity` (hermes native edges = tool_call_uid +
  id ordering, no parent pointer; old normalizers set parent None — the flattening sin),
  `essence-loss-audit` (Gold dropped graph + human signal; root cause entity-extraction —
  our canonical keeps `edges`/`states`/`human_signals`), `geodesic-retrieval-delta` R1
  (Model MUST be causal graph; retrieval walks it), `RAG_SOTA_PATH` (MinMax fusion beats
  RRF; cross-encoder rerank next; hard-negative fine-tune gated on rig).
