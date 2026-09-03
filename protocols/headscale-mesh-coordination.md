# CYNIC Headscale Mesh Coordination — Architecture & Plan de Déploiement

Date : 2026-09-04  
Statut : Spécification normative et plan de réhabilitation  
Autorité : subordonnée à `contracts/foundation.json`, `contracts/framework.json` et `cynic-protocol::network`

---

## 1. But et Justification Architecturale

CYNIC sépare strictement les responsabilités réseau en deux rôles fondamentaux (`contracts/framework.json` § Modularity & Agnosticity) :

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CYNIC MESH ROLES                               │
├───────────────────────────────┬─────────────────────────────────────────────┤
│ MeshRole::Transport           │ WireGuard (noyau Linux, interfaces wg0,     │
│ (Le plan de données)          │ chiffrement L4 des paquets, routes noyau)   │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ MeshRole::Coordination        │ Headscale (contrôle souverain auto-hébergé, │
│ (Le plan de contrôle)         │ distribution dynamique des clés, admission, │
│                               │ NAT traversal STUN/DERP, métriques nœuds)   │
└───────────────────────────────┴─────────────────────────────────────────────┘
```

### Pourquoi le WireGuard statique actuel est insuffisant :
1. **Création dynamique de VMs / LXCs** : Chaque conteneur de build éphémère ou sandbox d'agent exige d'éditer manuellement `wg0.conf` sur toutes les machines du parc. C'est incompatible avec une usine à code industrielle.
2. **Métriques et observabilité** : WireGuard brut ne fournit aucun export structuré (latence, handshakes historiques, débit par nœud) sans parsing texte ad-hoc de `wg show`.
3. **Traversée de NAT et nœuds nomades** : Les runners déportés (cloud Hetzner, GPU loué, laptop opérateur) ne peuvent pas se connecter à une topologie statique sans IP publique fixe.
4. **Adresses fantômes** : La destruction d'une VM ne purge pas ses baux d'IP sans intervention humaine.

---

## 2. Déclaration et Modélisation dans le Registre (`infra/registry.json`)

Conformément à l'audit `registry_claim_audit.rs` (F2c : *« an unmodelled occupant cannot be governed »*), Headscale doit être explicitement modélisé dans le registre canonique :

### A. Réservation d'IP sur le plan Synapse
* **IP réservée** : `10.10.10.40`
* **Noms DNS** : `headscale.synapse.cynic`, `organ-headscale.synapse.cynic`, `cynic-lxc-headscale.synapse.cynic`

### B. Nœud et Service
* **ID nœud** : `cynic-lxc-headscale`
* **Rôle** : `mesh_coordinator`
* **Proxmox ID** : `lxc/100`
* **Service** : `headscale` (port API de contrôle `8080` ou TLS `8443`)
* **Statut de déploiement** : Marqué `planned` dans un premier temps afin de ne pas bloquer les vérifications TCP strictes des autres nœuds (`status_allows_service_checks`).

---

## 3. Sécurité du Transport et Fin du HTTP Clair

L'ancienne instance `organ-headscale` avait été mise en quarantaine car son API écoutait en clair sur `http://10.10.10.40:8080`.

### Règle d'Admission de Transport :
L'API de contrôle Headscale ne doit **JAMAIS** transiter en clair sur un réseau non chiffré :
1. **Option 1 (Overlay-Only)** : L'API de contrôle écoute exclusivement sur l'adresse WireGuard `10.20.0.X` du conteneur, garantissant que tout échange de clés est encapsulé dans WireGuard L4.
2. **Option 2 (mTLS Natif)** : Terminaison TLS via reverse-proxy ou configuration native Headscale (`server_url: https://headscale.synapse.cynic:8443`) avec certificat validé par l'autorité de certificat interne CYNIC.

---

## 4. Cycle de Vie Dynamique des VMs (Intégration Proxmox)

L'adaptateur Rust existant dans `crates/cynic-node/src/adapters/headscale/mod.rs` fournit l'interface :

```rust
pub trait MeshCoordinationPort {
    fn create_join_grant(&self, user: &str, node_id: &str, duration: Duration) -> Result<MeshJoinGrant, MeshCoordinationError>;
    fn revoke_node(&self, node_id: &str) -> Result<(), MeshCoordinationError>;
    fn list_members(&self) -> Result<Vec<MeshMemberObservation>, MeshCoordinationError>;
}
```

### Workflow d'enrôlement :
1. **Ordre de création** : CYNIC ordonne la création d'un hôte Proxmox (ex. `cynic-builder-02`).
2. **Génération du token** : `cynic-node` appelle `HeadscaleAdapter::create_join_grant("cynic", "cynic-builder-02", 10m)`.
3. **Injection Cloud-Init** : Le jeton `preauthkey` à usage unique est injecté dans le payload de démarrage :
   ```bash
   tailscale up --login-server=https://headscale.synapse.cynic:8443 --authkey=${PREAUTH_KEY} --hostname=cynic-builder-02
   ```
4. **Auto-formation** : Le nœud rejoint le réseau, reçoit son IP Synapse, et les autres nœuds apprennent sa route immédiatement via Headscale.
5. **Révocation à la destruction** : Le workflow de destruction Proxmox invoque `HeadscaleAdapter::revoke_node("cynic-builder-02")`, libérant immédiatement l'allocation d'IP.

---

## 5. Observabilité et Métriques en Temps Réel

Headscale expose nativement l'état des nœuds sous forme de métriques Prometheus (`/metrics`) et de JSON (`headscale nodes list -o json`) :

* **Statut de liaison** : Direct (P2P WireGuard) vs Relayed (DERP).
* **Débit et paquets** : `rx_bytes`, `tx_bytes` par nœud pour auditer les coûts réseau de build et d'inférence.
* **Handshake et latence** : `last_seen_unix_s` précis permettant d'éliminer les pings TCP aveugles.

Ces observations alimentent directement le port `NodeRuntimeObservationPort` de `cynic-node`.

---

## 6. Feuille de Route d'Activation

* **Étape 1 (Faite)** : Modélisation formelle dans `infra/registry.json` et réservation IP `10.10.10.40`.
* **Étape 2** : Audit et réveil du conteneur Proxmox `lxc/100` avec liaison socket sécurisée (non-clair).
* **Étape 3** : Câblage de `create_join_grant` dans le usecase de création de builder éphémère.
* **Étape 4** : Migration progressive du `wg0.conf` statique vers la distribution dynamique Headscale.
