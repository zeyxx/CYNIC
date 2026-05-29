<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Research : Solana Program Frameworks — 2026-05-29

**Contexte :** Évaluation des frameworks pour le futur programme on-chain CYNIC (settlement de verdicts, mint-permit).

---

## Frameworks évalués

### 1. Anchor

**Le standard de facto pour Solana.**

- **Langage :** Rust avec macros proc
- **Avantages :**
  - Sérialisation/désérialisation automatique (Borsh)
  - Validation de comptes déclarative
  - Tooling mature (anchor init, build, test, deploy)
  - Grande communauté, beaucoup d'exemples
  - IDL automatique → génération clients TypeScript
- **Inconvénients :**
  - Overhead : binaire plus gros (~100KB+ vs ~10KB native)
  - Abstractions cachent la complexité (risque de mauvaise compréhension)
  - CPI (Cross-Program Invocation) verbeux
  - Macros = temps de compilation plus long
- **Verdict CYNIC :** Bon pour prototypage rapide, mais l'overhead contredit BURN (efficacité).

### 2. Native (sans framework)

**Solana SDK brut.**

- **Langage :** Rust pur avec solana-program crate
- **Avantages :**
  - Contrôle total
  - Binaire minimal (~5-10KB)
  - Pas de dépendances opaques
  - Maximum SOVEREIGNTY (pas de framework tiers)
- **Inconvénients :**
  - Beaucoup de boilerplate
  - Validation de comptes manuelle (risque de bugs)
  - Pas d'IDL automatique
  - Courbe d'apprentissage plus raide
- **Verdict CYNIC :** Maximum souveraineté mais risque de bugs de validation.

### 3. Pinocchio

**Framework léger, nouvelle génération.**

- **Langage :** Rust
- **Avantages :**
  - Binaire très petit (proche du natif)
  - Abstractions minimales mais utiles
  - Philosophie "zero-cost abstractions" (aligné avec Rust)
  - Validation de comptes sans macros proc
  - Bon compromis poids/sécurité
- **Inconvénients :**
  - Moins mature qu'Anchor
  - Communauté plus petite
  - Moins d'exemples/tutorials
  - Évolution rapide (API instable ?)
- **Verdict CYNIC :** **Meilleur alignement avec les axiomes CYNIC** — BURN (efficacité), SOVEREIGNTY (pas de framework opaque), VERIFY (code lisible).

### 4. Bolt (par MagicBlock)

**Framework pour jeux on-chain (ECS pattern).**

- **Langage :** Rust + TypeScript
- **Avantages :** ECS pattern, bon pour état complexe
- **Inconvénients :** Orienté gaming, overhead ECS inutile pour CYNIC
- **Verdict CYNIC :** Hors scope — pas adapté au cas d'usage jugement.

### 5. Steel

**Micro-framework récent.**

- **Langage :** Rust
- **Avantages :** Très léger, bon pour programmes simples
- **Inconvénients :** Trop récent, peu de production use
- **Verdict CYNIC :** À surveiller mais trop immature.

---

## Matrice de décision

| Critère | Poids | Anchor | Native | Pinocchio | Steel |
|---------|-------|--------|--------|-----------|-------|
| BURN (efficacité binaire) | 20% | 2/5 | 5/5 | 4/5 | 4/5 |
| SOVEREIGNTY (indépendance) | 20% | 2/5 | 5/5 | 4/5 | 3/5 |
| VERIFY (auditabilité) | 20% | 3/5 | 3/5 | 4/5 | 3/5 |
| Maturité / communauté | 15% | 5/5 | 4/5 | 3/5 | 1/5 |
| Vitesse de dev | 15% | 5/5 | 2/5 | 3/5 | 2/5 |
| Sécurité (validation) | 10% | 4/5 | 2/5 | 4/5 | 3/5 |
| **Score pondéré** | | **3.2** | **3.6** | **3.7** | **2.6** |

---

## Recommandation

**Pinocchio** est le framework recommandé pour le programme CYNIC on-chain :

1. Meilleur score pondéré (3.7)
2. Aligné avec la philosophie CYNIC (efficacité + souveraineté)
3. Binaire léger (important pour coût de déploiement Solana)
4. Code lisible sans macros opaques

### Plan d'implémentation

1. **Prototype** : programme de settlement de verdict (store verdict hash on-chain)
2. **v1** : mint-permit (token gating basé sur verdicts CYNIC)
3. **v2** : réseau de Dogs on-chain (staking + slashing)

---

## Références

- Pinocchio : github.com/febo/pinocchio
- Anchor : github.com/coral-xyz/anchor
- Solana Cookbook : solanacookbook.com
- Steel : github.com/regolith-labs/steel

---

*Research interne — supporte la roadmap technique pour le pitch.*
