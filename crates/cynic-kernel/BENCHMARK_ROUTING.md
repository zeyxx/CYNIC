# Routing Calculator Benchmark

## Hypothesis

By filtering Dogs based on observed performance metrics (latency SLA + success rate), the pipeline produces higher-confidence verdicts on the benchmark corpus.

**Acceptance Criterion:** Routing-filtered Dog selection improves accuracy (verdicts within expected bounds) by ≥5% compared to using all available Dogs.

## Test Architecture

### Files

- `tests/integration_routing_benchmark.rs` — Main benchmark test
- `tests/fixtures/benchmark.json` — 30 stimuli stratified by domain (chess, general) and verdict tier (HOWL/WAG/GROWL/BARK)
- `src/infra/routing_calc.rs` — RoutingCalculator implementation

### Data Flow

1. **Load Corpus** (`benchmark.json`)
   - 30 stimuli with expected score ranges (score_min/score_max)
   - Domains: chess, general
   - Verdicts: Howl (high confidence), Wag, Growl, Bark (low confidence)

2. **Inject Dog Performance**
   - Simulate different Dog profiles per domain
   - Fast Dogs: avg_latency_ms=10-50, success_rate=0.98-0.99
   - Slow Dogs: avg_latency_ms=250-500, success_rate=0.91-0.97

3. **Apply Routing Filter**
   - SLA: 200ms max latency
   - Success rate: ≥0.95
   - Result: Returns Dogs sorted by latency (fastest first)

4. **Measure**
   - Run both scenarios (with filter / without filter) on representative stimuli
   - Compare Q-score accuracy against expected bounds
   - Calculate % improvement

## Running the Benchmark

Currently marked `#[ignore]` because it uses simulated scores. To run:

```bash
cargo test --test integration_routing_benchmark -- --ignored --nocapture
```

## Unit Test (Always Runs)

```bash
cargo test routing_calc_filters_by_sla
```

Validates:
- SLA filtering (excludes Dogs exceeding latency_sla_ms)
- Success rate filtering (excludes Dogs below 0.95 success_rate)
- Latency sorting (fastest Dogs first)

## Live Benchmark (Phase 2)

Once the observer is wired (K15 seam 3), replace simulated scores with real pipeline calls:

1. Routing calculator observes Dog performance from real pipeline runs
2. Each stimulus judgment invokes `pipeline.run()` with:
   - Full Dog list (baseline)
   - Filtered Dogs via `routing_calc.dogs_for_domain()` (experimental)
3. Measure and compare:
   - Q-score accuracy (within expected bounds)
   - Average latency per judgment
   - Error rate (failed Dogs)
4. Confirm hypothesis: routing improves accuracy by ≥5%

## Expected Results

- **Current (simulated):** Routing improves accuracy by ~5% (at the acceptance threshold)
- **Target (live):** Routing improves accuracy by 10-15% once observer has real data

## Design Notes

- Benchmark corpus is **stratified**, not random. All verdict tiers represented.
- SLA_MS=200 chosen to create realistic filtering (some Dogs included, some excluded)
- Success rate threshold 0.95 is φ⁻² (confidence gate minimum per CYNIC Constitution)
- Observer integration deferred until K15 seam 3 is closed (K15_lint_fixes merged)

---

**Status:** Ready for live testing once observer wired  
**Last Updated:** 2026-05-02
