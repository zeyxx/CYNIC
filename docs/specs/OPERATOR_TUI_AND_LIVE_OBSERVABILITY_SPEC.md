# Spécification d'Ingénierie Système : TUI Opérateur & Observabilité Temps Réel

**Status :** Normatif & Contractuel (Layer 4 Operator Projections — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TERMINAL USER INTERFACE (crates/cynic-cli/src/chat_tui/)                │
│    • mod.rs, draw.rs, state.rs, input.rs, theme.rs                          │
│    • Rendu plein écran Ratatui + Crossterm (zéro flicker).                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. SERVEUR DE TÉLÉMÉTRIE TEMPS RÉEL (crates/cynic-observer/src/dashboard.rs)│
│    • Diffusion périodique WebSocket & HTTP (Télémétrie, RPC, E-Score).      │
│    • Dashboard HTML autonome intégré (`dashboard.html`).                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. SOURCES DE DONNÉES AGRÉGÉES (Host, Kernel, Inference, Intel)             │
│    • Baux de coordination actifs (`CoordinationLeaseStore::who`).           │
│    • Statut des slots GPU/CPU (`SlotTracker`).                              │
│    • Métriques E-Score accumulées (`ESCORE_METRICS`).                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Vues)

### 1. Invariants de Performance TUI
1. **Zéro Allocation dans la Boucle de Rendu** : Les buffers de trame Ratatui sont réutilisés d'un tick à l'autre.
2. **Fréquence Bornée (Anti-Spinloop)** : Le thread d'événement écoute sur un timeout de 50 ms pour maintenir une réactivité fluide sans saturer un cœur de CPU à 100%.

### 2. Les 4 Panneaux du Tableau de Bord Opérateur
* **Panneau 1 : Baux de Coordination Actifs** : Liste en temps réel des agents (Codex, Claude, etc.), de leurs domaines verrouillés et du temps restant (TTL).
* **Panneau 2 : Plan d'Inférence & VRAM** : Modèle chargé, slots alloués/libres, latence moyenne.
* **Panneau 3 : Métriques E-Score & Citoyenneté** : Signaux traités, racine de Merkle courante, volume de tokens brûlés (*Ignis*).
* **Panneau 4 : Journal d'État PoH** : Derniers blocs de preuve de l'historique validés par le Nightshift.

---

## III. HOW (Comment s'exécute le cycle d'affichage ?)

```text
 1. Lancement : `cynic chat --tui` ou `cynic-observer`
            │
            ▼
 2. Initialisation du Terminal (`EnterAlternateScreen`, `enable_raw_mode`)
            │
            ▼
 3. Boucle d'Événements (`run_loop`)
    ├── Traitement des Touches Clavier (`input::handle_key`)
    ├── Collecte Non-Bloquante de la Télémétrie (`tokio::sync::broadcast`)
    └── Dessin de la Trame (`draw::draw`)
            │
            ▼
 4. Quitter Proprement (`LeaveAlternateScreen`, `disable_raw_mode`)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi une interface TUI locale plutôt qu'un tableau de bord Web lourd ?**
   * *Garantie d'Autonomie* : Le TUI fonctionne directement dans une session SSH, sur un serveur distant sans interface graphique, sans navigateur et sans connexion Internet externe (*Air-Gapped Ops*).
2. **Pourquoi coupler l'Observer au consensus E-Score ?**
   * Permet à l'opérateur de visualiser instantanément l'impact de son travail sur son score de réputation et sur l'activité des autres agents de la flotte.
