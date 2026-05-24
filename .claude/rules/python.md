## Python Rules

Mirrored from kernel.md (Rust rules), adapted for Python. Python is NOT thin scripts. Python is tier-2 infrastructure (after Rust tier-1 kernel). All rules are **mechanical** (enforced via CI) or **design** (judgment guidance).

### Philosophy

Python tier-2 ≠ Python tier-1 (kernel). Python never owns critical gates, state machines, or core logic. Python owns:
- **Analysis & validation:** measure heuristic agreement, calibrate thresholds
- **Data processing:** ETL, enrichment, transformation
- **Experimentation:** A/B testing, hypothesis evaluation
- **Tooling:** CLI utilities, monitoring dashboards, audit trails

Production Python never calls kernel via REST (breaks sovereignty). Kernel consumes Python outputs (rulesets, thresholds) via code, not API.

### Enforced (CI gates)

**P1. Type annotations on all functions.** `mypy --strict` must pass. No `Any` without `# type: ignore` + WHY comment.

**P2. Test coverage ≥ 80%.** `pytest --cov` must report ≥80% for all modules.

**P3. Dependencies pinned.** `pyproject.toml` specifies exact versions. `pip freeze > requirements.lock` on every CI run.

**P4. No subprocess shell=True.** All `subprocess.run()` calls use list args (no shell).

**P5. Structured logging only.** All logs use `logging.info(msg, extra={...})` or JSON format. No print().

### Design Principles (judgment guidance)

**P9. Fail loud and early.** Unexpected input → raise exception immediately. No `except: pass`.

**P12. Test on real data.** Unit tests on synthetic data OK. Integration tests MUST use real samples (real tokens, real wallets).

**P14. Single source of truth per domain.** Token heuristics in `token.py`, wallet in `wallet.py`, chess in `chess.py`. Never duplicate.

**P16. Vocabulary contract across producer-consumer.** When a Python producer emits tagged data (narrative labels, status codes, domain IDs) that a consumer uses for routing/gating, both sides MUST share a single vocabulary file. The producer imports from it; the consumer reads it. Zero orphan tags on either side. Incident: x_proxy.py emitted `"warning"/"hype"` while narrative_domains.yaml expected `"rug_pull"/"pump_hype"` — ZERO overlap, narrative routing was dead code for the entire dataset (2026-05-16). — Falsify: add a narrative tag to the producer without updating the YAML; the contract check script should fail.

**P17. Append-only files need schema versioning.** JSONL datasets that grow by appending MUST include a `schema_version` field in every row. When the schema changes, the new producer writes a new version number. Consumers MUST filter by version or the dataset accumulates incompatible rows that pollute all metrics. Incident: dataset.v2.jsonl contained 2298 v1 rows (16 fields) mixed with 2890 v2 rows (49 fields) — 44% of the dataset was noise with no signal_score, no enrichment (2026-05-16). — Falsify: query `SELECT DISTINCT schema_version FROM dataset` — if >1 version exists, either migrate or split.

**P18. Distinguish transient from permanent errors.** HTTP 429 (rate limit) and 503 (overloaded) are transient — retry with backoff. HTTP 400/404/422 are permanent — don't retry. A daemon that treats all non-200 as permanent errors silently drops data when the cursor advances past failed rows. Incident: ingest daemon treated 429 as failure, cursor advanced, 176 tweets lost irrecoverably (2026-05-16). — Falsify: mock a 429 response; verify the row is retried and eventually succeeds or is explicitly dead-lettered.

### Structure

```
cynic-python/
├── pyproject.toml
├── requirements.lock
├── heuristics/
│   ├── __init__.py
│   ├── token.py
│   ├── wallet.py
│   └── chess.py
├── validation/
│   ├── measure_agreement.py
│   └── tune_gates.py
├── datasets/
│   ├── token_validation.json
│   ├── wallet_validation.json
│   └── README.md
├── tests/
│   ├── test_heuristics_token.py
│   ├── test_heuristics_wallet.py
│   ├── integration_agreement.py
│   └── integration_dataset.py
└── README.md
```

### Exports to Kernel

Python produces artifacts consumed by Rust via `include_str!()` or REST observation. No Python in production — ship Rust implementations of Python-derived rules.
