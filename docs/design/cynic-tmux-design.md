# cynic-tmux — Surface de Projection Terminale

- **Schema** : `cynic.tmux_surface.v1`
- **Statut** : Conception minimale
- **Date** : 2026-08-27
- **Rôle** : Multiplexage de terminal et persistance de session

---

## Définition

tmux = terminal multiplexer. Il fournit :
- Plusieurs panes dans une seule fenêtre
- Persistance de session (survit aux déconnexions SSH)
- Capture d'output (`capture-pane`)
- Envoi de commandes (`send-keys`)

tmux ne fournit PAS :
- Sandboxing (c'est bwrap/LXC)
- Validation de capabilities (c'est le Kernel)
- Monitoring CoT (c'est le Gateway)
- Audit PoH (c'est le Kernel)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ cynic-tmux (surface de projection uniquement)               │
│                                                             │
│ Pane 0: Log d'audit (passif — capture-pane → Kernel)       │
│ Pane 1: CoT mirror (passif — Gateway écrit, tmux affiche)   │
│ Pane 2: Shell agent (actif — bwrap/LXC gère le confinement) │
│                                                             │
│ tmux ne possède AUCUNE logique de sécurité.                 │
│ Toute la sécurité est gérée par les services systemd.       │
└─────────────────────────────────────────────────────────────┘
```

---

## Fichiers

| Fichier | Rôle | Lignes estimées |
|---------|------|-----------------|
| `cynic-protocol/src/tmux_session.rs` | Type de configuration (layout, panes, session) | ~30 |
| `infra/systemd/cynic-tmux.service` | Lancement au boot (Type=simple, After=network) | ~15 |

**Total** : ~45 lignes. Pas de crate dédiée, pas de script shell.

---

## Type de Configuration

```rust
pub struct TmuxSessionConfig {
    pub session_name: String,      // "cynic-agent-<agent_id>"
    pub socket_path: PathBuf,      // "/run/cynic/tmux/cynic.sock"
    pub layout: TmuxLayout,        // SplitVertical, Quad, Single
    pub panes: Vec<PaneRole>,      // Audit, CoT, Sandbox, Shell
}

pub enum PaneRole {
    AuditLog,       // Pane 0 : capture passive → Kernel
    CotMirror,      // Pane 1 : Gateway écrit, tmux affiche
    Sandbox,        // Pane 2 : bwrap/LXC (tmux ne gère PAS le confinement)
    Shell,          // Pane 3 : opérateur humain (optionnel)
}
```

---

## Invariant Fondamental

> **tmux est un projecteur, pas une autorité.**

- Aucune commande tmux ne valide une capability
- Aucun pane tmux n'exécute un agent sans wrapper de confinement
- tmux ne possède aucune clé privée
- tmux ne décide pas de la politique de sécurité

---

## Intégration avec les Services

```
cynic-kernel.service
    │
    ├──► Pipe-pane capture → Journal PoH
    │
cynic-tmux.service
    │
    ├──► Pane 0 : Audit (capture → Kernel)
    ├──► Pane 1 : CoT mirror (Gateway → tmux)
    └──► Pane 2 : Sandbox (bwrap/LXC → UDS → llama-server)
```

---

## Pourquoi tmux ?

| Avantage | Détail |
|----------|--------|
| Persistance | Survit aux coupures SSH et crashs terminaux |
| RAM <10Mo | Préserve la bandwidth APU pour l'inférence |
| API standard | `send-keys`, `kill-pane`, `list-sessions` pilotables par Rust |
| Multi-clients | Humain + audit simultanément |

---

## Ce qui n'est PAS dans ce document

- Sandboxing → voir `cynic-agent-isolation-plan.md`
- Capabilities → voir `cynic-master-isolation-plan.md`
- CoT monitoring → voir `cynic-master-isolation-plan.md`
- PoH audit → voir `cynic-master-isolation-plan.md`
- Services systemd → voir `infra/systemd/`

---

**Prochaine étape** : Implémenter `tmux_session.rs` et `cynic-tmux.service`.
