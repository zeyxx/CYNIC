# CYNIC Organism Topology (auto-generated 2026-06-03 23:46 UTC)

## Active Modules

| Module | Domain | Tier | Status | Crons | Key Outputs |
|--------|--------|------|--------|-------|-------------|
| cynic-kernel | core | 1 | ACTIVE | cynic-kernel (daemon), kairos-health (every 15min), kairos-backfill (every 1h), kairos-archive (daily at 03:00), surrealdb-backup (daily at 00:00), surreal-watchdog (every 1min), cynic-telemetry-digest (daily at 07:00) | REST /health, REST /verdicts, REST /crystals (+4) |
| mirror-agent | behavior | 2 | INACTIVE | mirror-agent (daemon) | profiles, segments, kernel /observe (domain=mirror-lifecycle) |
| hermes-github-organ | social | 2 | ACTIVE | none | kernel /observe (domain=github), github |
| hermes-x-organ | social | 2 | ACTIVE | hermes-data-organism (hourly at :17) | reflections, kernel /observe (domain=hermes-x) |
| hermes-x-scripts | social | 2 | ACTIVE | hermes-x-ingest (daemon), hermes-curation (every 30min at :03/:33), hermes-search-generator (every 15min at :02/:17/:32/:47), hermes-feedback-loop (every 1h), hermes-k15-consumer (every 10min), hermes-gemini-briefing (every 4h) | kernel /observe (domain=twitter), domain signals, SKILL (+1) |
| telegram-organ | social | 2 | active | none | data,  |
| token-data-collection | token-analysis | 2 | ACTIVE | token-snapshot (06:00 UTC daily) | holders date, metadata date, market snapshot date (+1) |
| token-data-store | token-analysis | 2 | ACTIVE | none | snapshots, market snapshots, token profiles (+4) |
| ab-test-enrichment-2026-05-16 | token-analysis | ? | CONSUMED | none |  |
| calibration-pipeline-2026-05-16 | token-analysis | ? | ACTIVE | none |  |
| ccm-discrimination-2026-05-17 | token-analysis | ? | CONSUMED | none |  |
| conviction-temporal-2026-05-16 | token-analysis | ? | CONSUMED | none |  |
| kscore-composite-2026-05-16 | token-analysis | ? | DEAD | none |  |
| meta-question-2026-05-17 | token-analysis | ? | ACTIVE | none |  |

## Data Flow (producer -> consumer)

- **cynic-kernel** -> Dog prompts (CCM), K15 consumer, all kernel queries, crystal pipeline, domain wisdom, experiments, feedback loop, health monitors, hermes agent task executor, hermes data organism, kairos, multi-cortex isolation, session-init.sh
- **hermes-github-organ** -> K15 observation consumer, gemini briefing
- **hermes-x-organ** -> K15 observation consumer, gemini briefing, hermes learning
- **hermes-x-scripts** -> K15 observation consumer, agent task executor, gemini briefing, hermes agent task executor, search generator, verdict pipeline
- **mirror-agent** -> kernel health monitoring, session analysis, zone dispatch, zone dispatch (L0→L1 bridge)
- **token-data-collection** -> benchmark analysis, conviction computation, outcome collector, session analysis, trajectory classification, unified profiles
- **token-data-store** -> Dog calibration, Dog stimuli, benchmark, conviction, deterministic dog threshold tuning, experiments, ground truth validation, outcome collector, statistical baseline for scoring, trajectory, unified profiles

## Verification Issues

- [warning] cynic-kernel: no files match SurrealDB tables
- [warning] token-data-store: no files match data/snapshots/
- [warning] token-data-store: no files match data/market_snapshots/
- [warning] token-data-store: no files match data/token_profiles.jsonl
- [warning] token-data-store: no files match data/calibration_results_real.json
- [warning] token-data-store: no files match data/population_stats.json
- [warning] hermes-github-organ: no files match ~/.cynic/organs/hermes/github/
- [warning] hermes-x-organ: no files match ~/.cynic/organs/hermes/x/reflections/
- [warning] telegram-organ: no files match ~/.cynic/organs/telegram/data.db
- [warning] hermes-x-scripts: no files match ~/.cynic/organs/hermes/x/domain_signals/

## Experiments

| ID | Status | Hypothesis (truncated) |
|----|--------|----------------------|
| ab-test-enrichment-2026-05-16 | CONSUMED | Divergence enrichment (buy/sell ratio + class) improves token judgment accuracy. |
| calibration-pipeline-2026-05-16 | ACTIVE | CultScreener labels + Helius enrichment can calibrate the deterministic dog to > |
| ccm-discrimination-2026-05-17 | CONSUMED | Dogs CAN discriminate legit vs scam tokens IF given enriched on-chain stimuli (n |
| conviction-temporal-2026-05-16 | CONSUMED | Conviction is temporal — no single on-chain snapshot captures holder commitment. |
| kscore-composite-2026-05-16 | DEAD | A composite K-Score combining multiple on-chain signals can predict token qualit |
| meta-question-2026-05-17 | ACTIVE | CYNIC Dogs (Qwen 7B + enrichment + crystals) produce better token verdicts than  |
