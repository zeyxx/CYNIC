# Spécification d'Ingénierie Système : Coordination Multi-Agents & Verrouillage de Baux

**Status :** Normatif & Contractuel (Layer 1 Coordination Protocol — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

1. **Contrats & Schémas Purs** :
   * Fichier : `crates/cynic-protocol/src/coordination_lease.rs`
   * Fichier : `crates/cynic-protocol/src/agent_recursion.rs`
   * Invariant : Zéro I/O, zéro dépendance système, types sérialisables `serde`.
2. **Moteur d'État & Gestionnaire de Baux en Mémoire / Fichier** :
   * Fichier : `crates/cynic-host/src/coordination_lease_store/mod.rs`
   * Responsabilité : Structure concurrente `Arc<RwLock<BTreeMap<String, ActiveLease>>>`, génération de tokens de fencing monotones croissants, purge des baux expirés.
3. **Surfaces d'Exposition MCP & CLI** :
   * Outils MCP : `cynic_coord_claim`, `cynic_coord_renew`, `cynic_coord_release`, `cynic_coord_finalize`, `cynic_coord_who` exposés dans `crates/cynic-mcp/src/routers/coord.rs`.
   * Projection CLI : `cynic coord ...` catalogué dans `infra/surface-registry.json`.

---

## II. WHAT (Que fait ce système ? Invariants & Données)

### 1. Types de Données Centraux
* **`LeaseGrant`** :
  ```rust
  pub struct LeaseGrant {
      pub lease_id: String,       // ID unique: "lease-{target}-{fencing_token}"
      pub fencing_token: u64,     // Entier strictement croissant (anti-split-brain)
      pub expires_at: String,     // Horodatage ISO-8601 UTC
  }
  ```
* **`LeaseGuard`** :
  ```rust
  pub struct LeaseGuard {
      pub agent_id: String,       // Identifiant de l'agent détenteur
      pub target: String,         // Chemin ou domaine sous verrou (ex: "crates/cynic-inference")
      pub lease_id: String,       // ID du bail octroyé
      pub fencing_token: u64,     // Token de fencing vérifié à la mutation
  }
  ```

### 2. Invariants d'Ingénierie Inviolables
1. **Exclusivité Mutuelle** : Pour un domaine donné $D$, il existe au plus un bail valide à tout instant $t$ tel que $\text{expires\_at} > t$.
2. **Monotonie du Token de Fencing** : Tout nouveau bail émis sur le système reçoit un `fencing_token` strictement supérieur au précédent ($\text{Token}_{n+1} > \text{Token}_n$).
3. **Rejet Systématique Hors-Bail** : Toute action de mutation (`Release`, `Renew`, `Finalize`) émise avec un `LeaseGuard` dont le token ne correspond pas au bail actif est rejetée avec `LeaseRejectionCode::GuardRejected`.

---

## III. HOW (Comment les agents et le runtime l'exécutent ?)

```text
       AGENT A (Codex / OMP)                 COORDINATION LEASE STORE             AGENT B (Claude / Pi)
                  │                                     │                                   │
                  ├─────── 1. Claim (target) ──────────►│                                   │
                  │◄────── 2. Grant (Token=1001) ───────┤                                   │
                  │                                     │                                   │
                  │                                     │◄────── 3. Claim (target) ─────────┤
                  │                                     ├─────── 4. Conflict (Agent A) ────►│ (B bascule sur
                  │                                     │                                   │  un autre domaine)
                  │                                     │                                   │
  [Travail & Édition]                                   │                                   │
                  │                                     │                                   │
                  ├─────── 5. Mutate (Finalize/Release)►│                                   │
                  │◄────── 6. Applied (Proof) ──────────┤                                   │
                  │                                     │                                   │
                  │                                     │◄────── 7. Claim (target) ─────────┤
                  │                                     ├─────── 8. Grant (Token=1002) ────►│
```

### Algorithme de Résolution de Conflits (Pseudo-Rust) :
```rust
fn claim(agent_id, target, ttl) -> TargetClaimOutcome {
    let now = Utc::now();
    if let Some(existing) = store.get(target) {
        if existing.expires_at > now && existing.agent_id != agent_id {
            return TargetClaimOutcome::Conflict { target, conflicts: [existing] };
        }
    }
    let token = store.next_token();
    let grant = LeaseGrant { lease_id, fencing_token: token, expires_at: now + ttl };
    store.insert(target, lease);
    TargetClaimOutcome::Claimed { target, grant }
}
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi des baux à durée déterminée (TTL) plutôt que des verrous permanents ?**
   * *Risque prévenu* : La mort ou le crash brutal d'un agent LLM laisserait un fichier verrouillé indéfiniment. Le TTL garantit l'auto-libération sans intervention humaine.
2. **Pourquoi des Tokens de Fencing ?**
   * *Risque prévenu* : Un agent ralenti (*lag réseau*) qui se réveille après l'expiration de son bail pourrait écraser le travail d'un second agent ayant pris le relais. Le token de fencing périmé bloque instantanément l'écriture en retard (*zombie write prevention*).
3. **Pourquoi le découpage par chemin de sous-système ?**
   * Permet à 4 agents différents de travailler simultanément sur `cynic-inference`, `cynic-intel`, `cynic-host` et `docs/` sans aucun risque de collision.
