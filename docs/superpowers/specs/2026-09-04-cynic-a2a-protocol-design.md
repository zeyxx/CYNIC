# Spécification de Design — Protocole Cynic-A2A & Passerelle WebSocket Node

- **Date** : 2026-09-04
- **Auteurs** : CYNIC Core Engine
- **Branche** : `agent/gemini/worktree-convergence-engine`
- **Cadre Méthodologique** : `cynic-layer-model` (v2.0.0), `contracts/foundation.json` (§481, §489), `infra/CRATE_MODELS.md`

---

## 1. Contexte & Motivation : Du JSON Passif au Protocole Vivant

### 1.1 Limites du Modèle Actuel
Actuellement, la coordination inter-agents repose sur `.agent-claims.json` au format passif :
- **Polling aveugle** : Les agents doivent scruter le disque dur pour détecter les changements.
- **Incapacité à détecter les crashs en temps réel** : Si un agent meurt, son bail reste actif jusqu'à expiration du TTL (15 à 60 minutes), bloquant le travail des autres agents.
- **Zéro communication directe** : Aucun moyen pour un agent A (Gemini sur Cortex) de solliciter une review ou une tâche auprès d'un agent B (Hermes sur la VM Proxmox `10.10.10.102`) sans passer par des commandes manuelles de l'opérateur.

### 1.2 Objectif
Implémenter **`Cynic-A2A`**, un protocole vivant, typé et sécurisé d'Agent-à-Agent :
1. **Handshake cryptographique d'enregistrement** (clé Ed25519, validation de `MandateAccount`).
2. **Keepalive / Liveliness actif** (heartbeat périodique, détection de mort et éviction en <45s).
3. **Passerelle WebSocket hébergée dans `cynic-node`** (route `/a2a/v1/ws` sur le réseau Synapse).
4. **Cycle d'échange de tâches structuré** (`TaskOffer`, `TaskAccept`, `TaskProgress`, `TaskComplete`).

---

## 2. Application du `cynic-layer-model` (SoC & Géométrie)

Selon le modèle d'architecture à 5 coordonnées :

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ L6/L5 : AGENT RUNTIME & CLIENTS                                         │
│   ├── Hermes Agent (VM 10.10.10.102) ────► Client A2A                   │
│   └── Gemini Agent (Cortex 10.10.10.100) ─► Client A2A                  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ (WebSocket / Synapse WireGuard 10.10.10.0/24)
┌────────────────────────────────────▼────────────────────────────────────┐
│ L4 : NODE GATEWAY (`cynic-node`)                                        │
│   ├── Axum WebSocket Server (/a2a/v1/ws)                                │
│   ├── Session Registry & Liveliness Watchdog (Arc<RwLock<PeerTable>>)   │
│   └── Task Broker & Eviction Hook                                       │
└──────────────────┬─────────────────────────────────┬────────────────────┘
                   │                                 │
