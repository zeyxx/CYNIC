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

**P6. Entry points versioned.** Every script/module exports a `__version__` and announces it on startup.

### Design Principles (judgment guidance)

**P7. Observability by design.** Every module must expose metrics (success count, error rate, timestamp). Silent Python = invisible failure.

**P8. Never hold state.** Modules are stateless. Persist to kernel (SurrealDB) or disk (JSON, git-tracked). No in-memory caches beyond request lifetime.

**P9. Fail loud and early.** Unexpected input → raise exception immediately. No `except: pass`.

**P10. Document assumptions.** Every function docstring: input contract, output guarantees, failure modes, valid domains.

**P11. Version artifacts, not code.** Commit rulesets/thresholds (JSON/YAML) to git. Don't commit generated artifacts.

**P12. Test on real data.** Unit tests on synthetic data OK. Integration tests MUST use real samples (real tokens, real wallets).

**P13. Measure before tuning.** Before any heuristic change, measure: before (confusion matrix vs LLM), after (same), accept only if specificity ↑ AND sensitivity drop ≤5%.

**P14. Single source of truth per domain.** Token heuristics in `token.py`, wallet in `wallet.py`, chess in `chess.py`. Never duplicate.

**P15. Python → Rust is 1:1.** Rust code from Python must be line-for-line translatable. Avoid Python idioms that need cleverness to port.

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

Python produces artifacts (JSON/YAML, git-tracked):

```
cynic-python/artifacts/
├── token_gates_v1.2.json
├── wallet_gates_v1.2.json
├── agreement_report.json
└── tuning_log.json
```

Rust kernel reads these:

```rust
const TOKEN_GATES: &str = include_str!("../../artifacts/token_gates_v1.2.json");
const WALLET_GATES: &str = include_str!("../../artifacts/wallet_gates_v1.2.json");
```

### Long-term Maintenance

**Every 3 months:**
1. Run integration tests against latest Dogs
2. Measure agreement (target ≥90%)
3. If drift, spawn tuning session (measure → tune → commit)
4. Update kernel artifact versions

**Never ship Python code to production.** Ship Rust implementations of Python-derived rules.

---

*P15 corollary: Python rules are FROZEN in Rust once written. Changes to Python heuristics require re-tuning + re-shipping Rust binary.*
