# CYNIC V2 — Constitution

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt.

## Security (INVIOLABLE — this repo is PUBLIC)

### Never commit
- **Real IPs** — use `<TAILSCALE_UBUNTU>`, `<TAILSCALE_FORGE>`, etc.
- **API keys, tokens, passwords** — they live in `~/.cynic-env` (never tracked). Systemd uses `~/.config/cynic/env` (generated from `~/.cynic-env` by deploy scripts).
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
GET  /crystal/{id}          → Bearer required
GET  /usage                 → Bearer required
GET  /dogs                  → Bearer required
GET  /temporal              → Bearer required
GET  /agents                → Bearer required
POST /observe               → Bearer required
POST /coord/register        → Bearer required (hooks + agents)
POST /coord/claim           → Bearer required (hooks + agents)
POST /coord/release         → Bearer required (hooks + agents)
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
HOWL > 0.528 (φ⁻²+φ⁻⁴)     WAG > 0.382 (φ⁻²)     GROWL > 0.236 (φ⁻³)     BARK ≤ 0.236
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
8. **Never discard fallible I/O.** `let _ =` on DB writes, HTTP calls, or file ops is forbidden. Handle the error (log + retry/skip) or propagate with `?`. Silent data loss is the worst bug.
9. **Wire or delete.** Every new config field, trait, or public method MUST have at least one caller AND one test. Dead code that "might be useful later" is debt, not investment. `grep` for zero-caller symbols before merging.
10. **Timeout every background `.await`.** Any `.await` inside a `tokio::spawn` must be wrapped in `tokio::time::timeout()`. Bare awaits in background tasks can stall indefinitely without any signal.
11. **`Display` implies `Error`.** Every type that implements `std::fmt::Display` for error reporting MUST also implement `std::error::Error`. No exceptions — it's two lines.
12. **Fix the class, not the instance.** When fixing a bug, `grep` the entire codebase for the same pattern. If `let _ =` was wrong in one flush path, check ALL flush paths. A fix that doesn't sweep its class is half a fix.
13. **The compiler is the enforcement layer.** `#![deny(dead_code, unused_imports)]` is in lib.rs. If a rule can be enforced by the Rust compiler or clippy, it MUST be — not by bash scripts, not by text in CLAUDE.md. Scripts are debt. Compiler lints are permanent.
14. **One value, one source.** Every config value (URL, secret, port) has exactly ONE source of truth. All other consumers derive from it, never copy. `backends.toml` = Dogs. `~/.cynic-env` = secrets. If you must edit 2 files to add a backend, the architecture is wrong.
15. **HTTP status codes are the contract.** `/health` returns 200 (sovereign) or 503 (degraded/critical). Monitoring scripts check the status code, never parse JSON. Any tool that parses API responses to make decisions is doing the server's job — move the logic to the server.
16. **Scripts are thin. Logic lives in the kernel.** Bash hooks and monitoring scripts are `curl + status code check`. Zero parsing, zero decision logic, zero Python. If a script has an `if` that depends on API response content, that `if` belongs in the kernel.

## Mandatory Tool Use (NON-OPTIONAL)

These are not suggestions. Skipping any of these is a workflow violation.

### BEFORE triggers
- **Before editing a file:** `cynic_coord_who()` then `cynic_coord_claim(agent_id, target)` — coordination is not optional
- **Before building something new:** `/cynic-skills:cynic-empirical` — look outside before inventing
- **Before a decision under uncertainty:** `/cynic-skills:crystallize-truth` — analyze before committing
- **Before architecting a system:** `/cynic-skills:engineering-stack-design` — inventory before priority

### AFTER triggers
- **After ANY code change:** `/build` (= `make check` = build + test + clippy --release)
- **After ILC complete:** `cynic_coord_release(agent_id, target)` per file, then `cynic_coord_release(agent_id)` for session
- **After scoring changes:** `/test-chess` — benchmark before and after

### ON triggers
- **Evaluate quality:** `/cynic-skills:cynic-judge` — 43-dimension φ-bounded scoring
- **Simplify or reduce code:** `/cynic-skills:cynic-burn` — don't extract, burn
- **Dilemma or philosophical tension:** `/cynic-skills:cynic-wisdom` — 19 traditions
- **Building/modifying CYNIC kernel:** `/cynic-kernel` — architecture reference (read before touching)
- **Infrastructure or AI pipeline work:** `/cynic-skills:ai-infrastructure`
- **Deploy to production:** `/deploy` (= build + test + backup DB + deploy binary + restart + verify)
- **System status:** `/status` — full dashboard (kernel + DB + backends + git)

### Environment
All tools use `${CYNIC_REST_ADDR}` and `${CYNIC_API_KEY}` from `~/.cynic-env`. Never hardcode IPs.

### MCP servers available
- **cynic** — kernel interaction (health, judge, infer, verdicts, crystals, coord_*)
- **tailscale** — fleet management (status, exec, logs, discover)
- **context7** — up-to-date library documentation
- **playwright** — browser automation

### When things go wrong
Escalation, troubleshooting, session lifecycle detail, conflict resolution: `/cynic-workflow`

## Canonical References

- **Cognitive architecture:** `docs/CYNIC-CRYSTALLIZED-TRUTH.md`
- **Infrastructure truths:** `docs/CYNIC-ARCHITECTURE-TRUTHS.md`
- **API contract:** `API.md`
- **Frontend guide:** `FRONTEND.md`
- **Build:** `make check` (or `cargo build/test/clippy -p cynic-kernel --release`)

## Build Notes

- **Toolchain:** stable 1.94+, edition 2024 (stable since Rust 1.85)
- **Known rustc bug:** LLVM stack overflow on deep monomorphization (serde+rmcp = 47% of IR). See rust-lang/rust #103767, #122357, #138561.
- **Workaround committed in `.cargo/config.toml`:** `jobs = 1` + `RUST_MIN_STACK = 16MB`. Do not change without testing clean release builds.
- **gRPC:** feature-gated (`--features grpc`), off by default. No client exists yet.
- **Do NOT change `tokio = "full"`** — targeted features alter the monomorphization graph and trigger the LLVM crash.
- **If builds crash after toolchain update:** `rustup toolchain uninstall <ver> && rustup toolchain install <ver>` — SIGSEGV can corrupt toolchain metadata.
