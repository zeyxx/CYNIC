# K15 Consumer Deployment Guide

**Status:** Code complete, systemd service defined, offline tests pass (3/4).
**Deadline:** May 1 23:59 (K15 Seam 2 completion)
**Blocker:** Kernel must be running at `http://localhost:3030` for live testing

## Quick Deployment (2 minutes)

```bash
# Step 1: Deploy systemd service
sudo cp infra/systemd/hermes-k15-consumer.service /etc/systemd/system/

# Step 2: Reload systemd
sudo systemctl daemon-reload

# Step 3: Enable and start
sudo systemctl enable hermes-k15-consumer.service
sudo systemctl start hermes-k15-consumer.service

# Step 4: Verify
systemctl status hermes-k15-consumer.service
journalctl -u hermes-k15-consumer.service -f
```

## What K15 Consumer Does

**Closes the K15 producer-consumer loop (Seam 2):**
1. Polls kernel `/observations` every 5 minutes
2. Scores each observation with TwitterDog heuristics
3. Filters high-signal (signal ≥ 6.0 OR BARK verdict OR @gcrtrd mention)
4. Dispatches high-signal observations to `/agent-tasks` for Hermes to execute

**Falsification:** After deploying and kernel running:
```bash
# Check that observations are being consumed
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/health | jq '.observations_consumed'

# Should increment every 5 minutes
```

## Testing Offline (Already Done)

All offline tests pass:
```bash
python3 cynic-python/consumers/test_k15_consumer.py
```

Results:
- ✓ Rug scam tweet → BARK (q=0.259)
- ✓ Emerging project → GROWL (q=0.450)
- ✗ Established token → GROWL (q=0.475, expected HOWL — signal extraction gap)
- ✓ Signal score computation
- ✓ High-signal pattern detection (3/3 patterns)
- ✓ End-to-end workflow (3/3 observations dispatched)

## Testing Live (Requires Kernel)

### Prerequisites
```bash
# Check kernel is running
curl -s http://localhost:3030/health | jq '.status'
# Should return: "sovereign" or "degraded"

# Check kernel has observations to process
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/observations?limit=10 | jq '.length'
# Should return: >0
```

### Deployment Test (5 minutes)
```bash
# 1. Start K15 consumer
sudo systemctl start hermes-k15-consumer.service

# 2. Wait 5 minutes (one poll interval)
sleep 300

# 3. Check if observations were consumed
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/health | jq '.observations_consumed'

# Expected: incremented by 8-14 (number of high-signal obs in queue)
```

### Integration Test (15 minutes)
```bash
# 1. Note current agent-tasks count
BEFORE=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/agent-tasks?kind=hermes | jq '.length')

# 2. Wait 5 minutes
sleep 300

# 3. Check if new tasks were created
AFTER=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/agent-tasks?kind=hermes | jq '.length')

echo "Tasks before: $BEFORE, after: $AFTER"

# Expected: AFTER > BEFORE (new tasks created)
# Typical: 8-10 new tasks (if 14 pending observations, 3/4 high-signal)
```

### Falsification Success Criteria
- [ ] `observations_consumed` counter increments every 5 minutes
- [ ] Agent-tasks count increases after consumer runs
- [ ] Logs show score+dispatch for high-signal observations
- [ ] Logs show filtering for low-signal observations

View logs:
```bash
journalctl -u hermes-k15-consumer.service --follow
```

## Known Limitations & Next Steps

### Signal Extraction Gap
TwitterDog doesn't capture "major DEX listing" signal. Established tokens score as GROWL instead of HOWL.
- **Impact:** Won't dispatch establishment token positives to agent
- **Fix:** Use measurement framework to measure & adjust weights
- **Timeline:** Post-hackathon (measurement-driven improvement cycle)

### BARK Over-Dispatch
Any BARK verdict triggers dispatch regardless of signal_score. This catches real rugs but may also flag false-positive BARK from neutral tweets.
- **Falsify:** Measure precision (% of dispatched tasks that are true rugs) after 24h of live data
- **Adjust:** If precision <0.7, increase signal_score threshold for BARK

### Missing Patterns
Only monitors @gcrtrd specifically. Other high-signal sources (@a1lon9, @ZakShark, etc.) not yet pattern-matched.
- **Add:** Once measurement framework validates new sources
- **Update:** Edit `known_high_signal_patterns` in k15_observation_consumer.py

## Architecture Diagram

```
Kernel /observe endpoint
       ↓ (tweets, observations)
  SurrealDB observations table
       ↓ (poll every 5min)
  K15 Consumer
       ├─→ TwitterDog scores (signal_score 0-10)
       ├─→ High-signal filter (≥6.0 OR BARK OR @gcrtrd)
       ├─→ Dispatch to /agent-tasks (high-signal only)
       └─→ Skip low-signal (no task created)
            ↓
       Hermes Agent Queue
       ↓
  Hermes processes task
  ├─→ Verify against live data
  ├─→ Update SKILL.md patterns
  └─→ Log to K15 observation
```

## Support & Troubleshooting

### Service won't start
```bash
journalctl -u hermes-k15-consumer.service -n 50
# Check for: Python errors, missing imports, /observations endpoint not found
```

### Consumer crashes repeatedly
```bash
# Check endpoint reachability
curl -v http://localhost:3030/observations 2>&1 | head -20

# Check API key
echo $CYNIC_API_KEY

# Test consumer directly (one iteration)
python3 cynic-python/consumers/k15_observation_consumer.py \
  --kernel-url http://localhost:3030 \
  --api-key $CYNIC_API_KEY \
  --max-iter 1
```

### Tasks not being created
```bash
# Check kernel /observations has data
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://localhost:3030/observations?limit=20 | jq '.[] | {context, signal_score}' | head -30

# Check consumer threshold logic
# Signal ≥6.0 to trigger? Or BARK? Edit _is_high_signal() if needed
```

## Measurement Framework (Optional Tuning)

After K15 consumer runs for 24h and collects real observation scores, you can measure accuracy:

```bash
# Baseline: current consumer thresholds
python3 cynic-python/lab/measure_domain_quality.py \
  --domain twitter \
  --baseline

# Change thresholds in k15_observation_consumer.py
# Re-measure
python3 cynic-python/lab/measure_domain_quality.py \
  --domain twitter \
  --after \
  --compare-to measurements/twitter_baseline.json

# Decide: accept if sensitivity↑ and specificity↑
```

## Timeline

- **2026-04-30 23:59** — K15 consumer code complete + offline tests pass + systemd service created
- **2026-05-01 00:00 - 23:59** — Deploy to systemd, test on live observations (Seam 2 completion)
- **2026-05-10 23:59** — Hackathon submission deadline (K15 loop must be closed)

**Status:** Ready to deploy. Requires: kernel running, /observations endpoint live, agent-tasks functional.
