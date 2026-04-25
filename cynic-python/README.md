# CYNIC Python Tier-2 Toolkit

Analysis & heuristics for multi-domain deterministic evaluators.

## Key Principles (from .claude/rules/python.md)

- **P1-P6:** Enforced via CI (mypy, pytest, ruff)
- **P7-P15:** Design guidance (measurement, real data, observability)
- **Never ship Python to production:** only Rust implementations of Python-derived rules
- **Tier-2:** Python validates/analyzes; Rust kernel owns gates

## Structure

```
cynic-python/
├── heuristics/        # token.py, wallet.py, chess.py
├── validation/        # measure_agreement.py, tune_gates.py
├── datasets/          # token_validation.json, wallet_validation.json
├── tests/             # unit + integration tests
└── artifacts/         # outputs (gates, metrics, logs)
```

## Dev Setup

```bash
pip install -e ".[dev]"
mypy --strict cynic-python/
pytest cynic-python/ -v --cov
python cynic-python/validation/measure_agreement.py
```

## Timeline (Apr 25 → May 1)

| Date | Task | Owner |
|------|------|-------|
| Apr 25-26 | Framework + dataset curation | T. |
| Apr 26-28 | Implement heuristics (token + wallet) | T. |
| Apr 28-29 | Measure agreement vs LLM Dogs | T. |
| Apr 29-30 | Tune gates + finalize artifacts | T. |
| May 1 | Rust deterministic-dog → consume artifacts | T. |

## Outputs

Git-tracked artifacts:
- `artifacts/token_gates_v1.2.json` → Rust kernel includes
- `artifacts/wallet_gates_v1.2.json` → Rust kernel includes
- `artifacts/agreement_report.json` → validation metrics
- `artifacts/tuning_log.json` → iteration history
