# CYNIC — Epistemic Immune System

Independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.

## Security (INVIOLABLE — public repo)

Use placeholders for IPs: `<TAILSCALE_CORE>`, `<TAILSCALE_GPU>`, `<TAILSCALE_KAIROS>`.
Never commit: real IPs, API keys/tokens/passwords, real names (use T./S.), machine hostnames, emails.
Secrets live in `~/.cynic-env` only. Systemd uses `~/.config/cynic/env`.
Auth: `Bearer $CYNIC_API_KEY` on all endpoints except `/health`.

## Ownership Zones

T. = `cynic-kernel/` (Rust backend) + `cynic-ui/` (CYNIC dashboard). S. = chess project consuming CCM via API. Never cross zones.

## Enforcement Architecture

Three tiers protect this codebase — each rule declares which tier enforces it:
1. **Gates** — Compiler, `make lint-rules`, `make lint-drift`, git hooks, Claude hooks. Impossible to violate.
2. **Rituals** — Skills invoked at workflow triggers (`.claude/rules/workflow.md`). Procedural when invoked.
3. **Principles** — Judgment guidance in `.claude/rules/universal.md` and `kernel.md`. No mechanical enforcement.

## Build

- **Validate:** `make check` (build + test + clippy + lint-rules + lint-drift + audit)
- **Toolchain:** stable 1.94+, edition 2024. `jobs = 1` + `RUST_MIN_STACK = 16MB` in `.cargo/config.toml`.
- Do NOT change `tokio = "full"` — triggers LLVM crash (serde+rmcp monomorphization).
- If builds crash after toolchain update: `rustup toolchain uninstall/install`.

## References

- **Rules:** `.claude/rules/universal.md` (20 rules), `kernel.md` (10 rules), `workflow.md` (triggers), `reference.md` (data)
- **Identity:** `docs/identity/` (epistemology, φ-convergence, sovereignty — pérenne)
- **Reference:** `docs/reference/` (V08 truths, CCM protocol, infra spec — must match code)
- **Audit:** `docs/audit/CYNIC-FINDINGS-TRACKER.md` (SoT for security debt)
- **API contract:** `API.md`
- **Env:** `${CYNIC_REST_ADDR}`, `${CYNIC_API_KEY}` from `~/.cynic-env`. Never hardcode.
