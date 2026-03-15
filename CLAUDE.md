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
7. **Zero hardcoded paths.** Use `$(git rev-parse --show-toplevel)` for project root, `${CYNIC_REST_ADDR}` from `~/.cynic-env` for kernel address. Tool-specific vars (`$CLAUDE_PROJECT_DIR`, etc.) only as optimization with git fallback. Never absolute paths in skills, hooks, or configs.

## Tool Ecosystem (MUST use — manual is the fallback, not the default)

### When to invoke what

| Trigger | Tool/Skill | Type |
|---------|-----------|------|
| **Any code change** | `/build` | Slash command |
| **Deploy to production** | `/deploy` | Slash command (includes DB backup) |
| **Start kernel** | `/run` | Slash command (systemd) |
| **Check system state** | `/status` | Slash command |
| **End-to-end test** | `/e2e` | Slash command |
| **Verify chess scoring** | `/test-chess` | Slash command |
| **Building/modifying CYNIC** | `cynic-kernel` | Skill (architecture ref) |
| **Evaluate quality of anything** | `cynic-judge` | Skill (43-dim scoring) |
| **Simplify/reduce code** | `cynic-burn` | Skill |
| **Philosophical grounding** | `cynic-wisdom` | Skill (19 traditions) |
| **Complex decisions** | `crystallize-truth` | Skill |
| **LLM/ML infrastructure** | `ai-infrastructure` | Skill |
| **System design from scratch** | `engineering-stack-design` | Skill |
| **Frontend work** | `frontend-dev` | Skill |
| **Library docs lookup** | `context7` MCP | Plugin (resolve-library-id → query-docs) |
| **Judge content via API** | `cynic_judge` MCP | CYNIC MCP tool |
| **Run inference** | `cynic_infer` MCP | CYNIC MCP tool |
| **Read verdicts/crystals** | `cynic_verdicts` / `cynic_crystals` MCP | CYNIC MCP tool |
| **Audit trail** | `cynic_audit_query` MCP | CYNIC MCP tool |
| **Tailscale discovery** | `ts_discover` / `ts_status` MCP | Tailscale MCP |
| **Remote execution** | `ts_exec` MCP | Tailscale MCP |
| **Remote logs** | `ts_logs` MCP | Tailscale MCP |
| **Before claiming "done"** | `verification-before-completion` | Superpowers plugin |
| **Before creative work** | `brainstorming` | Superpowers plugin |
| **Debugging** | `systematic-debugging` | Superpowers plugin |
| **After major feature** | `code-review` | Superpowers plugin |
| **Commit + push** | `commit` / `commit-push-pr` | Commit commands plugin |

### Slash Commands

| Command | What |
|---|---|
| `/build` | Build + test + clippy (--release) |
| `/deploy` | Build + test + backup DB + deploy binary + restart kernel + verify |
| `/run` | Start kernel via systemd |
| `/e2e` | End-to-end test against running kernel |
| `/test-chess` | 3 chess positions → verify scoring |
| `/status` | Full system status (kernel + DB + llama + Tailscale + backups + git) |

### MCP Tools

| Server | Tools | Purpose |
|--------|-------|---------|
| **cynic** | health, judge, infer, verdicts, crystals, audit_query | CYNIC kernel interaction |
| **tailscale** | status, health, discover, exec, logs, service, poll | Fleet management |
| **context7** | resolve-library-id, query-docs | Up-to-date library documentation |
| **playwright** | navigate, screenshot, snapshot, click, fill, evaluate | Browser automation |

### Environment

All skills use `${CYNIC_REST_ADDR}` and `${CYNIC_API_KEY}` from `~/.cynic-env`. Never hardcode IPs.

## Canonical References

- **Cognitive architecture:** `docs/CYNIC-CRYSTALLIZED-TRUTH.md`
- **Infrastructure truths:** `docs/CYNIC-ARCHITECTURE-TRUTHS.md`
- **API contract:** `API.md`
- **Frontend guide:** `FRONTEND.md`
- **Build:** `cargo build -p cynic-kernel --release` / `cargo test -p cynic-kernel --release` / `cargo clippy -p cynic-kernel --release -- -D warnings`
