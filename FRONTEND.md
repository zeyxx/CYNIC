<!-- AI-CONTEXT
role: frontend-developer-onboarding
audience: S. + his AI coding assistant (Claude Code / Gemini CLI)
purpose: everything needed to set up, build, and ship the CYNIC frontend during hackathon
priority: read this FIRST, then API.md for endpoint details, then HACKATHON-RULES.md for constraints
cross-refs: [API.md, HACKATHON-RULES.md, CLAUDE.md]
machine: desktop-mc9ffvt (Windows, Tailscale IP <TAILSCALE_STANISLAZ>)
backend: http://<TAILSCALE_UBUNTU>:3030 (T.'s Ubuntu machine via Tailscale)
-->

# Frontend Developer Guide — S.

## Git Workflow — CRITICAL

<!-- AI: enforce this workflow for every git operation -->

```yaml
ownership:
  frontend-dev:
    - cynic-ui/**          # frontend — your territory, full ownership
  backend-dev:
    - cynic-kernel/**      # backend — his territory, do not modify
  frozen_after_tonight:
    - API.md               # do not modify during hackathon
    - HACKATHON-RULES.md   # do not modify during hackathon
    - FRONTEND.md         # do not modify during hackathon
    - CLAUDE.md            # do not modify during hackathon
    - scripts/**           # do not modify during hackathon

workflow:
  branch: main             # single branch, no PRs (velocity over ceremony)
  rule: "never modify files outside your ownership zone"
  sync: "git pull --rebase before every push"
  conflict_prevention: "separate directories = zero conflicts"
  commit_style: "type(scope): description"
  commit_examples:
    - "feat(ui): add judge page with verdict display"
    - "fix(ui): handle API timeout gracefully"
    - "style(ui): verdict color coding for HOWL/WAG/GROWL/BARK"
```

---

## Quick Setup (10 min)

### Step 1 — Clone/Pull

```powershell
git clone git@github.com:zeyxx/CYNIC.git   # first time only
cd CYNIC
git pull origin main
```

### Step 2 — Prerequisites

```powershell
node --version    # required: >= 18
npm --version     # required: >= 9
```

If missing: install from https://nodejs.org/ (LTS version).

### Step 3 — Tailscale

Tailscale must be connected. Verify:

```powershell
tailscale status
```

Expected: you see `ubuntu` at `<TAILSCALE_UBUNTU>` in the list.

### Step 4 — Test Backend

```powershell
curl.exe http://<TAILSCALE_UBUNTU>:3030/health
```

Expected response:

```json
{"status":"sovereign","version":"0.1.0","phi_max":0.618033988749895,"axioms":["FIDELITY","PHI","VERIFY/FALSIFY","CULTURE","BURN","SOVEREIGNTY"]}
```

If unreachable: tell T.. It's a server-side issue.

### Step 5 — Scaffold Frontend

```powershell
npm create vite@latest cynic-ui -- --template react-ts
cd cynic-ui
npm install
npm run dev
```

---

## Architecture

```
┌─────────────────────────────────┐
│  cynic-ui (React + Vite + TS)   │  ← S. builds this
│  Runs on: localhost:5173        │
└──────────┬──────────────────────┘
           │ HTTP (JSON)
           ▼
┌─────────────────────────────────┐
│  CYNIC Kernel (Rust/Axum)       │  ← T. maintains this
│  Runs on: <TAILSCALE_UBUNTU>:3030    │
│  Connected via: Tailscale       │
└─────────────────────────────────┘
```

---

## API Summary

**Base URL:** `http://<TAILSCALE_UBUNTU>:3030`
**Full reference:** see `API.md` in repo root

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/health` | System status check |
| POST | `/judge` | Submit content for evaluation → returns verdict |
| GET | `/verdicts` | List recent verdicts (max 20) |
| GET | `/verdict/{id}` | Get specific verdict by ID |

### POST /judge — Primary Endpoint

**Request:**

```json
{
  "content": "The earth is flat",
  "context": "Scientific claim",
  "domain": "science"
}
```

`context` and `domain` are optional.

**Response:**

```json
{
  "verdict_id": "90fa5ed3-a92f-472c-96ae-6ca08f347346",
  "verdict": "Bark",
  "q_score": {
    "total": 0.1842,
    "fidelity": 0.15,
    "phi": 0.20,
    "verify": 0.10,
    "culture": 0.25,
    "burn": 0.30,
    "sovereignty": 0.22
  },
  "reasoning": {
    "fidelity": "Contradicts established scientific evidence",
    "phi": "Lacks structural coherence with observable reality",
    "verify": "Easily falsified by circumnavigation, satellite imagery",
    "culture": "Contradicts centuries of scientific consensus",
    "burn": "Simple claim but simplicity doesn't redeem falsehood",
    "sovereignty": "Does not restrict agency but promotes misinformation"
  },
  "dogs_used": "deterministic-dog+gemini",
  "phi_max": 0.618033988749895,
  "dog_scores": [
    {
      "dog_id": "deterministic-dog",
      "fidelity": 0.20, "phi": 0.25, "verify": 0.15,
      "culture": 0.30, "burn": 0.45, "sovereignty": 0.40,
      "reasoning": {"fidelity": "Heuristic: ...", "phi": "...", "verify": "...", "culture": "...", "burn": "...", "sovereignty": "..."}
    },
    {
      "dog_id": "gemini",
      "fidelity": 0.10, "phi": 0.15, "verify": 0.05,
      "culture": 0.20, "burn": 0.15, "sovereignty": 0.05,
      "reasoning": {"fidelity": "...", "phi": "...", "verify": "...", "culture": "...", "burn": "...", "sovereignty": "..."}
    }
  ],
  "anomaly_detected": false,
  "max_disagreement": 0.15,
  "anomaly_axiom": null
}
```

---

## What to Build

### Priority 1 — Must Have (for demo)

1. **Judge Page** — single page app
   - Textarea input for content
   - Optional fields: context, domain
   - "Judge" button → `POST /judge`
   - Verdict display with color coding:
     - **HOWL** (q_score.total >= 0.5207): gold/green — highest quality
     - **WAG** (>= 0.382): blue/teal — good
     - **GROWL** (>= 0.236): orange — questionable
     - **BARK** (< 0.236): red — rejected
   - Q-Score total displayed prominently
   - 6 axiom scores as radar chart or bar chart
   - Reasoning text for each axiom

2. **Health indicator** — small status dot
   - Green if `GET /health` returns `"sovereign"`
   - Red if unreachable

### Priority 2 — Should Have (if time)

3. **Verdict history** — `GET /verdicts`
   - List of recent evaluations
   - Click to expand details

4. **Dog comparison view**
   - Side-by-side: deterministic-dog vs gemini scores
   - Highlight when `anomaly_detected: true` (dogs disagree = discovery signal)

### Priority 3 — Nice to Have

5. Animations on verdict reveal
6. `anomaly_axiom` highlighted when dogs disagree
7. Dark theme with phi-golden accents

---

## Key Concepts

### Verdicts

```typescript
type VerdictKind = 'Howl' | 'Wag' | 'Growl' | 'Bark';

