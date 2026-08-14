# Spécification d'Ingénierie Système : Relais Souverain & Opération Distante (Off-Site)

**Status :** Normatif & Contractuel (Layer 5 Git Placement & Sovereignty — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit cette autorité de placement ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. REGISTRE D'AUTORITÉ DE PLACEMENT (infra/repo-remotes.json)               │
│    • Déclaration des remotes, des rôles (sovereign_candidate vs public_relay)│
│    • Schéma : cynic.repo_remotes.v1                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. OBSERVATEUR D'ACCÈS AU DÉPÔT (crates/cynic-host/src/repo_access.rs)      │
│    • Sonde non bloquante avec timeout strict (200 ms).                      │
│    • Résolution DNS dynamique et test de port TCP 3000 / SSH.               │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. CONVERGENCE FORGEJO (crates/cynic-host/src/forgejo_control.rs)           │
│    • Détection de dégradation : Proven, Absent, Unknown.                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que dit la loi CYNIC sur le fonctionnement hors-site ?)

### 1. La Règle de Dualité Souveraine
* **Forgejo (`forgejo.synapse.cynic`)** : Candidat SSOT Souverain sur le LAN Proxmox.
* **GitHub (`https://github.com/zeyxx/CYNIC.git`)** : Relais public (*Public Relay*) pour la collaboration distante et le déploiement CI.

### 2. Invariant de Non-Blocage (*Graceful Degradation*)
* Quand l'opérateur est hors de son domicile (hors du réseau physique Proxmox) et que le tunnel WireGuard n'est pas monté :
  * Le statut de Forgejo bascule en `remote_unreachable_offsite_github_relay_active`.
  * **Aucune commande ne bloque ni n'échoue** : le flux de travail et les pushs basculent automatiquement sur le relais public GitHub (`origin`).

---

## III. HOW (Comment l'organisme gère la bascule ?)

```text
 1. Tentative de Synchronisation (`git push` ou `cynic forgejo plan`)
            │
            ▼
 2. Sonde Rapide TCP (< 200 ms) vers `forgejo.synapse.cynic:3000`
            │
            ├──► Si Joignable (LAN)  : Push vers Forgejo (SSOT Souverain)
            │
            └──► Si Injoignable (WAN): Statut = Absent/Unknown (Off-Site)
                                       Push vers GitHub `origin/main` (Relais Public)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi ne pas bloquer le système si Forgejo est éteint ou inaccessible ?**
   * *Principe de Réalité* : L'infrastructure physique locale peut subir des pannes d'électricité, des coupures Internet ou l'éloignement géographique de l'opérateur. La souveraineté ne doit pas devenir un point de blocage rigide qui paralyse le développement.
2. **Pourquoi enregistrer l'état dans `repo-remotes.json` ?**
   * Évite que les agents autonomes perdent du temps à tenter de se connecter en boucle à une adresse IP locale inaccessible depuis leur environnement d'exécution.
