# Audit preuves PR 137 — Verdict : APPROVE

Prisme : preuves. Head `2601eef2`, base `9c2480ea`. Rejoué au SHA exact + reproduction sur `main` vierge.

| Check | PR 137 | Main vierge | Conclusion |
|---|---|---|---|
| `crate_topology_coherence` | 6/6 | — | Vert |
| `cynic-secrets` (seed jetable) | 25/25 | — | Vert |
| `cynic-cli --lib` | 115/116 | `manifest_domains_match_clap_surface` FAILED aussi | Pré-existant, pas régression |
| `cynic-node --lib` | 492/494 | 2 dns FAILED aussi | Pré-existant (réservation plan-service, environnemental) |

**Bonus** : `builder_endpoint_resolves_from_registry` échoue sur main, passe sur la PR — la tranche répare un test pré-existant.

**Audit** : 0 code PATH-004 confirmé. Restants = cohérence ledger hors périmètre (compteurs exacts variables selon substrate ; main lui-même à ~300+, même nature). Host E0252 : profil test cassé pré-existant sur main.

Note : le reviewer signale le checkout partagé vide/instable pendant l'audit (re-cloné depuis Forgejo). Hygiène worktree à surveiller, sans impact sur les résultats.
