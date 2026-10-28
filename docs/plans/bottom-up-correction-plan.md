# Plan boussole — Corrections structurelles bottom-up

Date : 2026-10-27
Statut : document de travail, à compléter par observation
Source : session hermes, couches observées du bas vers le haut

## Principe

La réalité matérielle dicte l'ordre. Chaque couche doit être vérifiée (critères de réussite) avant de passer à la suivante. Pas de couche supérieure construite sur une couche inférieure cassée.

---

## Couche 0 — Substrat physique

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| Forgejo 11.0.1 | CVEs non corrigées (CVE-2026-89094, RCE via dépôt template). Binaire posé 2026-05-20. Aucune trace d'installeur. | Mise à jour vers 15.0.11 LTS ou 17.0.1. Version figée + hash dans `infra/`. Test sur copie du LXC 101. | `forgejo --version` retourne la version cible ; hash binaire = hash officiel ; `forgejo check` sans erreur |
| Sauvegarde Forgejo | Un dump manuel (145 Mo, 2026-10-27). Aucune planification. | Cron dans le LXC : `forgejo dump` quotidien + snapshot Proxmox hebdomadaire | Dump daté de moins de 24h ; `unzip -t` OK ; hash vérifié |
| Proxmox `192.168.0.37` | Fermé. Registre périmé. | Corriger `infra/registry.json` : `proxmox-forge` → `10.10.10.1` | `cynic audit all` ne signale pas de registre périmé ; SSH `root@10.10.10.1` répond |
| Port 22 sur 0.0.0.0 | Surface publique non documentée | Fermer ou documenter dans le registre (champ `exposed_ports`) | Nmap externe ne montre pas 22 ; ou registre le déclare |
| Freebox DMZ | Endpoint connu (`GET /api/v4/fw/dmz/`), non appelé | Lire la DMZ via l'API | Sortie `live_dmz` renseignée dans `cynic node compute approval list` |
| Freebox IPv6 | Endpoint inconnu | Documenter l'absence ou trouver l'endpoint | Aucune affirmation non vérifiée dans le registre |

---

