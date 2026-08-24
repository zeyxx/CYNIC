# CYNIC — Primitives d’un organisme auto-conscient

## 1. Self-awareness primitives (kernel)

| Primitive | Implementation concrète | Source |
|---|---|---|
| **Observation interne** (`observe → reconcile`) | `introspection::analyze()` tick toutes les 5 min → compare metrics atomiques (`Metrics`) vs seuils phi | `introspection.rs:35` |
| **Métabolisme** (data flow auto-conscien) | `metabolism::snapshot()` + `metabolism::diagnose()` → ratios digestion/crystallisation/goutte | `metabolism.rs:54,117` |
| **Journal d’état hash-chaîné** | `StateBlock` SHA-256 chaîné → `GENESIS_HASH` bloc 0 | `state_log.rs:79,94` |
| **Preuve de l’histoire** | `Provable` trait pour `StateBlock` → émission `ProofOfHistory` | `proof.rs:33` |
| **Sensory organs** (perception externe) | `senses/mod.rs` → RTK (tokens), Hermes-X (social) | `senses/mod.rs:6-7` |
| **Alertes d’anomalie** | `Alert` struct → streaming via SSE/MCP/WebSocket | `events.rs:31` |

### Auto-contrôle (ce qui est ACTIF)
- Introspecte **metrics atomiques** : `dog_evaluations_total`, `embedding_failures`, `cache_hits`
- Compare **StateBlock trends** : régression `sovereign → degraded`
- Détecte **organ silence** : sources permanentes sans heartbeat > 1h
- **N’importe quel code applicatif** → ne passe PAS par `pipeline::run()` (évite boucle de rétroaction)

### Ce qui N’EST PAS auto-conscient
```rust
// introspection.rs:634 — test explicite que analyze() ne touche PAS pipeline
// Pas d'accès à Judge, EmbeddingPort, DogUsageTracker, VerdictCache
```

## 2. Primitives d’un organisme respecté

1. **Coherence Pattern** : `desired state → observe → reconcile → bounded action → read-back proof`
2. **Truth flows upward** : les régies/vols d’introspection montent dans `StateBlock`
3. **Policy portable / substrate injecté** : les sens sont derrière des ports (`OrganPort`)
4. **Bounded mutations** : les actions sont explicites (`--apply`, `--yes`) avec rollback intent
5. **Unknown is data** : `Option<EnvironmentSnapshot>`, `None` ≠ échec

## 3. Création d’agents incarnés — primitives existantes

| Surface | Fichier | Primitif |
|---|---|---|
| **Runtime inventory** | `agent_runtime/types.rs` | `AgentRuntimeObservation` — état d’observation RO |
| **Adoption** | `agent_runtime/adoption.rs` | `AgentRuntimeAdoptionPlan` — plan d’adoption |
| **Authority** | `agent_access/authority.rs` | `AgentAuthorityBundleRequest` — capabilities/mutation |
| **Bootstrap plan** | `bootstrap_host_plan.rs` | `HostBootstrapCompositePlan` — plan composite |

### Modes d’agent
- `Interactive` : codex-cli, antigravity-cli
- `Autonomous` : hermes-agent, mercury-agent

## 4. Chemin vers finalisation des agents incarnés

### Phase courante → `Autonomous:AutonomousActive`
1. **Observation** → `AgentRuntimeObservation` (probe CLI, status argv, ownership)
2. **Capability graph** → `Vec<CapabilityNode>` — dépendances + retenus + bloqués
3. **Adoption decision** → `AgentRuntimeAdoptionDecision` (policy pure)
4. **Apply authority** → `apply_agent_authority_projection` (mutation bornée)

### Points bloquants à vérifier
- `AgentRuntimeInstallChannel::Unknown` — provenance non classifiée pour hermes-agent
- `AgentRuntimeInstallOwnership::Unknown` — ownership non détecté
- Pas de `AdoptionDesiredState` validé → agent reste en `AutonomousCandidate`

### Actions concrètes
1. Lire `agent_runtime/probe.rs` — voir comment la classification ownership/channel est faite
2. Lire `agent_runtime/adoption.rs` — identifier le critère d’escalade vers `AutonomousActive`
3. Vérifier `agent_access/decision.rs` — `reconcile_agent_access` gate
4. Confirmer `HostBootstrapCompositePlan` accepte l’agent comme surface autorisée

---
*Généré depuis l’analyse source cynic-kernel + cynic-host.*