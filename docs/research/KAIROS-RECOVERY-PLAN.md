# KAIROS Recovery Plan — 2026-04-13

## Status
- **Last Ingestion**: 2026-03-23
- **Current State**: STALE (Candles frozen, enriched signals using placeholders)
- **Root Cause**: `kairos-hl-stream.service` and `kairos-hl-snapshot.service` are inactive or crashing silently.

## Recovery Procedure

### Phase 1: Diagnostic
1. Check for zombie processes:
   ```bash
   ps aux | grep hl_collector
   ```
2. Inspect last logs:
   ```bash
   journalctl --user -u kairos-hl-stream -n 100
   ```

### Phase 2: Database Integrity
The `hl_collector` uses `CREATE TEMP TABLE` patterns. Ensure the TimescaleDB user has sufficient permissions and that the connection pool isn't exhausted.
- Check SurrealDB/TimescaleDB connectivity from the KAIROS environment.

### Phase 3: Data Backfill
Comply with the "no gaps" policy. Run the historical collector first:
```bash
cd /home/user/Bureau/KAIROS
/home/user/.cargo/bin/uv run python -m ingestion.hl_collector backfill
```

### Phase 4: Live Stream Resumption
Restart the systemd units:
```bash
systemctl --user daemon-reload
systemctl --user start kairos-hl-snapshot.service
systemctl --user start kairos-hl-stream.service
```

### Phase 5: Verification
Check CYNIC health or logs for new trading verdicts:
```bash
# In CYNIC
GET /health
# Expected: "trading" domain shows recent observations
```

## Future Improvements (Post-Recovery)
- Implement a **Liveness Probe** in CYNIC that alerts if no trading observations are received for > 15 minutes.
- Move the `_enrich_signals` logic to a more robust error-handling pattern (detailed logging of HTTP failures to Hyperliquid).
