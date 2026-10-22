# C5-4 execution log — 2026-10-22 (build GC timer on CT105)

Branch: `agent/hermes/c54-build-gc`. Approval: operator option 2 (deploy the real GC).

## Deployed: DONE

- Source: `infra/os/systemd/soak/cynic-build-artefact-gc.{service,timer}` @ `origin/main` (PR #224, commit `b5360cd1`), copied verbatim (scp to /tmp, `install -m 644`, tmp copies removed).
- `systemctl daemon-reload`, `systemctl enable --now cynic-build-artefact-gc.timer` as root on CT105.
- Read-back: `list-timers` shows `cynic-build-artefact-gc.timer`, next run 2026-10-23 00:19 UTC.
- Pre-deploy dry run (read-only, as `cynic`): `cynic runtime build artefact gc --json` parses, schema `cynic.retention_observation.v1`.

## Measured limits (honest correction to the plan)

- The GC observes ONLY `/var/lib/cynic/checkouts` (8 items, 3 orphans, ~2 GB reclaimable today — down from 7.27 GB at PR time).
- It does NOT cover `/tmp/*target` per-job cargo dirs (23 GB: pooltarget 6.1, cynic-ext 3.6, convtarget 3.4, retire-target 2.7, retentiontarget 2.0, ...). Those are the actual blocker for the CT105 vzdump, and this timer will not free them.
- Freeing `/tmp` needs either a manual purge (artefacts regenerable, one host command) or a GC policy extension (code change + builder proof — not today).

## Exit criteria status

- [x] CT105 has the GC timer, enabled, next run scheduled.
- [ ] CT105 disk pressure resolved (blocked on /tmp decision above).
- [ ] CT105 vzdump fits (blocked on the same).
