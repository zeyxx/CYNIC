# Organism Health Monitoring System

**Purpose:** Keep MEMORY.md synchronized with reality. Automatically detect infrastructure drift and alert.

**Architecture:**

```
Tier 1 (Free): organism-probe.sh
  └─ Runs every 30min via cron
  └─ Gathers: git status, kernel health, hermes crons, llama-server, system metrics
  └─ Output: /tmp/organism_state.json

Tier 2 (Haiku, $0.004/trigger): organism-health-cron.sh
  └─ Compares current vs previous state
  └─ Threshold: only trigger if divergence > 200 bytes (noise filter)
  └─ Writes marker: /tmp/organism_health_trigger.txt

Tier 3 (Human): /memory-sync skill
  └─ Manual full audit (before demo, session-end)
  └─ Updates MEMORY.md with findings
  └─ Cost: ~$0.004 Haiku synthesis
```

## Setup

### 1. Install Cron Job (every 30 minutes)

```bash
chmod +x /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/organism-probe.sh
chmod +x /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/organism-health-cron.sh

# Add to crontab:
(crontab -l 2>/dev/null; echo "*/30 * * * * /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/organism-health-cron.sh >> /var/log/organism-health.log 2>&1") | crontab -
```

### 2. Verify Cron Status

```bash
crontab -l | grep organism
tail -20 /var/log/organism-health.log
cat /tmp/organism_state.json | jq .
```

### 3. Test Manual Audit

```bash
/memory-sync
```

(This will run organism-probe + Haiku synthesis + update MEMORY.md)

## State Schema

### `/tmp/organism_state.json`

```json
{
  "timestamp": "2026-05-01T19:45:00Z",
  "git": {
    "branch": "feat/wallet-behavior",
    "last_commit": "a67bfd73",
    "dirty_files": 3
  },
  "kernel": {
    "running": true,
    "pid": "28451",
    "address": "http://<TAILSCALE_CORE>:3030",
    "health": { "dogs_healthy": 5, "dogs_total": 6, ... }
  },
  "hermes": {
    "cron_timers": 4,
    "systemd_services": 4
  },
  "llama_server": {
    "running": true
  },
  "system": {
    "memory_top3": [
      { "cmd": "python", "mem_mb": 1506 },
      { "cmd": "node", "mem_mb": 647 }
    ]
  }
}
```

## Integration with Claude Code

When `/tmp/organism_health_trigger.txt` is created:
1. Claude Code reads the trigger file
2. Dispatches `organism-health-monitor` agent
3. Agent synthesizes divergence + updates memory
4. Removes trigger file

**Cost per month:**
- If all divergence = noise: $0.12-0.25 (probe cost only)
- If real divergence 2-3x/month: $0.15-0.30 (add Haiku cost)

## Examples

### Example 1: Kernel Restart Detected

```
Previous: kernel.running = true, pid = "28451", health.dogs_healthy = 6
Current:  kernel.running = true, pid = "28523", health.dogs_healthy = 5

Divergence: Kernel restarted (PID changed), one dog lost
Action: Haiku updates project_status with incident timestamp
Memory: "Kernel restart 2026-05-01T19:45Z, 1 dog recovery pending"
```

### Example 2: Git Dirty Files Persistent

```
Previous: git.dirty_files = 0
Current:  git.dirty_files = 3 (validation_corpus_phase2.json, config.toml, .env)

Divergence: New uncommitted work detected
Action: No Haiku trigger (expected, in-progress work)
        Only alert if dirty_files > 10 or persist > 4 hours
```

### Example 3: Hermes Cron Down

```
Previous: hermes.cron_timers = 4
Current:  hermes.cron_timers = 2

Divergence: 2 hermes crons stopped
Action: Haiku triggered, alerts: "Hermes-X crons degraded (2/4)"
Memory: "hermes-x-status.md → cron down incident 2026-05-01T19:45Z, blocked: domain exploration"
```

## Noise Filtering

Threshold logic (200 bytes):
- **Noise examples (< 200 bytes):**
  - Single process memory change (top3 processes)
  - One kernel health dog transient
  - git status delta (1-2 dirty files added)

- **Signal examples (> 200 bytes):**
  - Kernel restart (PID + health dict changes)
  - Hermes service count changed (impacts state)
  - Multiple git dirty files + kernel health change

## Monitoring Dashboard

Check health status anytime:

```bash
# Last probe
cat /tmp/organism_state.json | jq .

# Historical probes
tail -50 /var/log/organism-health.log

# Manual audit
/memory-sync
```

## Troubleshooting

### Probe fails (can't reach kernel)

```bash
# Check kernel is running
ps aux | grep cynic-kernel

# Check address is correct
echo $CYNIC_REST_ADDR

# Manually test
curl http://<TAILSCALE_CORE>:3030/health
```

### Cron not running

```bash
crontab -l | grep organism  # verify installed
sudo service cron status    # check cron daemon
sudo /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/organism-health-cron.sh  # test manually
```

### Memory sync not triggering

```bash
# Check trigger file was created
ls -la /tmp/organism_health_trigger.txt

# Check Haiku can be dispatched
# (depends on Claude Code agent dispatch integration)
```

## Future Enhancements

- [ ] Grafana dashboard integration (push state to metrics)
- [ ] Slack alerts on critical drift
- [ ] Org-level dashboards (all organs: hermes-x, cynic-core, cynic-gpu)
- [ ] Predictive alerts (e.g., "memory trending up 5MB/h")
