Frontend development helper for cynic-ui. Use when building or modifying the CYNIC dashboard.

IMPORTANT: Read `cynic-ui/CLAUDE.md` first — it has the full context, real score ranges, and color scheme.
Also read `API.md` for the complete REST contract with TypeScript interfaces.

Context:
- CYNIC kernel: http://<TAILSCALE_UBUNTU>:3030 (Tailscale) — CORS allows all origins
- API endpoints:
  - GET /health — {"status":"sovereign","phi_max":0.618,...}
  - POST /judge — submit content for chess evaluation, returns Verdict with 6 axiom scores
  - GET /verdicts — list recent verdicts
  - GET /verdict/:id — get specific verdict

Key data structures:
- Verdict: { verdict_id, verdict (Howl/Wag/Growl/Bark), q_score: {total, fidelity, phi, verify, culture, burn, sovereignty}, reasoning: {per-axiom text}, dog_scores[], anomaly_detected, anomaly_axiom }
- All scores are 0.0-0.618 (phi-bounded). This is structural, NOT a bug.
- Verdict thresholds on q_score.total: HOWL >= 0.5207, WAG >= 0.382, GROWL >= 0.236, BARK < 0.236

CYNIC judges SUBSTANCE not FORM — in chess, it judges the strategy quality, not the text.
Real scores: Sicilian Defense → Howl (high scores), Fool's Mate → Bark (all zeros).

Frontend stack: React 18 + TypeScript + Vite (cynic-ui/)
- react-chessboard + chess.js for interactive board
- Recharts or similar for axiom radar/bar chart
- Dark theme, golden accents (#C9A84C)

Use context7 plugin to fetch latest docs for react-chessboard, chess.js, recharts.

Colors:
- Howl: #FFD700 (gold)
- Wag: #2196F3 (blue)
- Growl: #FF9800 (orange)
- Bark: #F44336 (red)
