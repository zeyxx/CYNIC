# Hermes + CCM Validation Run

**Goal:** Collect empirical data on CCM loop behavior after band-aids (pause nightshift + --parallel 2).

**Duration:** 4-6 hours (1-2 Hermes cron cycles)

**Metrics collected every 30 seconds:**
- Total observations (count from /observations endpoint)
- Hermes-specific observations (agent_id contains "hermes")
- Crystals: forming vs crystallized (from /crystals endpoint)
- Dog scores (from /health: avg q_score across Dogs)
- GPU load (llama-server token queue depth)

**What we're validating:**
- [T2 falsified?] Hermes crons ARE producing observations (hermes_obs count > 0)
- [K15 working?] Observations trigger crystal formation (crystals.forming → crystallized)
- [Dogs using crystals?] q_score changes if crystals present in context
- [Contention resolved?] GPU load stays <75% with --parallel 2 + nightshift paused

**Run command:**
```bash
export CYNIC_REST_ADDR="http://127.0.0.1:3030"
export CYNIC_API_KEY="$(cat ~/.cynic-env | grep CYNIC_API_KEY | cut -d= -f2)"
./scripts/monitor-hermes-validation.sh /tmp/hermes-validation
```

**Expected output:**
- `/tmp/hermes-validation/hermes-validation-*.jsonl` (raw metrics, one per 30s)
- Final summary: observation growth, crystal formation, avg dog score

**Falsification criteria:**
- If hermes_obs = 0 after 4h: Hermes crons NOT running (root cause is NOT GPU)
- If hermes_obs > 0 but crystals don't form: K15 violation persists (SurrealDB issue)
- If crystals form but dog_score = constant: Dogs NOT using crystal context
- If gpu_load > 75%: Contention NOT resolved (band-aids insufficient)

**Next step:** Run this, collect data, analyze. Then decide which layer to fix next (K15, MCP-to-REST, Soma).

---
*Created 2026-04-26. Band-aids deployed: nightshift paused, --parallel 2 active.*
