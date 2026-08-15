# Spécification d'Ingénierie Système : Surface de Durcissement Hôte (`cynic host harden`)

**Status :** Normatif & Contractuel (Layer 0 Substrate Security — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit cette commande et ses briques ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. CONTRATS PURS (crates/cynic-protocol/src/hardening.rs)                   │
│    • Types : HostHardeningPlan, SshdObservedState, FirewallObservedState    │
│    • Schéma : cynic.host_hardening_report.v1                                │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. MOTEUR DE DOMAINE HÔTE (crates/cynic-host/src/host_hardening/)           │
│    • observe.rs   : Inspection non-mutatrice sshd & pare-feu.               │
│    • reconcile.rs : Calcul du delta et des actions de rollback.             │
│    • apply.rs     : Écriture atomique du drop-in et rechargement de service.│
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. PROJECTION CLI (crates/cynic-cli/)                                       │
│    • Commande : `cynic host harden [--apply] [--json]`                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. QUELS SONT LES DOMAINES AGNOSTIQUES ? (Réponse Formelle)

La commande `cynic host harden` est conçue pour être **strictement agnostique** sur 4 dimensions d'infrastructure :

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        MATRICE D'AGNOSTICISME DU DURCISSEMENT                          │
├──────────────────────────┬─────────────────────────────────────┬───────────────────────┤
│ Domaine Agnostique       │ Variantes Prises en Charge          │ Méthode d'Abstraction │
├──────────────────────────┼─────────────────────────────────────┼───────────────────────┤
│ 1. Distribution Linux    │ • Debian / Ubuntu / Proxmox VE      │ Drop-in standardisé   │
│                          │ • Arch Linux / Manjaro              │ `/etc/ssh/sshd_config │
│                          │ • Alpine Linux / OpenRC             │ .d/99-cynic-hardening │
│                          │ • Fedora / RHEL / Rocky             │ .conf` (RFC OpenSSH)  │
├──────────────────────────┼─────────────────────────────────────┼───────────────────────┤
│ 2. Substrat Pare-Feu     │ • nftables (recommandé CYNIC)       │ Détection dynamique   │
│                          │ • iptables / ufw (fallback legacy)  │ backend sans          │
│                          │ • Windows Firewall (netsh/powershell│ hardcoding d'outils.  │
├──────────────────────────┼─────────────────────────────────────┼───────────────────────┤
│ 3. Gestionnaire Init     │ • systemd (`systemctl reload sshd`) │ Port ServiceManager   │
│                          │ • OpenRC (`rc-service sshd reload`) │ avec rechargement     │
│                          │ • Process direct (`kill -HUP`)      │ non disruptif.        │
├──────────────────────────┼─────────────────────────────────────┼───────────────────────┤
│ 4. Topologie du Nœud     │ • Hyperviseur Proxmox (`vmbr0`)     │ Règle basée sur le    │
│                          │ • Nœud GPU Dédié (Cortex / Baremetal│ sous-réseau IP Synapse│
│                          │ • Cloud VPS / Nœud de build distant │ `10.10.10.0/24`.      │
└──────────────────────────┴─────────────────────────────────────┴───────────────────────┘
```

---

## III. WHAT (Que fait ce système ? Invariants & Données)

### 1. Invariant de Sécurité SSH
* **`PasswordAuthentication no`** : Interdiction absolue des mots de passe sur SSH (seules les clés Ed25519/RSA autorisées).
* **`PermitRootLogin prohibit-password`** : Le compte `root` ne peut jamais être accédé par mot de passe brut.
* **`KbdInteractiveAuthentication no`** : Élimination des invites interactives exploitables par force brute.

### 2. Invariant de Non-Destruction & Rollback
* Chaque action de durcissement génère une action inverse explicite :
  ```rust
  rollback_action: "rm -f /etc/ssh/sshd_config.d/99-cynic-hardening.conf && systemctl reload sshd"
  ```
* Si l'opération échoue, le système peut restaurer instantanément la configuration antérieure sans bloquer l'accès console.

---

## IV. HOW (Comment s'exécute le cycle déterministe ?)

```text
 1. Invocation : `cynic host harden --json` (Mode Lecture Seule)
            │
            ▼
 2. Observation Non-Mutatrice (`observe.rs`)
    ├── Lecture de `/etc/ssh/sshd_config` et drop-ins
    └── Détection du backend pare-feu actif
            │
            ▼
 3. Réconciliation (`reconcile.rs`)
    ├── Calcul des actions manquantes (ex: SSH-DISABLE-PASSWORD-AUTH)
    └── Émission du plan `HostHardeningPlan` (is_converged: false)
            │
            ▼
 4. Application Bornée (`cynic host harden --apply --json`)
    ├── Écriture de `/etc/ssh/sshd_config.d/99-cynic-hardening.conf`
    └── Rechargement atomique de `sshd`
            │
            ▼
 5. Émission du Read-Back Proof (`HostHardeningProof`)
    └── Validation cryptographique de convergence (is_converged: true)
```

---

## V. WHY (Pourquoi cette architecture remplace les scripts Bash ?)

1. **Zéro Échec Silencieux** : Contrairement à un script Bash `sed -i` qui peut corrompre un fichier ou échouer silencieusement si la syntaxe change, l'analyseur Rust vérifie la structure de configuration avant et après.
2. **Auditabilité Machine-Readable** : Le résultat est un JSON typé exploitable directement par les agents autonomes (Codex, Claude, Pi) et les métriques de conformité (`cynic runtime conformance`).
