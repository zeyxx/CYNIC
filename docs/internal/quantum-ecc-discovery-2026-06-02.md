# Quantum ECC Challenge — Discovery Log 2026-06-02

## Situation (observed)

- **Challenge**: ecdsa.fail — optimize reversible secp256k1 point addition circuit
- **Metric**: avg_toffoli × peak_qubits (lower is better)
- **Repo**: https://github.com/ecdsafail/ecdsafail-challenge

## Baseline confirmed

```
avg executed Toffoli : 1,688,703
peak qubits          : 1,558
score                : 2,630,999,274
```

This already **beats the "3B Google target"** cited in the June 2026 article.
The repo has been updated since Gajesh's 6.5B run.

## Circuit architecture (observed — read from mod.rs)

The circuit is **not** naïve affine point addition. It uses:

- **Dialog-GCD algorithm** for modular inversion (Stehlé-Zimmermann style binary GCD)
- **Karatsuba multiplication** (`ROUND84_XTAIL_KARATSUBA=1`) — already implemented
- **Qubit windowing** (`DIALOG_GCD_APPLY_WINDOW_BLOCKS`, `HOST_GATED`) — already done
- **Measured uncomputation** (`DIALOG_GCD_MEASURED_APPLY_SUB`) — already done
- **Dozens of micro-optimizations** tuned via Fiat-Shamir reroll search

All four levers from our initial tractability report (windowed scalar mult, projective coords,
Toffoli ladder, Karatsuba) are **already implemented** in the existing codebase.

## Why the tractability report was wrong (epistemic lesson)

The Phase C Workflow probe returned "O = observed" findings, but the circuit content agent
stalled (parallel[1] failed). So the architecture characterization was based on the Gajesh/
Gautham descriptions (6.5B era), not the current repo state (2.63B era).

**Lesson**: always read the actual code before designing the optimization strategy.
`grep -n "configure_ecdsafail_submission_route"` in 5 seconds would have revealed this.

## What actually drives improvements at this frontier

From the comments in `configure_ecdsafail_submission_route()`:

> "Found by a 2D reroll search (DIALOG_REROLL=3 + DIALOG_POST_SUB_REROLL=18) —
> 1D reroll sweeps miss it. 0/0/0 @ 1571."

The search process is:
1. Tweak a structural parameter (COMPARE_BITS, WIDTH_MARGIN, etc.)
2. Do a 2D grid search over (DIALOG_REROLL, DIALOG_POST_SUB_REROLL)
3. Look for a "clean island" — a parameter combination where all 9024 test shots pass
4. The Fiat-Shamir hash derives test inputs from the op stream → reroll shifts the stream

Each successful improvement in the git history followed this pattern.

## Key parameters and current values

| Parameter | Current | Role |
|-----------|---------|------|
| `DIALOG_REROLL` | 13 | Fiat-Shamir stream seed offset |
| `DIALOG_POST_SUB_REROLL` | 14 | Post-subtract stream seed |
| `DIALOG_GCD_COMPARE_BITS` | 59 | Branch comparator width (−Toffoli, ±qubits) |
| `DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS` | 19 | Apply-phase clean comparator width |
| `DIALOG_GCD_WIDTH_MARGIN` | 27 | GCD body width envelope |
| `DIALOG_GCD_ACTIVE_ITERATIONS` | 399 | GCD iteration count |
| `DIALOG_GCD_APPLY_WINDOW_BLOCKS` | 2 | Carry lane windowing blocks |
| `DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS` | 2 | Chunked apply F blocks |
| `DIALOG_GCD_APPLY_CHUNKED_F_CUT` | 70 | First cut position |
| `KAL_DOUBLE_CARRY_TRUNC_W` | 20 | Kaliski double carry truncation |
| `KARA_SOL_SHIFT22_DOUBLES` | 1 | Replace shift-22 with doublings |

## Phase A pivot: parameter search harness

Built `search.py` — runs `cargo eval_circuit` with env var overrides, no Gemini needed.

Modes:
- `reroll2d`: 2D grid over DIALOG_REROLL × DIALOG_POST_SUB_REROLL (most promising)
- `compare_bits`: sweep COMPARE_BITS with co-tuned reroll scan
- `width_margin`: sweep WIDTH_MARGIN with co-tuned reroll scan

```bash
cd cynic-python/lab/quantum-ecc
python3 search.py --repo ./ecdsafail-challenge --mode reroll2d --dry-run  # preview
python3 search.py --repo ./ecdsafail-challenge --mode reroll2d             # run overnight
```

Expected: ~500 combinations × ~30s each ≈ 4h total. No API key required.

## Phase B (later): algorithmic study

Read the Dialog-GCD paper (Stehlé-Zimmermann or Bernstein-Yang) to understand
what structural changes could reduce the Toffoli count below current level.
Potential angles:
- Reducing GCD active iterations (currently 399) with tighter termination
- Half-GCD variants (partially present in codebase as `halfgcd_live_pa`)
- Improved carry management in the GCD body

## What to submit

If `search.py` finds a new island with score < 2.63B:
1. Record the params in `search_state.json`
2. The circuit runs with `env var overrides` (no code changes) — submission means
   documenting the env vars and running `eval_circuit` with them
3. Open a PR/issue on the ecdsa.fail challenge repo with the result
