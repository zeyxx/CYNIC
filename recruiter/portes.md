# Cartographie des Portes — Plan d'Action Multi-Échelle

Document de cadrage pour la recherche d'alternance / revenus stables.
Corporate front uniquement (aucune référence Solana / agents financiers / crypto).

## Contexte matériel prouvé (levier technique)

APU Renoir (mémoire unifiée DDR4-3200). Offload Vulkan GTT validé:

| Modèle | Contexte | Decode (tok/s) | Qualité équivalente |
|--------|----------|----------------|---------------------|
| LFM2.5-2.6B Q4_K_M | 256k | 16.5 | ~9B dense |
| **Qwen3.6-35B-A3B MoE IQ2_M** | **256k** | **19.0** | **35B (MoE)** |
| qwythos-9B Q4 | 8k | 7.4 | 9B |
| qwen3.8-27B IQ3 | 8k | 3.1 | 27B |

Le MoE 35B-A3B (3B actifs/token) bat un dense 2.6B sur contexte 256k sur
hardware pourri — preuve qu'un petit budget compute + bon choix de modèle
(MoE) bat un gros modèle dense. C'est le pitch PME: "IA 35B souveraine sur
PC standard, sans GPU dédié, conforme données".

## Portes principales

### Porte A — Alternance dès 21 sept + PME automation / freelance Rust
- **Revenu**: salaire alternance (stable) + complément freelance court.
- **Actif**: CYNIC comme "produit d'automatisation PME" (Rust, local, souverain).
- **Échéances**:
  - Avant 21/09: dossier de vente PME (offre audit IT + automatisation Rust).
  - Avant 21/09: 1 démo CYNIC live (supervision/déploiement sur une machine).
  - 21/09→: alternance = base. 1er contrat PME visé avant fin oct.
- **Risque**: temps limité (35h/sem). Mitigation: missions courtes, pas de SaaS 2 ans.

### Porte B — Pas d'alternance, école initiale jusqu'à déc + temps pour TALARIA SAS
- **Revenu**: 0 garanti (école unpaid) → TALARIA SAS doit générer avant déc.
- **Actif**: TALARIA SAS = véhicule de vente (PME automation + Rust infra).
- **Échéances**:
  - Sem 1 (sept): site vitrine TALARIA SAS (offre PME claire, 0 crypto).
  - Sem 2-4: 3-5 RDV PME/sem, 1 contrat signé visé avant fin oct.
  - Parallèle: CYNIC en état "démontrable" (make check vert, 1 cas client).
- **Risque**: trou noir financier sept→déc. Mitigation: épargne + contrats courts.

### Porte C — Pas d'alternance ni école possible → FULL TIME TALARIA SAS
- **Revenu**: TALARIA SAS = seul revenu, urgence cash.
- **Actif**: boîte de service IA souveraine + automatisation Rust PME.
- **Échéances**:
  - J+1: pack commercial (devis, offre "IA privée sur votre PC" + automat IT).
  - J+1→J+15: 10 prospects PME chauds, 1er contrat sous 15j.
  - J+15→: livraison + itérer. CYNIC = socle, pas le produit vendu.
- **Risque**: maximal (0 revenu pendant conquête). Mitigation: contrat avant dépense.

## Portes émergentes (à surveiller / cultiver)

1. **Noeud d'inférence fédéré CYNIC** — un 2e hardware (carte d'occasion 4060Ti
   16GB ou nœud distant) fédéré via le runtime. Monétisable: inférence à la
   demande pour PME sans hardware.
2. **Produit "Audit IT souverain"** — utiliser `crates/cynic-host` audit engine
   comme service récurrent (conformité, supervision) facturable mensuel.
3. **Formation / prestation Rust pour PME** — le skill Rust souverain est rare;
   vendre de la montée en compétence interne.
4. **Contribution open-source CYNIC → notoriété recruteur** — GitHub vert
   (CI, tests, bench) = preuve technique pour alternance (Porte A) SANS pitch.
5. **Pivot CAC40** — si une PME débouche sur réseau plus gros (corporate front
   réussit), monter en gamme. Cypherpunk back reste caché.

## Critère de bascule entre portes

| Signal | Bascule vers |
|--------|-------------|
| Alternance confirmée avant 21/09 | Porte A |
| Alternance refusée + école OK | Porte B |
| Alternance refusée + école impossible | Porte C |
| 1er contrat TALARIA SAS signé | accélère B/C, confirme viabilité |

## Première étape commune (cette semaine, toutes portes)

1. 1 page offre PME (Rust / souverain / IA locale) — 0 crypto.
2. 1 démo CYNIC live (supervision ou déploiement) à montrer.
3. MoE 35B-A3B benchmarké = preuve technique du pitch "IA 35B sans GPU cher".

## Read-back attendu (preuve, pas promesse)

- Alternance: contrat signé + 1 déploiement CYNIC chez PME réel.
- TALARIA SAS: 1er virement = preuve de survie.
- Repo: `make check` vert + bench MoE documenté = preuve technique recruteur.
