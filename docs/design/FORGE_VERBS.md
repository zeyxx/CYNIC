# Forge Verbs — Distilling Manual Forge Ops into the CLI

Spec for mapping manual Forge operations to existing CLI families. No new
top-level families (operator law): new verbs nest under `build`, `source`,
or `audit`.

## Gap-to-family mapping

| # | Manual op | Gap | Target family | Verb (planned) | Status |
|---|-----------|-----|---------------|----------------|--------|
| 1 | runner token + enrollment | build | `build` | `cynic build runner enroll` / `token` | planned |
| 2 | actions runs watch | build | `build` | `cynic build runs watch` | planned |
| 3 | push-mirror inspect | source | `source` | `cynic source mirror inspect` (push-mirror) | planned |
| 4 | main-divergence | source | `source` | `cynic source divergence [--json]` | **implemented** |
| 5 | gate-receipt verify | audit/source | `audit` (or `source`) | `cynic audit gate-receipt` / `cynic source receipt verify` | planned |

## Implemented verb: `cynic source divergence`

- Read-only. Compares Forgejo (SSOT) `main` vs GitHub (relay) `main` SHAs.
- Hosts resolved from `infra/registry.json` (`vcs_provider.sovereign_ssot`,
  `vcs_provider.relay`, forgejo node ssh transport) — no hardcoded IPs.
- Transport: `git ls-remote` over SSH for Forgejo, HTTPS for GitHub.
- Ancestry via a throwaway git repo + `git fetch` both mains + `git merge-base
  --is-ancestor`.
- States: `AGREE`, `GH-AHEAD`, `FORGEJO-AHEAD`, `DIVERGED` (mirrors
  `/home/user/.local/bin/cynic-main-watchdog.sh`).
- Output: text `AGREE <sha>` / `<STATE> gh=<sha> fj=<sha>`; `--json` emits
  `cynic.source.divergence.v1`.

## Non-goals

- No mutation, no push, no force-update.
- No new top-level CLI family.
- No domain logic in CLI beyond classification glue; I/O lives in the
  surface module (thin read-back).
