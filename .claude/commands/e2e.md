---
name: e2e
description: Run an end-to-end test against the running CYNIC kernel.
disable-model-invocation: true
allowed-tools: Bash(curl *) Bash(source *)
---

Run an end-to-end test against the running CYNIC kernel.

Steps:
1. `source ~/.cynic-env 2>/dev/null`
2. Check kernel is running: `curl -s http://${CYNIC_REST_ADDR}/health`
3. If not running, tell user to run `/run` first
4. Submit a test judgment:
```
curl -s -X POST http://${CYNIC_REST_ADDR}/judge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -d '{"content": "e4 e5 Nf3 Nc6 Bb5 — the Ruy Lopez opening", "context": "Classical chess opening theory", "domain": "chess"}'
```
5. Parse the verdict: show Q-score, verdict kind (HOWL/WAG/GROWL/BARK), per-axiom scores, dogs used
6. If argument provided, use it as the content instead of the default chess example
