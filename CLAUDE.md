# CYNIC V2 — Constitution

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt.

## Security (INVIOLABLE — this repo is PUBLIC)

### Never commit
- **Real IPs** — use `<TAILSCALE_UBUNTU>`, `<TAILSCALE_FORGE>`, etc.
- **API keys, tokens, passwords** — they live in `~/.config/cynic/env` (never tracked)
- **Real names** — use initials (`T.`, `S.`) or roles (`backend-dev`, `frontend-dev`)
- **Machine hostnames** that reveal identity
- **Email addresses** other than `@users.noreply.github.com`

### Before every commit
Verify no secrets are staged: `git diff --staged | grep -iE "api.key|token|password|secret|AIza|hf_|100\.(74|75|119|65|105)\."` — must return empty.

### Infrastructure references
All IPs and hostnames in docs/code use placeholders. Real values are in env files only.
```
<TAILSCALE_UBUNTU>     → backend machine
<TAILSCALE_STANISLAZ>  → frontend machine
<TAILSCALE_FORGE>      → CI/build server
```

### Auth
- REST API requires `Authorization: Bearer $CYNIC_API_KEY` on all endpoints except `/health`
- llama-server requires `--api-key` (read from `~/.config/cynic/llama-api-key`)
- SSH is key-only (password auth disabled), Tailscale interface only

### Roles
T. = backend (cynic-kernel/). S. = frontend (cynic-ui/).

## Ownership Zones (CRITICAL)

```
cynic-kernel/    → T. ONLY. Backend Rust.
cynic-ui/        → S. ONLY. Frontend React+TS.
Root docs        → Frozen during hackathon (API.md, FRONTEND.md, CLAUDE.md)
```
**Rule:** Never modify files outside your zone. `git pull --rebase` before every push.

## Live Infrastructure

| Service | URL | What |
|---|---|---|
| CYNIC Kernel | http://<TAILSCALE_UBUNTU>:3030 | REST API (Tailscale) |
| llama-server (Gemma 3 4B) | http://<TAILSCALE_UBUNTU>:8080 | Sovereign inference (local CPU) |
| llama-server (Qwen 3.5 9B) | http://<TAILSCALE_STANISLAZ>:8080 | Sovereign inference (S. GPU) |

## API Essentials

All endpoints except `/health` require `Authorization: Bearer $CYNIC_API_KEY`.

```
GET  /health                → public, no auth
POST /judge                 → Bearer required
GET  /verdicts              → Bearer required
GET  /verdict/{id}          → Bearer required
GET  /crystals              → Bearer required
GET  /usage                 → Bearer required
```
Rate limit: 30 requests/minute. `/health` exempt.

Full contract: `API.md`. Frontend guide: `FRONTEND.md`.

## Axioms (inviolable)

| Axiom | Judges |
|---|---|
| FIDELITY | Is this faithful to truth? Sound principles? |
| PHI | Structurally harmonious? Coordinated? Proportional? |
| VERIFY | Testable? Can be verified or refuted? |
| CULTURE | Honors traditions and established patterns? |
| BURN | Efficient? Minimal waste? |
| SOVEREIGNTY | Preserves individual agency and freedom? |

**CYNIC judges SUBSTANCE, not FORM.** In chess, the strategy quality — not the text description.

## φ Constants

```
φ    = 1.618034   Golden ratio
φ⁻¹  = 0.618034   Max confidence / crystallization threshold
φ⁻²  = 0.382      Decay threshold / anomaly trigger
HOWL ≥ 0.5207     WAG ≥ 0.382     GROWL ≥ 0.236     BARK < 0.236
```
Real chess scores: Sicilian Defense → Howl. Scholar's Mate → Growl. Fool's Mate → Bark.

## Dogs (Independent Validators)

| Dog | Model | Where |
|---|---|---|
| deterministic-dog | Heuristics (instant) | In-kernel |
| gemini | Gemini 3 Flash | Google API |
| huggingface | Mistral 7B | HF Inference |
| sovereign | Qwen 3.5 9B | S. RTX 4060 Ti |
| sovereign-ubuntu | Gemma 3 4B | Ubuntu CPU |

## Development Principles

1. **Diagnose before fixing.** Read errors, trace data, one hypothesis, test minimally.
2. **2 fix attempts max.** Obvious → alternative → escalate. Never brute-force.
3. **One logical change per commit.** `type(scope): description`.
4. **Domain purity.** Zero `#[cfg]` in domain code.
5. **Port contracts first.** New dependency → trait → adapter → test.
6. **Bounded everything.** Channels, retries, confidence. Unbounded = debt.

## Skills (MUST invoke before acting)

| Skill | When |
|-------|------|
| `cynic-kernel` | Building/modifying any CYNIC component |
| `cynic-judge` | Evaluating code, decisions, or content |
| `cynic-burn` | Simplification — orphans, hotspots, dead code |
| `cynic-wisdom` | Philosophical grounding for decisions |
| `ai-infrastructure` | LLM serving, inference pipelines |
| `crystallize-truth` | Complex decisions, hidden assumptions |
| `frontend-dev` | Building/modifying cynic-ui |
| `deploy` | After ANY code change to cynic-kernel/ |
| `test-chess` | Verify chess scoring works end-to-end |
| `status` | Check kernel, DB, backends, network |
| `context7` | Fetch up-to-date docs for any library |

## Slash Commands

| Command | What |
|---|---|
| `/build` | Build + test + clippy |
| `/deploy` | Build + deploy binary + restart kernel + verify |
| `/run` | Start kernel |
| `/e2e` | End-to-end test |
| `/test-chess` | 3 chess positions → verify scoring |
| `/status` | Full system status |

## Canonical References

- **Cognitive architecture:** `docs/CYNIC-CRYSTALLIZED-TRUTH.md`
- **Infrastructure truths:** `docs/CYNIC-ARCHITECTURE-TRUTHS.md`
- **API contract:** `API.md`
- **Frontend guide:** `FRONTEND.md`
- **Build:** `cargo build -p cynic-kernel --release` / `cargo test -p cynic-kernel` / `cargo clippy --workspace -- -D warnings`
