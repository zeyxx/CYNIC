# Audit disque cortex (2026-08-25) — où passent les 312G

## Répartition racine

| Chemin | Taille | Nature |
|---|---|---|
| /home | 196G | worktrees + caches dev |
| /var/lib/containerd | 41G | **docker overlay du container cynic-seed-build** |
| /usr | 18G | système |
| /tmp | 17G | artefacts de builds éphémères |
| /var/lib (hors containerd) | ~26G | snapd 7.3G, cynic 6.1G, flatpak 3.9G, prometheus 3.8G |
| /root | 1.7G | — |

## Détail des gros postes

### Docker : 43.3G images + **39.7G RW layer d'UN container**
- `cynic-seed-build` (rust:1.96.0, up 36h) → son layer inscriptible = **39.7G**
  - `/tmp/build` = 32G dont build-targets 16G + target 15G
  - des shells de retry rustup/cargo check tournent encore dedans
- Images docker réclaimables : 42G/43.3G marquées reclaimable (97%)

### /home/user (183G)
| Poste | Taille | Nature |
|---|---|---|
| Bureau/SOLANA/zeyxx/CYNIC/target | 20G | artefacts cargo (re-générables) |
| Bureau/SOLANA/zeyxx/CYNIC-hermes | 20G | **worktree duplicata** (target 19G dedans) |
| .cache | 19G | uv 7.1G · huggingface 3.2G · chrome 2.1G · playwright 1.7G · yarn 1.6G · solana 1.6G · puppeteer 0.6G |
| dream-server/data | 16G | données app (à ne PAS toucher sans validation) |
| .cynic/organs | 14G | hermes/x/chrome-profiles 6.1G · captures 2.1G · telegram 5G |
| .rustup/toolchains | 11G | toolchains rust multiples |
| ODYSSEUM_LEGACY + ODYSSEUM | 9.3G | projets séparés |
| blitz-and-chill | 3.6G | projet |
| CYNIC target-cli etc. | ~6G | /var/lib/cynic côté cortex |

### /tmp (17G)
cynic-nightly4 4.4G · cynic-cargo-targets 2.5G · cynic-nightly 1.7G ·
cynic-seed-sync 1.3G · cynic-combat 1.3G (+ le reste dispersé)

## Total récupérable identifié (artefacts & caches purs)

| Cible | Gain estimé | Risque |
|---|---|---|
| CYNIC/target + CYNIC-hermes/target | 39G | zéro (cargo clean, se régénèrent) |
| docker: stop+rm cynic-seed-build puis prune images | ~40G | faible — c'est un seed-build terminé depuis le 24/08, à confirmer |
| /tmp/cynic-* | 11G | zéro (sauf cynic-combat à archiver avant) |
| .cache/uv + yarn + puppeteer + playwright | ~11G | zéro-faible (re-téléchargement au besoin) |
| solana cache + chrome profile cache | 3.7G | faible |
| **TOTAL court terme** | **~105G** | 94% → ~64% |

Plus tard (décision): .rustup toolchains redondantes (~6G), organs/hermes
chrome-profiles (6.1G si les profils X sont régénérables), prometheus
rétention (3.8G), flatpak inutilisés.

## Loi à corriger (proposition de formulation)

« registry = truth » est trop court. Le registry porte le **desired state**;
la vérité d'exécution reste l'observation du substrate. Loi corrigée :

> Le registry est la source de vérité du *désir* ; le substrate est la
> source de vérité du *fait*. Une divergence entre les deux est un finding
> typé (jamais un mensonge), résolue par réconciliation explicite — jamais
> en réécrivant silencieusement l'un ou l'autre.

C'est exactement ce que `cynic dev fleet-divergence` fait déjà (2 refutations
locales-kernel détectées et rapportées comme findings, pas corrigées en
silence).
