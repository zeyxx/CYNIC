# CYNIC V2 — Epistemic Immune System

## Project Overview

CYNIC is an epistemic immune system — multiple independent AI validators ("Dogs") evaluate content through 6 philosophical axioms and reach consensus under mathematical doubt. Maximum confidence is bounded at 61.8% (golden ratio inverse).

Built for the Gemini 3 Paris Hackathon (2026-03-14). Demo domain: Chess — each move is judged through 6 philosophical axioms.

## Architecture

```
cynic-kernel/    → Rust backend (Axum REST + MCP)
cynic-ui/        → React + TypeScript + Vite frontend
```

### Backend (cynic-kernel)
- **Language:** Rust (edition 2024)
- **Framework:** Axum (REST) + rmcp (MCP). Tonic gRPC is feature-gated off.
- **REST Port:** 3030 (or $CYNIC_REST_ADDR env var)
- **gRPC Port:** [::1]:50051 (feature-gated, off by default)
- **Storage:** SurrealDB via HTTP adapter (graceful degradation if unavailable)
- **Build:** `cargo build -p cynic-kernel --release`

### Frontend (cynic-ui)
- **Language:** TypeScript + React 18
- **Bundler:** Vite 8
- **Port:** 5000 (dev server, 0.0.0.0 host)
- **Libraries:** react-chessboard, chess.js, recharts
- **API base:** Cloudflare tunnel URL (see `cynic-ui/src/types.ts`)

## Key Features

- Interactive chessboard — drag-and-drop moves
- Each move → POST /judge → CYNIC verdict
- Verdict display: Howl (gold), Wag (blue), Growl (orange), Bark (red)
- Radar chart of 6 axiom scores (FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY)
- Q-Score display (0.0–0.618 range)
- Health indicator (green dot = SOVEREIGN)
- Verdict history tab

## The 6 Axioms

| Axiom | Measures |
|-------|----------|
| FIDELITY | Truth loyalty |
| PHI | Structural harmony (golden ratio) |
| VERIFY | Falsifiability |
| CULTURE | Pattern continuity |
| BURN | Simplicity/efficiency |
| SOVEREIGNTY | Individual agency |

## Verdict Thresholds

```
HOWL > 0.528     (exceptional — φ⁻²+φ⁻⁴ golden subdivision)
WAG  > 0.382     (good — φ⁻²)
GROWL > 0.236    (questionable — φ⁻³)
BARK ≤ 0.236     (rejected)
```

## Workflow

- **"Start application"** — runs `cd cynic-ui && npm run dev` on port 5000 (webview)

## Deployment

- Configured as static site deployment
- Build: `cd cynic-ui && npm run build`
- Public dir: `cynic-ui/dist`

## Environment Notes

- Node.js 20 installed
- Rust stable (1.88.0)
- No database configured locally — backend uses SurrealDB with graceful degradation
- Backend API is remote (Cloudflare tunnel / Tailscale) — not running locally in this Repl

## File Structure

```
cynic-kernel/          Rust backend source
  src/
    main.rs            Boot sequence, REST server (gRPC feature-gated)
    rest.rs            Axum REST routes (/health, /judge, /verdicts, /verdict/:id)
    judge.rs           Multi-dog consensus engine
    dog.rs             Dog trait (independent validator)
    deterministic_dog.rs  Heuristic validator (always available)
    inference_dog.rs   LLM-backed validator
    backend_openai.rs  OpenAI-compatible backend adapter
    config.rs          Backend config loading (backends.toml / env vars)
    storage_http.rs    SurrealDB HTTP adapter

cynic-ui/              React frontend
  src/
    types.ts           TypeScript interfaces and constants
    api.ts             API client functions
    App.tsx            Main app with tab navigation
    components/
      ChessJudge.tsx   Interactive chess board with judging
      VerdictDisplay.tsx  Verdict + axiom charts + reasoning
      AxiomChart.tsx   Radar chart and bar chart components
      VerdictHistory.tsx  Verdict history browser
      HealthIndicator.tsx  Live health status dot
```
