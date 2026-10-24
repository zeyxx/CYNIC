# CYNIC — Plans d'Audit et de Simplification (Axiome BURN)

> **Objectif :** Guider l'exécution autonome par **Hermes-Agent** pour réduire la complexité cognitive, éliminer le code mort, consolider les couches dupliquées et faire baisser drastiquement le nombre de lignes de code (LOC) tout en générant des traces pour l'auto-évolution des skills.

---

## Invariants d'Exécution pour Hermes-Agent
1. **SSOT Builder :** Ne jamais compiler lourdement sur cortex (RAM instable). Fast-checks autorisés : `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`.
2. **Pas de mutation sans preuve de relecture :** Chaque suppression de code doit être validée par un `cargo check` ou un `cynic audit crate`.
3. **Commit Atomique :** Un commit par sous-tâche avec identité `-c user.name=hermes -c user.email=hermes@agent.local`.
4. **Git Propre :** Jamais de `git add .` ou `git add -A`. Stager explicitement chaque fichier touché.

---

## Plan 1 : Chasse au Code Mort & Surfaces Orphelines dans `cynic-cli`
* **Cible :** `crates/cynic-cli` (47 155 LOC, 143 fichiers).
* **Axiome :** **BURN** (Rasoir d'Occam) & **KENOSIS** (Supprimer l'inutile).
* **Méthodologie :**
  1. **Lever le voile du compilateur :**
     * Éditer [`crates/cynic-cli/src/lib.rs`](crates/cynic-cli/src/lib.rs) : retirer `#![allow(dead_code)]`.
     * Exécuter un check de compilation ciblé pour capturer le rapport d'avertissements de code mort :
       `RUSTFLAGS="-C debuginfo=0" cargo check -p cynic-cli --lib 2>&1 | grep "warning: .* is never used" > /tmp/dead_code_cynic_cli.txt`
  2. **Triage et élagage :**
     * Pour chaque avertissement : vérifier s'il s'agit d'une fonction d'API publique réellement exportée ou d'un artefact abandonné.
     * Supprimer systématiquement les structs, enums, imports et méthodes privées jamais appelées.
  3. **Vérification de non-régression :**
     * `RUSTFLAGS="-C debuginfo=0" cargo check -p cynic-cli --lib`
     * `cynic audit crate --json`
  4. **Receipt attendu :** Baisse immédiate de 1 000 à 3 000 lignes de code mort.

---

## Plan 2 : Fusion et Remplacement de `deps_surface_*` (Micro-Package Manager)
* **Cible :** 10 fichiers `crates/cynic-cli/src/deps_surface*.rs` (2 178 LOC).
* **Axiome :** **BURN** & **TOTALITY**.
* **Problème :** `cynic-cli` héberge un gestionnaire de dépendances éclaté en 10 fichiers alors que son rôle se limite à appeler des sondes de version.
* **Méthodologie :**
  1. **Analyse des interfaces :**
     * Observer les seuls points d'entrée consommés par `root_dispatch.rs` et `dependency_surface.rs` :
       `execute_deps`, `execute_deps_plan`, `execute_deps_audit`, `execute_deps_upgrade`.
  2. **Consolidation :**
     * Fusionner les modèles de données redondants (`deps_surface_model.rs`, `deps_surface_specs.rs`, `deps_surface_pins.rs`) dans un unique sous-module compact `crates/cynic-cli/src/dependency/model.rs`.
     * Rapatrier la logique d'exécution dans un fichier unique `crates/cynic-cli/src/dependency/mod.rs`.
     * Supprimer les 10 fichiers éclaté `deps_surface_*.rs`.
  3. **Receipt attendu :** Réduction de 2 178 LOC à moins de 600 LOC (gain net : ~1 500 LOC).

---

## Plan 3 : Rapatriement de la Logique Métier FCIS (Réseau et Coordination)
* **Cible :** 
  * `crates/cynic-cli/src/network_surface/execution.rs` (1 426 LOC)
  * `crates/cynic-cli/src/agent_surface/coordination.rs` (1 624 LOC)
* **Axiome :** **SOVEREIGNTY** & **FCIS (Zero-I/O Core)**.
* **Problème :** La CLI compile du `nftables`, audite WireGuard et gère des baux d'agents directement dans le binaire d'interface.
* **Méthodologie :**
  1. **Réseau vers `cynic-network` :**
     * Identifier les fonctions pures de génération de règles de pare-feu et d'audit de handshake WireGuard.
     * Les transférer dans `crates/cynic-network/src/domain/firewall.rs` et `crates/cynic-network/src/domain/wireguard.rs`.
     * Réduire `network_surface/execution.rs` à un appel direct aux fonctions du domaine.
  2. **Coordination vers `cynic-node` / `cynic-protocol` :**
     * Rapatrier les 1 600 lignes de gestion de claims d'agents dans les ports de `cynic-node`.
     * La CLI ne doit que formater le JSON ou le texte retourné.
  3. **Receipt attendu :** Respect strict de FCIS. La CLI perd 2 500 LOC de logique métier indue.

---

## Plan 4 : Consolidation du Graphe des Micro-Crates (44 → ~30 Crates)
* **Cible :** Crates satellites < 1 000 LOC dans `crates/`.
* **Axiome :** **CULTURE** (Graphe de dépendances sain) & **BURN**.
* **Candidats à la fusion :**
  * `cynic-discovery-host` (242 LOC) $\rightarrow$ fusionné dans `cynic-discovery` (634 LOC).
  * `cynic-intel-soc` (618 LOC) $\rightarrow$ fusionné dans `cynic-intel` (10k LOC).
  * `cynic-solana-pay` (511 LOC) & `cynic-zeroclaw-solana` (448 LOC) $\rightarrow$ consolidés en un module `cynic-solana`.
  * `cynic-vcs-providers` (2 894 LOC) $\rightarrow$ réintégré dans `cynic-vcs` (10 222 LOC).
* **Méthodologie par fusion :**
  1. Déplacer le code sous forme de module interne dans la crate parente.
  2. Mettre à jour `Cargo.toml` de la crate parente et du workspace.
  3. Mettre à jour `infra/crate-topology.json` (mettre à jour les entrées et dépendances).
  4. Supprimer le dossier de la crate orpheline.
  5. Vérifier avec `cynic audit topology` et `cynic audit crate`.
* **Receipt attendu :** Suppression de 5 à 10 crates, réduction de la dette de topologie L0-L4.

---

## Plan 5 : Cartographie de Dédoublement `cynic-host` vs `cynic-node` (Le Mammouth)
* **Cible :** `crates/cynic-host` (93k LOC) et `crates/cynic-node` (65k LOC).
* **Axiome :** **COHERENCE** & **BURN**.
* **Méthodologie :**
  1. **Matrice des modules jumeaux :**
     * Comparer `cynic-host/src/storage` et `cynic-node/src/storage`.
     * Comparer `cynic-host/src/runtime_layout` et `cynic-node/src/runtime_layout`.
     * Comparer les adapters de métriques et de packages.
  2. **Extraction dans `cynic-types` / `cynic-protocol` :**
     * Toute structure de données identique déclarée des deux côtés doit être unifiée au niveau L0 (`cynic-types`) ou L1 (`cynic-protocol`).
  3. **Suppression de la duplication :**
     * Faire consommer le type unique par l'hôte et le nœud.
* **Receipt attendu :** Début du dégraissage du cœur historique de CYNIC.