## Couche 1 — Registre et configuration

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| `local-kernel` | `retired` (fait, PR #447) | — | Statut `retired`, `deploy_target: false` |
| `cynic-lxc-headscale` | `candidate`, service `planned` | Décision : déployer ou retirer | Statut `active` ou `retired`, jamais `candidate` sans date |
| Status enum | Pas de valeurs contraintes | Définir les valeurs valides dans un schéma JSON | `cynic audit` rejette une valeur inconnue |
| Politique de version | Pas de version/hash figés | Champ `version` + `sha256` pour tout artefact dans `infra/` | Aucun `latest` dans `infra/` |
| Sauvegarde déclarée | Pas de champ backup | Champ `backup` dans le registre pour chaque service critique | Chaque service critique a un `backup` non vide |
| `proxmox-forge` | IP fermée | Corrigé avec la couche 0 | — |

---

## Couche 2 — Forgejo (source de vérité)

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| Version | 11.0.1 (CVEs) | Mise à jour (voir couche 0) | Version cible, hash vérifié |
| Miroir natif | Retiré (bug recul refs, #14273) | Ne pas remettre. Remplacer par publication explicite | 0 miroir configuré dans Forgejo |
| Publication vers GitHub | Aucun mécanisme | Job `ls-remote` + push sans force, `main` et tags seulement | `git ls-remote github main` = `git ls-remote origin main` après chaque fusion |
| Branches `agent/*` publiques | 14 branches, 3 d'un autre agent | Décision : publication ou privatisation | 0 branche `agent/*` sur GitHub, ou toutes documentées |
| Sauvegarde | Manuel | Planification (voir couche 0) | — |

---

## Couche 3 — VCS et publication

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| Convergence des stores | Forgejo == GitHub | Maintenir après chaque fusion. Avance rapide du checkout principal sans retard | `cynic source divergence` = `AGREE` après chaque session |
| Checkout principal en retard | 17 commits de retard (corrigé) | Règle : fetch + merge --ff-only en début de session | `git rev-list --count HEAD...origin/main` = 0 |
| Branches mortes | 0 ahead sur certaines | `cynic git reconcile --apply` pour les branches à 0 commits | `branch_drift.orphan_count <= 50` |
| Merge gate | 5 critères, 2 bloquants en pratique | Voir détail ci-dessous | — |

### Détail du merge gate (`pr.rs:129-150`, `pull_request.rs:125`)

| Critère | État | Problème | Correction | Critère de réussite |
|---|---|---|---|---|
| 1. `builder_ok` (audit_core_crates) | Jamais bloqué | N'a jamais été déclenché | Vérifier qu'il tourne vraiment | Un échec de compilation bloque la fusion |
| 2. `store_converged` (Forgejo == GitHub) | Bloqué 2 fois | Checkout principal en retard | Convergence en début de session | `AGREE` systématique |
| 3. `secret_lint_passed` | Toujours `true` en dur | Pas de scan réel | Brancher sur un scan de secrets dans le diff | Un secret dans le diff bloque la fusion |
| 4. PR ouverte | OK | — | — | — |
| 5. Mergeable + pas de conflit | OK | — | — | — |

---

## Couche 4 — Build et CI

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| CI | Supprimé (#420) | Reconstruire avec périmètre ciblé (fermeture inverse) | Workflow sur `main` qui compile les crates touchés |
| Admission | `secret_lint = true` en dur, `builder_ok` = audit local | Lire le statut de compilation des PR | `pr.rs:163` lit un vrai statut |
| Routage du pool | HTTP seulement, nœud déconnecté éligible | Exclure les nœuds `blocked_disconnected` ; lire l'éligibilité du registre | `submit` ne sélectionne jamais un nœud déconnecté |
| `submit --wait` | Retourne `completed` sans vérifier l'action | Retourner le verdict de l'action | `submit --wait` retourne `success` ou `failed` avec raison |
| Runner | Arrêté et désactivé sur le builder | Remettre un runner, ou désigner compute-01 | Un runner actif sur un hôte sain |
| Build lease | Dépend du kernel (port 3030) | Remplacer par un endpoint servi par `cynic-node` ou une vérification locale | `cargo check` passe sans kernel |
| Périmètre de compilation | `cargo check --workspace` trop lourd | Fermeture inverse : médiane 7/38 crates | `cargo check -p <crate>` pour les crates touchés |

---

## Couche 5 — Plan de contrôle

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| `cynic-kernel` | Binaire `c26e4d7` non reproductible, service arrêté | Absorber les fonctions utiles dans `cynic-node` | Aucune dépendance à `cynic-kernel` dans le code de `main` |
| 12 consommateurs orphelins | Orphelins, cible `unset` | Traiter un par un (remplacer, retirer, garder) | 0 consommateur orphelin dans `kernel-consumers.json` |
| Cache de l'état souhaité | Aucun | Cache local côté nœud | Un nœud continue sur un état connu pendant une panne du plan de contrôle |
| Claims des agents | Dépend du kernel | Migrer vers un fichier ou un service dans `cynic-node` | `cynic agent claim` fonctionne sans kernel |
| Vérification de bail | Dépend du kernel | Vérification locale ou endpoint dans `cynic-node` | `cynic build` fonctionne sans kernel |

---

## Couche 6 — Fédération et mesh

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| `local-kernel` | `retired` (fait, PR #447) | — | — |
| `cynic-lxc-headscale` | `candidate`, service `planned` | Décision : déployer ou retirer | Statut `active` ou `retired` |
| Mesh | Pas abandonné, pas déployé | Décision (garder, répliquer, abandonner) + déploiement | Headscale actif sur un hôte, WireGuard fonctionnel entre au moins deux nœuds |
| SPOF du plan de contrôle | cortex est le seul hôte | Sortir le plan de contrôle de cortex (voir couche 5) | Le plan de contrôle tourne sur un hôte serveur, pas sur un poste opérateur |
| Vue fédérale vide | Corrigé manuellement + PR #444 | — | `membership tick` ne produit plus `parse view` |
| `tick_once` | Charge avant d'écrire | Écriture atomique (fait dans #444) | Un fichier corrompu est mis en quarantaine, pas fatal |

---

## Couche 7 — Secrets

| Élément | État observé | Correction | Critère de réussite |
|---|---|---|---|
| `freebox-app-token` | Magasin chiffré, identité incohérente | Résoudre `freebox` vs `freebox-app-token` | `cynic secret resolve freebox-app-token` retourne une valeur |
| Commentaire trompeur | Corrigé (#439, non compilé) | Compiler et fusionner #439 | Commentaire reflète le code |
| Gouvernance | Pas de documentation | Documenter que le fichier est chiffré | Le registre mentionne le type de stockage |

---

## Ordre d'exécution (imposé par la réalité matérielle)

```
1. Restaurer une voie de build (compute-01, sans kernel si possible)
   → débloque la vérification de tout le reste

2. Corriger le routage du build (#446)
   → le builder ne sera plus sélectionné s'il est déconnecté

3. Corriger les 12 consommateurs orphelins du kernel
   → un par un, avec la voie de build disponible

4. Mettre à jour Forgejo (sur copie du LXC, avec sauvegarde)
   → nécessite la voie de build pour tester

5. Publication explicite vers GitHub
   → nécessite Forgejo mis à jour (miroir natif)

6. Mesh et fédération
   → dépend du plan de contrôle fonctionnel
```

Chaque couche doit être vérifiée (critères de réussite) avant de passer à la suivante.

---

## Ce qui est fait

- Couche 0 : kernel arrêté, Freebox DMZ lue (code), registre corrigé (partiel).
- Couche 1 : `local-kernel` retiré, `kernel-consumers.json` inventorié.
- Couche 2 : miroir retiré, dump manuel existant.
- Couche 3 : convergence rétablie (Forgejo == GitHub), checkout principal à jour.
- Couche 4 : CI supprimé, fgate supprimé, issue #446 ouverte.
- Couche 5 : PR #444 fusionnée (vue fédérale atomique).
- Couche 6 : vue fédérale corrigée.
- Couche 7 : commentaire corrigé (#439, non compilé).

## Ce qui reste (par couche, par priorité)

1. **Couche 0** : mise à jour Forgejo (décision LTS vs 17.0.1, test sur copie du LXC).
2. **Couche 2** : publication explicite vers GitHub.
3. **Couche 4** : routage du build (#446), remise en service d'un runner, CI reconstruite.
4. **Couche 5** : absorption du kernel (12 consommateurs).
5. **Couche 6** : décision mesh, déploiement Headscale.
6. **Couche 7** : nommage des secrets.

---

## Références

- Issue #446 : routage du build
- PR #447 : registre (fusionnée)
- PR #444 : vue fédérale atomique (fusionnée)
- PR #450 : lecture DMZ (fusionnée)
- PR #439 : commentaire Freebox (non compilé)
- PR #434 : déploiement Forgejo (ouvert)
- `infra/kernel-consumers.json` : inventaire des 12 consommateurs orphelins
- `docs/archive/fleet/fleet-network-architecture.md` : doc d'architecture (archivée)
- `docs/specs/headscale-mesh-coordination.md` : spec mesh
- `contracts/federation-mycelium-policy.json` : politique de fédération
