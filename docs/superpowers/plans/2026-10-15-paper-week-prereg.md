# Paper Week Exit Evaluation — Preregistration v1

> Status: prereg draft, frozen before first signal. Any edit after first signal = new version, never silent.
> Date: 2026-10-15. External pin: TrenchScanner @ cffd91c (manifests in data_system/vault/captures/external-pins-2026-10-15/).
> Division: GCRtrd owns entries (detection). We own the exit evaluation protocol. No live trading, no keys.

## Hypotheses (each with H0, metric, rejection rule)

- H1 (primary): close-confirmed trailing exit beats instant-tick trailing exit on net median return per trade.
  H0: median(A − B) = 0. Metric: paired net-return difference per trade. Test: Wilcoxon signed-rank, two-sided α = 0.05.
  Rejection of H0 requires p < 0.05 AND |Δ median| ≥ 5 pp. Else: inconclusive (never "no effect").
- H2 (secondary): A vs B within topslice-passing calls. Same test, Holm-corrected with H3.
- H3 (secondary): A vs B within Gemerald-passing matches. Same test, Holm-corrected with H2.
- Descriptive only: topslice vs default entries, Gemerald vs default entries (different populations, no causal claim).

## Arms (exact)

- Entry (both arms): price = freshQuote when present and plausible (ratio ≤ 5 vs scan price), else scan price.
  Time = CuratedAlert.createdAt (ms). Never Telegram reception time.
- Arm A = EXIT_PLAN copy from pin cffd91c profitSim.ts: takeProfits [{multiple 2, sellFraction 0.5}],
  stopFraction 0.5, maxHoldMinutes 30, trail [{fromMultiple 2, fraction 0.35}], trailMaxHoldMinutes 180.
  Trail fills at seen price under the level. Stop fills at the level.
- Arm B = same entries; trailing exit confirmed by candle close instead of tick.
  Timeframe: 1m (default, TO-FREEZE before first signal; 5m fallback only if 1m closes missing).
- Costs: −1% entry, −1% per sale leg. Applied identically to both arms.
- Frozen for the run: champion modelId, cutoffs, rule-4-only rows. Recorded at freeze (J-1), see unknowns.

## Ledger (conservation)

- JSONL append-only, one line per position per arm. Line 0 = SHA-256 of this file.
- Signal line fields: signal_id, raw_capture_sha256, prev_hash, mint, createdAt_ms,
  entry_price, entry_reason (fresh|slow), arm, exclusion_reason (nullable).
- Raw capture (alert JSON + snapshot) frozen in vault captures with SHA-256 BEFORE any processing.
- n signals = n evaluated-or-excluded lines. Missing data → NON_EVALUABLE, excluded, never imputed.
- Disqualified rows: excluded from primary, logged with reason.
- Multiple alerts same mint: independent positions, logged. Overlapping positions allowed (per-trade metric, no portfolio constraint).

## Verdict rules (fixed before measure)

- Primary set: trades closed in BOTH arms. Report completion rate per arm; < 90% closed = limitation flagged.
- Primary stat: median net return per trade + IQR. Secondary: 10/90 winsorized mean. Never raw mean alone.
- Fidelity gate: |paper return − GCRtrd grade| ≤ 5 pp acceptable per trade; > 10 pp on aggregate = investigate, do not deploy.
- Target n: ≥ 60–80 per arm by week 4. Week 1 = pilot + corpus (all price paths stored for matrix v2).
- Week-4 verdict cross-reviewed by GCRtrd + 10% positions independently recalculated (double entry, explicit tolerance).

## Explicit unknowns (TO-FREEZE before first signal)

1. Candle timeframe 1m vs 5m (default 1m).
2. Champion modelId + freeze datetime (J-1, from GCRtrd).
3. Data access shape: subscriber account vs read-only endpoint vs history dump.
4. Gemerald filter definition (params) + its Match rows access.
5. Daily report cutoff hour + X format.
