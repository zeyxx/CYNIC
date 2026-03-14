Run an end-to-end test against the running CYNIC kernel.

Steps:
1. Check kernel is running: `curl -s http://localhost:3030/health`
2. If not running, tell user to run `/run` first
3. Submit a test evaluation:
```
curl -s -X POST http://localhost:3030/evaluate \
  -H "Content-Type: application/json" \
  -d '{"content": "e4 e5 Nf3 Nc6 Bb5 — the Ruy Lopez opening", "context": "Classical chess opening theory", "domain": "chess"}'
```
4. Parse the verdict: show Q-score, verdict kind (HOWL/WAG/GROWL/BARK), per-axiom scores, anomaly status
5. If argument provided, use it as the content instead of the default chess example
