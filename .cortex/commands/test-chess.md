---
name: test-chess
description: Run 3 chess test cases against the live kernel and display results.
disable-model-invocation: true
allowed-tools: Bash(curl *) Bash(source *)
---

Run 3 chess test cases against the live kernel and display results.

First: `source ~/.cynic-env 2>/dev/null`

1. POST /judge with Sicilian Defense (good opening) — expect Howl or high Wag
2. POST /judge with Scholar's Mate (beginner trap) — expect Growl or low Wag
3. POST /judge with Fool's Mate (worst opening) — expect Bark or low Growl

For each, show: verdict, q_score.total, per-axiom scores, dogs_used, and first 60 chars of fidelity reasoning.

Base URL: http://${CYNIC_REST_ADDR}
Auth header: Authorization: Bearer ${CYNIC_API_KEY}

Test data:
- Sicilian: content="1. e4 c5 — The Sicilian Defense. Black fights for the center asymmetrically.", context="Most popular response to 1.e4 at grandmaster level", domain="chess"
- Scholar: content="1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# — Scholar's Mate.", context="Beginner trap that fails against prepared opponents", domain="chess"
- Fool: content="1. f3 e5 2. g4 Qh4# — Fool's Mate. Worst possible opening.", context="No grandmaster would ever play this", domain="chess"
