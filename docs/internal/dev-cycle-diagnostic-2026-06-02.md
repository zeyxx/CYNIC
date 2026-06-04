# Cycle de développement — Diagnostic + Plan — 2026-06-02

## État observé (epistemic: observé directement cette session)

**CYNIC kernel :**
- Gate cassé sur 1.96.0 (clippy ICE) → forcé downgrade 1.95.0 manuellement
- cargo-audit non installé → bloque make gate
- pre-push hook bloquait sur fichiers untracked + curation live → 3 rewrites
- 12+ commits de fix de gate pour pousser 2 commits de feature
- Pas de CI GitHub Actions → tout validé localement uniquement

**Coordination T. ↔ S. :**
- PR#3 ouverte depuis 2026-05-18 (rich embeds B&C) — sans merge
- PR#4 ouverte depuis 2026-05-27 (asdf-forge constants) — sans merge
- Pas de critères de merge définis, pas de reviewer assigné

**Général :**
- Pas de workflow PR → review → merge documenté
- Repos CultScreener, B&C, CYNIC sans CI
- Secrets/credentials gérés informellement (bloqueur SSH cynic-forge cette session)

---

## Plan post-raise (par priorité)

### P1 — GitHub Actions pour CYNIC (1 journée)
```yaml
# .github/workflows/gate.yml
on: [pull_request]
jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with: { toolchain: '1.95.0', override: true }
      - run: cargo install cargo-audit --quiet
      - run: make gate
```
→ Chaque PR validée automatiquement avant merge. Plus de gate manuel.

### P2 — Vaultwarden sur cynic-forge (½ journée)
Centralise tous les credentials. Bloque : SSH, API keys, wallets backup.
Voir [[project-credential-manager]].

### P3 — Règles de merge T. ↔ S.
- PR ouvertes ≥ 7j sans review → auto-close ou ping obligatoire
- Reviewer assigné dès ouverture (T. review B&C, S. review CYNIC)
- Merge criterion : gate vert + 1 approbation

### P4 — GitHub Actions pour B&C
S. gère côté B&C. Même pattern que CYNIC.

---

## Ce qu'on ne fait PAS maintenant

- Pas de monorepo (trop de friction, pas de gain à ce stade)
- Pas de release tags (YY.M.DD déjà défini dans scripts/bump-version.sh, à activer quand CI est vert)
- Pas de Dependabot (post-P2, une fois les basics en place)
