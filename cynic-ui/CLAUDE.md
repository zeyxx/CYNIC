# CYNIC Frontend — cynic-ui

You are building the CYNIC chess judgment dashboard. Read this FIRST, then `../API.md` for the full REST contract.

## What CYNIC Does

CYNIC is an epistemic immune system. Multiple independent AI validators ("Dogs") evaluate content through 6 philosophical axioms and reach consensus under mathematical doubt. Max confidence = 61.8% (golden ratio inverse). This is structural, NOT a bug.

## Your Mission (Hackathon Demo)

A chess page where:
1. Interactive chessboard (react-chessboard + chess.js)
2. Each move → `POST /judge` with the position/move as content, domain="chess"
3. Verdict display: Howl (gold), Wag (blue), Growl (orange), Bark (red)
4. Radar chart or bar chart of 6 axiom scores
5. Q-Score displayed prominently (0.0–0.618 range)
6. Per-axiom reasoning text from the Dogs
7. Health indicator dot (GET /health)
8. Dog comparison — show when Dogs disagree (anomaly_detected = discovery signal)

## API

**Base URL (Tailscale):** `http://<TAILSCALE_UBUNTU>:3030`
**Base URL (Public tunnel):** `https://associations-mailed-treasury-component.trycloudflare.com`

Use the public tunnel URL for Replit. Use Tailscale URL for local dev.

```
POST /judge     → {"content": "1. e4 c5 — Sicilian Defense", "context": "optional", "domain": "chess"}
GET  /health    → {"status": "sovereign", "phi_max": 0.618, ...}
GET  /verdicts  → [array of past verdicts]
```

Full contract with TypeScript interfaces: `../API.md`

## Real Score Ranges (tested)

| Position | Verdict | Score Range |
|---|---|---|
| Sicilian Defense (great) | Howl | fidelity=0.75, phi=0.40, verify=0.85 |
| Scholar's Mate (trap) | Growl | fidelity=0.20, phi=0.20, verify=0.10 |
| Fool's Mate (worst) | Bark | fidelity=0.00, all=0.00 |

**CYNIC judges the STRATEGY, not the text.** Scores vary dramatically — don't hardcode placeholder values.

## 6 Axioms

| Axiom | Key | Color | Icon |
|---|---|---|---|
| FIDELITY | fidelity | Shield blue | Shield |
| PHI | phi | Gold | Spiral |
| VERIFY | verify | Green | Magnifying glass |
| CULTURE | culture | Purple | Temple |
| BURN | burn | Orange/flame | Flame |
| SOVEREIGNTY | sovereignty | Crown gold | Crown |

## Verdict Colors

```typescript
const VERDICT_COLORS: Record<string, string> = {
  Howl: '#FFD700',   // gold — exceptional
  Wag: '#2196F3',    // blue — good
  Growl: '#FF9800',  // orange — questionable
  Bark: '#F44336',   // red — rejected
};
```

## Stack

- React 18 + TypeScript + Vite
- react-chessboard + chess.js for the board
- Recharts or similar for axiom visualization
- Dark theme, golden accents (#C9A84C for phi/golden ratio)

## Skills & Plugins to Use

- Use `context7` plugin to fetch latest docs for react-chessboard, chess.js, recharts
- Use `/frontend-dev` skill for CYNIC-specific frontend patterns
- CORS: backend allows all origins — no proxy needed

## Git Rules

- Only touch `cynic-ui/` — NEVER modify `cynic-kernel/` or root docs
- `git pull --rebase` before every push
- Commits: `type(ui): description`