┌──────────────────▼─────────────────┐ ┌─────────────▼────────────────────┐
│ L0 : DROIT PUR (`cynic-protocol`)   │ │ L1 : SUBSTRAT (`cynic-host`)     │
│   - A2AFrame (types purs)          │ │   - source_worktree cleanup      │
│   - Handshake verifier (Ed25519)   │ │   - Eviction actuation           │
│   - MandateAccount validation      │ │   - Process observation          │
└────────────────────────────────────┘ └──────────────────────────────────┘
```

| Coordonnée | Affectation | Justification |
|---|---|---|
| **WHERE** | `cynic-protocol::a2a` (types) + `cynic-node::a2a_gateway` (serveur) | Séparation stricte entre les lois pures et l'exécution réseau. |
| **WHAT** | `A2AFrame`, `AgentSessionRecord`, `A2ALivelinessStatus` | Modèle de trames déterministe et table d'état en mémoire. |
| **HOW** | `axum = "0.8"` + `tokio-tungstenite` sur interface Synapse (`10.10.10.100:8790`) | Réseau privé WireGuard sans exposition internet, SSL/TLS optionnel grâce au chiffrement au repos WireGuard. |
| **WHY** | Éliminer les blocages fantômes et permettre le handoff asynchrone | Remplacement du polling JSON par du streaming réactif. |
| **PROOF** | `A2ASessionObservation` & Journal de déconnexion / éviction | Preuves signées attestant de la présence et des tâches traitées. |

---

## 3. Spécification des Trames `Cynic-A2A` (`cynic-protocol`)

Dans `crates/cynic-protocol/src/a2a/mod.rs` :

```rust
pub const CYNIC_A2A_PROTOCOL_VERSION: &str = "cynic.a2a.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2AFrame {
    // ── 1. Handshake & Enregistrement ──
    Hello {
        agent_id: String,
        node_id: String,
        public_key_ed25519: String,
        capabilities: Vec<String>,
        mandate_id: Option<String>,
        timestamp_ms: u64,
        nonce_signature: String,
    },
    Welcome {
        session_id: String,
        heartbeat_interval_ms: u64,
        heartbeat_timeout_ms: u64,
        granted_capabilities: Vec<String>,
    },
    Reject {
        reason: String,
        error_code: String,
    },

    // ── 2. Keepalive & Vivacité ──
    Ping {
        session_id: String,
        timestamp_ms: u64,
        status: AgentLivelinessState,
        active_tasks: usize,
        memory_rss_bytes: Option<u64>,
    },
    Pong {
        timestamp_ms: u64,
    },

    // ── 3. Échange de Tâches & Handoff ──
    TaskOffer {
        task_id: String,
        envelope: AgentWorkloadEnvelope,
        from_agent: String,
        timeout_ms: u64,
    },
    TaskAccept {
        task_id: String,
        accepted_by: String,
    },
    TaskReject {
        task_id: String,
        reason: String,
    },
    TaskProgress {
        task_id: String,
        turn_index: usize,
        status_message: String,
        progress_pct: Option<u8>,
    },
    TaskComplete {
        task_id: String,
        success: bool,
        evidence_digest: Option<String>,
        read_back_summary: String,
    },

    // ── 4. Déconnexion & Éviction ──
    Goodbye {
        session_id: String,
        reason: String,
    },
    Evicted {
        session_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLivelinessState {
    Idle,
    Working,
    AwaitingArbitration,
    Draining,
}
```

---

## 4. Passerelle WebSocket dans `cynic-node`

### 4.1 Architecture du Serveur
Dans `crates/cynic-node/src/a2a_gateway/` :
- **Listen Address** : `10.10.10.100:8790` (ou paramétrable via `CYNIC_A2A_LISTEN_ADDR`).
- **Endpoint** : `/a2a/v1/ws`.
- **État partagé (`AppState`)** :
  ```rust
  pub struct A2AGatewayState {
      pub sessions: Arc<RwLock<HashMap<String, ActiveAgentSession>>>,
      pub task_router: Arc<RwLock<TaskRouter>>,
      pub eviction_tx: tokio::sync::mpsc::Sender<EvictionEvent>,
  }
  ```

### 4.2 Machine à États de Vivacité & Détection de Mort
Chaque session enregistrée est surveillée par un watcher `tokio::spawn` cadencé toutes les 5 secondes :

```text
Dernier Heartbeat
  │
  ├─► t < 15s  : [HEALTHY]   Normal, l'agent répond.
  │
  ├─► t >= 15s : [DEGRADED]  Alerte interne /attention : heartbeat en retard.
  │
  ├─► t >= 30s : [UNREACHABLE] Les nouvelles tâches ne lui sont plus attribuées.
  │
  └─► t >= 45s : [EVICTED / DEAD]
                     │
                     ├─► Envoi d'une frame `Evicted` (si socket ouvert).
                     ├─► Fermeture du socket.
                     ├─► Émission d'un `EvictionEvent` :
                     │      1. Libération immédiate des baux dans `.agent-claims.json`.
                     │      2. Notification à `source_worktree_reconcile` pour planifier
                     │         le nettoyage du worktree orphelin.
                     └─► Nettoyage de la table des sessions.
```

---

## 5. Client A2A & Cas d'Usage de Référence (Review Inter-Agents)

### 5.1 Scénario Réel de Référence
1. **Gemini (Cortex)** rédige une spécification de design (`docs/...`).
2. Gemini se connecte au socket `ws://10.10.10.100:8790/a2a/v1/ws` et émet un `TaskOffer` destiné à `agent:hermes:main`.
3. **Hermes (VM Proxmox `10.10.10.102`)**, connecté au même socket via Synapse, reçoit le `TaskOffer`.
4. Hermes accepte (`TaskAccept`), lit la spec par référence Git, produit son analyse critique, et renvoie `TaskComplete` avec les observations.
5. Tout le cycle est tracé dans les logs du nœud sans qu'aucun humain n'ait eu à ouvrir une session SSH ou un tmux.

### 5.2 Client Python / CLI Léger
Un client de référence en Rust (`cynic a2a`) et en Python (`~/.hermes/a2a_client.py`) implémente la boucle :
- Reconnexion automatique avec backoff exponentiel.
- Heartbeat en tâche de fond (`asyncio.create_task(heartbeat_loop())`).
- Dispatch des callbacks de tâches.

---

## 6. Plan de Test & Validation

1. **Test unitaire de sérialisation des trames** (`cynic-protocol`) :
   - Vérifier la conformité JSON des frames `Hello`, `Ping`, `TaskOffer`.
2. **Test d'éviction sur timeout** (`cynic-node`) :
   - Démarrer un serveur `A2AGateway` in-memory.
   - Connecter un faux client qui cesse de pinguer.
   - Prouver qu'à $t=45\text{s}$, le client est évincé et l'événement de libération de bail est émis.
3. **Test de boucle fermée** :
   - Connexion simultanée de deux agents simulés avec échange d'une tâche `RequestReview` et accusé de réception.
