# CYNIC — Repo Self-Management : spécification des surfaces natives

> **Version :** 0.1-draft
> **Date :** 2026-09-06
> **Statut :** Spécification — aucun code, aucune admission d'exécution
> **Autorité :** Subordonné à `contracts/foundation.json` et `infra/runtime-authority.json`
> **Origine :** Convergence du 2026-09-06 réalisée en contournement (skill + git brut + curl) — chaque contournement ci-dessous est un interface finding à résorber

---

## 1. Problème

La convergence d'aujourd'hui (branche 29 commits → rebase → push → PR #28) a exigé :
- `git rev-list` / `git stash` / `git rebase` bruts (pas de preuve typée),
- `curl` vers l'API Forgejo pour lister/créer la PR (aucune surface native),
- vérification manuelle de la triade SHA local = forgejo = builder,
- test de subsumption manuel (rejeu cherry-pick vide) pour les fossiles.

Un agent (LLM ou humain) qui fait cela sans surfaces natives accumule de la dette cognitive et des gestes non prouvés. Ces 3 manques deviennent 3 surfaces natives.

## 2. Rôles — qui fait quoi (ratifié par cette spec)

| Rôle | Qui | Pouvoirs | Interdits |
|------|-----|----------|-----------|
| **Agent** | LLM, session hermes | propose, exécute, observe, simule | rien admettre |
| **Coordinateur** | session ou surface dédiée | tient les claims, ordonne les preuves, joue la séquence convergence | admettre, merger, supprimer du distant |
| **Juge** | lecture des preuves + scoring | lit les proofs, émet verdict (mergeable / bloqué + motif) | exécuter, admettre |
| **Humain** | opérateur souverain | ADMET ou REJETTE (merge, delete distant, force-push) | — (hors boucle quotidienne : niveaux 1 auto-admis) |

Flux : Agent propose → Coordinateur prouve (surfaces ci-dessous) → Juge verdict → Humain admit → exécution bornée → read-back.

## 3. Surface A — `cynic source pr` (lecture d'abord)

### 3.1 Commandes

```text
cynic source pr list [--state open|closed|all] [--json]
cynic source pr get <number> [--json]
cynic source pr open --head <branch> --base <base> --title <t> [--body <b>] [--json]   # phase 2 (mutation admise)
```

### 3.2 Schémas

```rust
pub struct SourcePrList {
    pub schema: String, // "cynic.source_pr_list.v1"
    pub observed_at_unix_s: u64,
    pub remote_id: String, // "forgejo" — résolu par registre, jamais hardcodé
    pub prs: Vec<SourcePrSummary>,
    pub residual_unknowns: Vec<String>,
}

pub struct SourcePrSummary {
    pub number: u64,
    pub head_ref: String,
    pub base_ref: String,
    pub head_sha: String,   // 40-hex, observé — pas le ref seul (un ref ne prouve rien)
    pub title: String,
    pub mergeable: Option<bool>, // None = inconnu explicite, pas false déguisé
}

pub struct SourcePrDetail {
    pub schema: String, // "cynic.source_pr.v1"
    pub summary: SourcePrSummary,
    pub ci_status: Option<String>,
    pub approvals: u32,
}
```

### 3.3 Obligations de preuve

- `list` prouve fraîcheur (`observed_at_unix_s`) + identité du distant (URL résolue).
- `get` prouve le head SHA exact — la décision merge porte sur le SHA, jamais sur le nom de branche.
- `open` (phase 2) exige claim actif sur le scope + read-back `get` post-création.

### 3.4 Port / adapter

- Port `SourcePrPort` dans `cynic-protocol` (types ci-dessus, aucune I/O).
- Adapter `ForgejoPrAdapter` dans `cynic-host`, URL résolue depuis `infra/repo-remotes.json` + `in
...[truncated 3075 chars]