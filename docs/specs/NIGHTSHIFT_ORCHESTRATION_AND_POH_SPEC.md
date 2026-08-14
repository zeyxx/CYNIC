# Spécification d'Ingénierie Système : Orchestrateur Nightshift & Proof of History

**Status :** Normatif & Contractuel (Layer 0 Kernel State Maintenance — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. ORCHESTRATEUR DE RONDES (crates/cynic-kernel/src/infra/tasks/nightshift.rs│
│    • Boucle asynchrone autonome cadencée par intervalle et CancellationToken│
│    • Schéma : cynic.nightshift_report.v1                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. JOURNAL DE PREUVES D'HISTOIRE (crates/cynic-kernel/src/storage/)         │
│    • proof_of_history.rs : Chaînage cryptographique des transitions d'état.│
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. SANTÉ DES TÂCHES (crates/cynic-kernel/src/infra/task_health.rs)          │
│    • TaskHealth : Heartbeat temps réel et détection de blocage asynchrone.  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Données)

### 1. Structure du Rapport de Ronde Nocturne
```rust
pub struct NightshiftReport {
    pub schema: &'static str,             // "cynic.nightshift_report.v1"
    pub round_number: u64,                // Numéro de ronde strictement croissant
    pub timestamp: String,                // Horodatage ISO-8601 UTC
    pub expired_leases_swept: usize,      // Nombre de baux périmés libérés
    pub state_entries_verified: usize,    // Nombre d'entrées du journal validées
    pub poh_checkpoint_digest: String,    // Condensat SHA-256 de la ronde
}
```

### 2. Invariants d'Ingénierie
1. **Auto-Guérison des Baux Délaissés** : Aucun bail expiré ne peut persister au-delà d'un intervalle de ronde sans être purgé.
2. **Chaînage Déterministe PoH** : Le condensat de la ronde $N$ est une fonction cryptographique déterministe de l'état observé et de l'horodatage.
3. **Tolérance aux Arrêts Brefs (Graceful Shutdown)** : L'orchestrateur écoute le `CancellationToken` et termine proprement sa tâche en cours avant l'arrêt du processus.

---

## III. HOW (Comment s'exécute une ronde Nightshift ?)

```text
 1. Déclenchement du Tick Horaire (`interval.tick()`)
            │
            ▼
 2. Émission du Heartbeat de Santé (`task_health.mark_alive()`)
            │
            ▼
 3. Purge des Baux de Coordination Périmés (`CoordinationLeaseStore::who`)
            │
            ▼
 4. Vérification de l'Intégrité du State Log (`StoragePort::verify_log`)
            │
            ▼
 5. Calcul du Checkpoint PoH (`execute_nightshift_round`)
    └── Hasher SHA-256 : Round + Timestamp + SweptCount + VerifiedCount
            │
            ▼
 6. Diffusion de l'Événement Système (`KernelEvent::StateTransition`)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi une tâche dédiée en arrière-plan ?**
   * *Risque prévenu* : L'accumulation silencieuse de verrous obsolètes et la dérive d'état entre nœuds. Le Nightshift garantit l'homéostasie permanente du système sans action manuelle de l'opérateur.
2. **Pourquoi ancrer chaque ronde dans le Proof of History ?**
   * *Garantie* : Fournit une ligne de temps inaltérable prouvant que les audits et nettoyages ont bien eu lieu à l'heure déclarée.
