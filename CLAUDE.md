# CYNIC — Epistemic Immune System

Independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.

## Security (INVIOLABLE — public repo)

Use placeholders for IPs: `<TAILSCALE_UBUNTU>`, `<TAILSCALE_STANISLAZ>`, `<TAILSCALE_FORGE>`.
Never commit: real IPs, API keys/tokens/passwords, real names (use T./S.), machine hostnames, emails.
Secrets live in `~/.cynic-env` only. Systemd uses `~/.config/cynic/env`.
Auth: `Bearer $CYNIC_API_KEY` on all endpoints except `/health`.

## Ownership Zones

T. = `cynic-kernel/` (Rust backend). S. = `cynic-ui/` (React+TS frontend). Never cross zones.

## Skill Routing (MUST invoke before acting)

| Trigger | Skill |
|---------|-------|
| Before adding a module, dependency, or Cargo.toml change | `/cynic-skills:cynic-empirical` |
| Before choosing between architecturally different approaches | `/cynic-skills:crystallize-truth` |
| Before designing a new subsystem | `/cynic-skills:engineering-stack-design` |
| After ANY code change to cynic-kernel/ | `/build` |
| After modifying >5 files or >100 lines in a session | `/cynic-skills:distill` |
| After changes to scoring, prompts, or crystal injection | `/test-chess` (before AND after) |
| Before editing any file | `cynic_coord_who()` then `cynic_coord_claim()` |
| After work complete on a file | `cynic_coord_release()` |
| Evaluate quality of code/decisions | `/cynic-skills:cynic-judge` |
| Simplify or burn dead code | `/cynic-skills:cynic-burn` |
| Touching cynic-kernel/ source | `/cynic-kernel` (architecture reference) |
| Deploy to production | `/deploy` |
| System health check | `/status` |

## Top Rules (most violated — full set in `.claude/rules/`)

1. **Diagnose before fixing.** Read errors, trace data, one hypothesis, test minimally. Never chain reactive fixes.
2. **Fix → Test → Gate → Verify.** Every fix: code fix + regression test + mechanical gate + gate verification.
3. **Measure before AND after.** No "improved X" claims without before/after numbers.

## References

- **Full rules:** `.claude/rules/` (universal, kernel, workflow, reference)
- **Architecture:** `docs/architecture/CYNIC-CRYSTALLIZED-TRUTH.md`, `docs/architecture/CYNIC-ARCHITECTURE-TRUTHS.md`
- **API contract:** `API.md` | **Frontend:** `FRONTEND.md`
- **Build:** `make check` (build + test + clippy --release)
- **Env:** `${CYNIC_REST_ADDR}`, `${CYNIC_API_KEY}` from `~/.cynic-env`. Never hardcode.

## Build Notes

- Toolchain: stable 1.94+, edition 2024. `jobs = 1` + `RUST_MIN_STACK = 16MB` in `.cargo/config.toml`.
- Do NOT change `tokio = "full"` — triggers LLVM crash (serde+rmcp monomorphization).
- If builds crash after toolchain update: `rustup toolchain uninstall/install`.
