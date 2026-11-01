# Spécification Canonique : Verticale Souveraine de Build, Release et Distribution (FCIS / Hexagonale / Pull-Native)

## 1. Contexte et Motivations

L'architecture actuelle de release et distribution de CYNIC souffre d'une fragmentation structurelle :
- Dépendance résiduelle ou tentation de fallback SSH pour l'observation ou le déploiement.
- Modèle de publication en "Push" vers des serveurs d'infrastructure hypothétiques (`artifacts.service.cynic`).
- Projections CLI redondantes (`cynic build trigger` déprécié, `cynic build submit` incomplet, fragmentation des commandes d'artefacts).
- Types concurrents entre crates (`cynic-protocol`, `cynic-build`, `cynic-release`, `cynic-node`).

Cette spécification unifie l'architecture sous les normes :
1. **Domain-Driven Design (DDD)** : Ubiquitous Language unifié dans `cynic-protocol`.
2. **Architecture Hexagonale & FCIS** : Cœur fonctionnel pur, zéro I/O dans le domaine, ports et adaptateurs explicites.
3. **Pull-Native & Content-Addressed** : Distribution via digest immuable servi par le pool de builders HTTP, sans push tiers.
4. **Élimination de SSH comme voie primaire** : HTTP natif authentifié / hermétique comme canal primaire d'observation et de distribution. SSH strictement confiné au breakglass (L0/L1 `cynic-ssh`).
5. **Thin Surface Doctrinale** : CLI mince, élimination totale des rétrocompatibilités dépréciées et des alias superflus.

---

## 2. Modèle de Domaine & Ubiquitous Language (L0 - `cynic-protocol`)

Le domaine de Release et Distribution s'articule autour de 3 agrégats purs :

```
                  ┌──────────────────────┐
                  │   SourceCommitRef    │
                  └──────────┬───────────┘
                             │ (compilation hermétique)
                             ▼
                  ┌──────────────────────┐
                  │  BuildEvidenceRef    │ (digest binaire + logs scellés)
                  └──────────┬───────────┘
                             │ (qualification semver & monotonicité)
                             ▼
                  ┌──────────────────────┐
                  │    ReleaseSeed       │ (epoch, version, artifacts manifest)
                  └──────────────────────┘
```

### Types Canoniques (SSOT L0)
- `ReleaseArtifactRef`: Identifiant content-addressed (`name`, `sha256`, `size_bytes`).
- `ReleaseManifest`: Contrat scellé liant un commit source, un ensemble de `ReleaseArtifactRef`, un epoch et une signature machine.
- `WorkloadSubmission`: Payload d'entrée pour toute action de build (`commit`, `action`, `release_intent: Option<ReleaseIntent>`).

---

## 3. Architecture Hexagonale & Distribution Pull-Native

```
+-----------------------------------------------------------------------------------+
| Cynic-Node Builder (Role: Compute / Builder)                                      |
|                                                                                   |
|  [HTTP Inbound Adapter]                                                           |
|    POST /workloads/submit  --> [BuildScheduler UseCase]                           |
|    GET  /workloads/{id}    --> [Proof & Evidence Port]                            |
|    GET  /artifacts/{sha256}--> [LocalEvidenceArtifactStorage] (Stream bytes)      |
+-----------------------------------------------------------------------------------+
                                      ▲
                                      │ (HTTP Pull par Digest / Token)
+-------------------------------------+---------------------------------------------+
| Cynic Fleet Nodes (Role: Cortex, Bastion, Hosts...)                               |
|                                                                                   |
|  [Release Deployment UseCase]                                                     |
|    1. Query builder for latest verified release manifest (or receive via gossip)   |
|    2. Verify SHA256 digest + signature locally (FCIS pure verification)            |
|    3. PULL artifact binary: GET /artifacts/{sha256}                               |
|    4. Verify downloaded bytes against expected SHA256 (Fail-closed)                |
|    5. Two-slot A/B atomic takeover (cynic-host runtime-seed)                      |
+-----------------------------------------------------------------------------------+
```

---

## 4. Élimination de SSH comme Voie Primaire

- **Principe** : Aucun agent, aucun workflow régulier (build, release, observation fleet) n'invoque SSH par défaut.
- **Canal Primaire** :
  - Métriques & observation nœud : `GET /health`, `GET /status` sur l'agent HTTP du nœud.
  - Build & compilation : `POST /workloads/submit` sur le pool de builders.
  - Distribution : `GET /artifacts/{sha256}`.
- **SSH Breakglass** : Strictement relégué à un flag explicite `--breakglass-ssh` ou binaire dédié de secours sous `cynic-ssh`.

---

## 5. Assainissement du CLI (`cynic-cli`)

Éradication des reliques et unification sous la Thin Surface :
1. **Suppression définitive de `cynic build trigger`** (suppression du dead code et des avertissements de dépréciation).
2. **Consolidation de `cynic build submit`** :
   ```
   cynic build submit --commit <sha> [--action <check|build|build-release|test>] [--release-id <id>] [--wait] [--json]
   ```
3. **Consolidation de `cynic release`** :
   - `cynic release plan --commit <sha>` (calcule le plan de promotion/canari).
   - `cynic release fetch --digest <sha256> --output <path>` (tire directement depuis le builder).
   - `cynic release status` (affiche l'état de synchronisation des nœuds).

---

## 6. Découpage des Tranches d'Exécution (PRs Atomiques)

1. **Tranche 1 (L0/L1 Protocol & Pure Domain)** :
   - Unification des types d'artefacts et de release dans `cynic-protocol`.
   - Élagage des doublons dans `cynic-release` et `cynic-node`.
2. **Tranche 2 (L2 Node Artifact Server & Pull Port)** :
   - Ajout du streaming HTTP `/artifacts/{digest}` dans `cynic-node::usecases::builder_http`.
   - Implémentation du client pull content-addressed sans dépendance externe.
3. **Tranche 3 (L3 CLI & Purge SSH / Trigger)** :
   - Purge de `BuildCommand::Trigger`.
   - Unification des options de `BuildCommand::Submit`.
   - Câblage thin surface pur.
4. **Tranche 4 (CI/CD Automatisé)** :
   - Script/Hook de rebouclage déclenchant automatiquement la compilation et la qualification de release post-PR merge.
