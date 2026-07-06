# CYNIC Organism Topology (auto-generated 2026-06-27 00:32 UTC)

## Active Modules

| Module | Domain | Tier | Status | Crons | Key Outputs |
|--------|--------|------|--------|-------|-------------|
| organ-supervisor | body-governance | 1 | ACTIVE | none | SSOT, supervision, action-queue |
| cynic-kernel | core | 1 | ACTIVE | cynic-kernel (daemon), kairos-health (every 15min), kairos-backfill (every 1h), kairos-archive (daily at 03:00), cynic-telemetry-digest (daily at 07:00) | REST /health, REST /verdicts, REST /crystals (+4) |
| organ-jupyter | environment | 2 | ACTIVE | none | kernel /observe (domain=jupyter), dataset |
| organ-forge | infrastructure | 2 | ACTIVE | none | kernel /observe (domain=forge), dataset |
| organ-exposure | organism/exposure | 2 | ACTIVE | none | stdout, REST future kernel /observe |
| organ-percolator | security-bounty | 2 | declared | none | SSOT, infer-queue |
| organ-slack | slack | 2 | ACTIVE | none | REST Slack API (chat.postMessage), kernel /observe (domain=slack, status=delivered|failed) |
| hermes-x-scripts | social | 2 | legacy_blocked | none |  |
| organ-discord | social | 2 | ACTIVE | none | messages, 127.0.0, kernel /observe (domain=discord) |
| organ-telegram | social | 2 | ACTIVE | none | messages, kernel /observe (domain=telegram) |
| organ-twitter | social | 2 | ACTIVE | none | latest, latest, reflections (+4) |
| organ-keep | storage | 2 | planned | none |  |
| organ-reflection | synthesis | 2 | ACTIVE | none | askesis, kernel /observe (domain=reflection) |
| organ-browser | tech-watch/browser | 2 | ACTIVE | none | kernel /observe, evidence, action ledger |
| token-data-collection | token-analysis | 2 | ACTIVE | token-snapshot (06:00 UTC daily) | holders date, metadata date, market snapshot date (+1) |
| token-data-store | token-analysis | 2 | ACTIVE | none | snapshots, market snapshots, token profiles (+4) |
| organ-kanban-ui | presentation | 3 | ACTIVE | none | board |
| ab-test-enrichment-2026-05-16 | token-analysis | ? | CONSUMED | none |  |
| calibration-pipeline-2026-05-16 | token-analysis | ? | ACTIVE | none |  |
| ccm-discrimination-2026-05-17 | token-analysis | ? | CONSUMED | none |  |
| conviction-temporal-2026-05-16 | token-analysis | ? | CONSUMED | none |  |
| kscore-composite-2026-05-16 | token-analysis | ? | DEAD | none |  |
| meta-question-2026-05-17 | token-analysis | ? | ACTIVE | none |  |

## Data Flow (producer -> consumer)

- **cynic-kernel** -> Dog prompts (CCM), K15 consumer, all kernel queries, crystal pipeline, domain wisdom, experiments, feedback loop, health monitors, hermes agent task executor, hermes data organism, kairos, multi-cortex isolation, session-init.sh
- **token-data-collection** -> benchmark analysis, conviction computation, outcome collector, session analysis, trajectory classification, unified profiles
- **token-data-store** -> Dog calibration, Dog stimuli, benchmark, conviction, deterministic dog threshold tuning, experiments, ground truth validation, outcome collector, statistical baseline for scoring, trajectory, unified profiles

## Experiments

| ID | Status | Hypothesis (truncated) |
|----|--------|----------------------|
| ab-test-enrichment-2026-05-16 | CONSUMED | Divergence enrichment (buy/sell ratio + class) improves token judgment accuracy. |
| calibration-pipeline-2026-05-16 | ACTIVE | CultScreener labels + Helius enrichment can calibrate the deterministic dog to > |
| ccm-discrimination-2026-05-17 | CONSUMED | Dogs CAN discriminate legit vs scam tokens IF given enriched on-chain stimuli (n |
| conviction-temporal-2026-05-16 | CONSUMED | Conviction is temporal — no single on-chain snapshot captures holder commitment. |
| kscore-composite-2026-05-16 | DEAD | A composite K-Score combining multiple on-chain signals can predict token qualit |
| meta-question-2026-05-17 | ACTIVE | CYNIC Dogs (Qwen 7B + enrichment + crystals) produce better token verdicts than  |
