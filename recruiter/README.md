# CYNIC — Présentation Recruteur (Corporate Front)

> Infrastructure souveraine en Rust. Coordination multi-composants sans
> contrôleur centralisé. Preuve par read-back à chaque mutation.

Cette section est la face présentable du projet pour recruteurs / PME.
Aucune technologie financière ou speculative n'est exposée ici.

## Ce que CYNIC fait (en une phrase)

Un runtime Rust bas-niveau qui orchestre host, build, artifact, agent, et
inférence sur des nœuds (une machine ou un parc distribué) avec des
preuves vérifiables à chaque étape — pas de boîte noire.

## Preuves techniques (read-back)

| Preuve | Commande | Statut |
|--------|----------|--------|
| Build reproductible | `make check` | voir CI |
| Audit runtime | `make audit-runtime-phases` | intégré |
| Bench inférence locale | `scripts/benchmark-all-models.py` | MoE 35B @ 19 tok/s (APU) |

## Compétences démontrées

- **Rust systèmes** (1.95/1.96): crates modulaires, types partagés no-I/O.
- **Souveraineté données**: inférence locale, aucun cloud obligatoire.
- **Automatisation**: supervision host, déploiement, audit conformité.
- **Preuve par évidence**: chaque mutation laisse une trace machine-readable.

## Cas d'usage PME (pitch)

- Audit IT souverain (conformité, supervision) — facturable, récurrent.
- Automatisation de déploiement sur le matériel existant (pas de GPU cher).
- IA locale 35B sur PC standard (vf: MoE + offload unifié) — données restent
  sur site, coût ≈ électricité.

## Structure du repo (pour qui veut creuser)

- `crates/cynic-kernel` — bus de coordination, observations, health.
- `crates/cynic-host` — primitives host/OS portables.
- `crates/cynic-node` — supervision nœud-local.
- `crates/cynic-cli` — surface opérateur.
- `crates/cynic-protocol` — types de protocol partagés.
- `infra/` — registry, systemd, storage, firewall, exposition agent.
- `contracts/foundation.json` — contrat de politique évolutive minimal.

## Éthique du projet

Le human reste l'autorité souveraine. Aucune décision automatisée sans
preuve, aucune amplification silencieuse d'autorité. Transparence par
read-back, pas par promesse.

---
*Détail technique complet: `../README.md` et `../AGENTS.md` (runtime law).*
*Cartographie des portes de revenus: `portes.md`.*