const VERDICT_COLORS: Record<VerdictKind, string> = {
  Howl: '#FFD700',  // gold
  Wag:  '#2196F3',  // blue
  Growl:'#FF9800',  // orange
  Bark: '#F44336',  // red
};

const VERDICT_THRESHOLDS = {
  Howl:  0.5207,  // 82% of phi
  Wag:   0.382,   // phi^-2
  Growl: 0.236,   // phi^-3
  Bark:  0,       // below all thresholds
};
```

### The 6 Axioms

| Axiom | Measures | Icon |
|-------|----------|------|
| FIDELITY | Truth loyalty | Shield |
| PHI | Structural harmony (golden ratio) | Spiral |
| VERIFY | Falsifiability — can it be proven wrong? | Magnifying glass |
| CULTURE | Pattern continuity with traditions | Temple |
| BURN | Simplicity, efficiency, no waste | Flame |
| SOVEREIGNTY | Individual agency, not coercive | Crown |

### Critical Constant

`phi_max = 0.618033988749895` — **no score can ever exceed this.** This is the golden ratio inverse, a structural epistemic limit. It is not a bug.

### Dogs (Independent Validators)

Each "Dog" evaluates content independently (double-blind). Results are merged:
- `deterministic-dog`: rule-based heuristics, instant, always available
- `gemini`: Gemini 2.5 Flash AI evaluator, needs network
- `huggingface`: Mistral 7B Instruct via HF Inference API, needs network
- `sovereign`: Qwen 3.5 9B on S. RTX 4060 Ti (23 t/s), via Tailscale
- `sovereign-ubuntu`: Gemma 3 4B on Ubuntu Ryzen 7 CPU (23 t/s), via Tailscale

3 model families (Google, Alibaba, Google-local) = sovereignty through diversity.

When `anomaly_detected: true` → the Dogs significantly disagree. This is a **discovery signal**, not an error. The `anomaly_axiom` field tells which axiom they disagree on.

---

## TypeScript Interfaces

```typescript
// Copy these into your project

interface JudgeRequest {
  content: string;
  context?: string;
  domain?: string;
}

interface QScore {
  total: number;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
}

interface Reasoning {
  fidelity: string;
  phi: string;
  verify: string;
  culture: string;
  burn: string;
  sovereignty: string;
}

interface DogScore {
  dog_id: string;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
  reasoning: Reasoning;
}

type VerdictKind = 'Howl' | 'Wag' | 'Growl' | 'Bark';

interface Verdict {
  verdict_id: string;
  verdict: VerdictKind;
  q_score: QScore;
  reasoning: Reasoning;
  dogs_used: string;
  phi_max: number;
  dog_scores: DogScore[];
  anomaly_detected: boolean;
  max_disagreement: number;
  anomaly_axiom: string | null;
}

interface HealthResponse {
  status: string;
  version: string;
  phi_max: number;
  axioms: string[];
}
```

---

## Hackathon Constraints (critical)

<!-- AI: enforce these constraints in all code suggestions -->

```yaml
allowed_models: [Google Gemini only for closed-source]
forbidden_apis: [Claude, OpenAI, Mistral, Cohere]
demo_scope: "show ONLY what was built during hackathon (2026-03-14)"
repo: "must be public on GitHub"
```

Full rules: see `HACKATHON-RULES.md`

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `curl /health` fails | Tell T. — server-side issue, 30 sec to fix |
| Tailscale not connecting | Try T.'s phone hotspot as backup network |
| Slow API response | Normal for Gemini calls (~2-5 sec). Show loading state. |
| CORS error | Should not happen (all origins allowed). If it does, tell T.. |
| `anomaly_detected: true` | Not a bug — it means the Dogs disagree. Highlight it in UI. |
