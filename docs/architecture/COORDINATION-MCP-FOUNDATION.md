# Coordination MCP Foundation (CYNIC) — v1.0

Policy authority: `docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md`.

This document describes the current coordination implementation. It must not
redefine organism policy. When there is a disagreement, the policy matrix
decides the dimension/layer boundary, and this file maps that boundary to
concrete code.

## 1) Ground truth (CYNIC actuel)

- **Kernel (source de vérité)**: endpoint REST `/observe` + `/observations` + `/coord/*` + `/health`.
- **MCP portable**: `crates/cynic-mcp` expose les outils ci-dessous via `cynic-coord`.
  - `coordination_status_publish`
  - `coordination_status_read`
  - `coordination_thread_reply`
  - `coordination_artifact_share`
  - `coordination_decision_record`
  - `coordination_contract_manifest`
- **Contrat Canonique**: `axioms/coordination-protocol-v1.json`
  - `routing_model.target = coord://{project}/{workstream}[#{thread_id}]`
  - `permission_model` + liste des opérations.
- **Contrat infra/réseau séparé**: `axioms/infrastructure-mcp-v1.json`
  - couvre les opérations registry/IPAM/fleet/mesh/service.
  - les outils Proxmox bruts sont des adapters, pas la surface portable.
- **Profil consommateur**: `axioms/consumer-profiles/privacy-ethereum-framework-v1.json` (version opérationnelle).
- **Adapter Slack**: `organs/slack` est branché via `organs/slack/src/router.py` (routing table + env, sans hard-code canal Slack en dur).
- **Couche transport**: `organs/slack/src/dispatch.py` poll/query + livraison + observabilité (`/observe`).

### Ce qui manque encore / fragilité

- Certaines implémentations anciennes du routeur/adapter mélangent encore logique métier et transport.
- La projection des droits/clients est encore incomplète pour un scénario multi-agents strict (les profils existent, mais la gouvernance runtime est partielle).
- Les observateurs de consommation (mètres de qualité de filtrage/permission) ne sont pas encore consolidés en un seul document de garde-fous.
- Les politiques transversales doivent rester dans la matrice organisme; ce
  document ne doit contenir que le mapping implementation/contrat.

## 2) Gaps de portabilité détectés

1. **Risque de filtre incomplet côté kernel**
   - Avant correction, `/observations` ne permettait pas de filtrer `project/workstream/thread_id` de façon native.
   - Effet: pagination/`limit` pouvant tronquer le contexte du thread.
2. **Logique d’extraction de target**
   - Extraction `workstream` depuis `target` présentait une faiblesse de parsing.
3. **Surcouche “Slack-centric” encore visible dans certains chemins**
   - Historiquement utile, mais politiquement elle ne doit pas devenir la grammaire métier.
4. **Inventaire client/SoC**
   - La distinction “contrat sémantique vs transport” doit être explicitée pour chaque acteur (CLI, future UI, webhook).

## 3) Architecture cible (stable, transport-agnostique)

### Couches

- **Kernel truth layer** (`cynic-kernel`)
  - Persist les observations coordonnantes.
  - Sert d’interface de requête/filtrage.
  - N’édicte pas les règles de diffusion sociale.
- **MCP abstraction layer** (`cynic-mcp`)
  - Contrat d’opérations.
  - Applique validation de politique/permission.
  - Traduit vers `/observe` et `/observations`.
- **Infrastructure MCP layer** (`cynic-mcp` + organs infra)
  - Expose des opérations sémantiques telles que `network.ipam.check`,
    `fleet.node.provision`, `fleet.node.exec` et `service.health`.
  - Masque les adapters Proxmox/systemd/WireGuard derrière un contrat
    déterministe, registry-first.
- **Adapter layer**
  - Slack aujourd’hui, autres adaptateurs demain.
  - Responsable uniquement du transport et de la présentation.

### Primitives minimales

- `coordination_contract_manifest(client?, project?, operation?, include_default_project_profile?)`
- `coordination_status_publish`
- `coordination_status_read`
- `coordination_thread_reply`
- `coordination_artifact_share`
- `coordination_decision_record`

### Modèle d’identité

- `client` explicite (ex: `codex`, `claude`, `antigravity`, `hermes`).
- Résolution stricte des opérations via policy.
- Scope projet: par défaut `*`, à réduire quand un environnement le permet.

### Modèle de permissions recommandé

- `strict` par défaut, deny-by-default hors profils listés.
- Validation côté MCP de:
  - `operation` demandée,
  - `project` et champs obligatoires,
  - capacité du client.

### Mapping projet/canaux

- Canonique: `project + workstream + thread_id` stocké dans le contrat.
- Transport: résolu via routing table (`infra/registry.json` + env) sans logique métier.

### Confidentialité

- Déclaration claire: **Slack est un transport**, pas la source d’autorité.
- Données sensibles: minimales, non obligatoires dans les payloads transport.
- Les décisions, artefacts et commits restent ancrés par les références repo/CI/consommateur.

## 4) Changements déjà appliqués

- `crates/cynic-kernel/src/api/rest/data.rs`
  - `GET /observations` prend désormais `project`, `workstream`, `thread_id`, `target`.
- `crates/cynic-kernel/src/domain/storage/mod.rs`
  - ajout de `list_observations_filtered(...)` à `ActivityStorage`.
- `crates/cynic-kernel/src/storage/{memory,reconnectable,surreal}.rs`
  - implémentation/forward de `list_observations_filtered`.
- `crates/cynic-kernel/src/storage/surreal/activity.rs`
  - filtres coordination robustes avec fallback `value` + parsing de `target`.
- `crates/cynic-kernel/src/api/mcp/mod.rs`
  - implémentation mock de `list_observations_filtered` pour DownStorage.
- `crates/cynic-mcp/src/proxy.rs`
  - correction du parsing `coord://...` et robustesse de `coordination_filter_row`.

## 5) Changements restants

- Finaliser une matrice de projection client/perf/risque de permissions (Codex/Claude/Antigravity/Hermes).
- Ajouter un observatoire de conformité “who uses what” pour éviter l’érosion du contrat.
- Mettre en place au moins un second adapter de transport pour prouver la portabilité réelle.
- Valider une version mineure de `project-thread` canonicalization dans tous les producteurs.
- Implémenter `axioms/infrastructure-mcp-v1.json` côté MCP pour sortir
  les agents des outils provider-level (`cynic_proxmox_*`).

## 6) Contrat de consommation — privacy-ethereum-framework

Consommation minimale requise par le repo consumer:

- `coordination_status_publish`
- `coordination_status_read`
- `coordination_thread_reply`
- `coordination_artifact_share`
- `coordination_decision_record`

Payload/lecture attendus:

- `project` et `workstream` requis sur toutes les opérations de coordination.
- `status.publish`/`thread.reply`/`artifact.share`/`decision.record` structurés sous contrat.
- `decision` + `source_artifact` + (`artifacts`/`open_points`) pour décisions archivables.

`coordination_status_read` côté consumer doit requêter `domain=coordination`, avec `project` obligatoire, et filtrer côté projet/workstream/thread_id selon besoin.
