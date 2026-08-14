# Spécification d'Ingénierie Système : Audits Adversariaux & Matrice Red Team

**Status :** Normatif & Contractuel (Layer 0-4 Security & Antifragility — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système de test adversarial ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. SUITE DE TESTS ADVERSARIAUX (crates/cynic-host/tests/)                   │
│    • adversarial_red_team_suite.rs : Fuzzing, Concurrence & Stress Extrême │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. VECTEURS ADVERSARIAUX COUVERTS                                           │
│    • cynic-host::coordination_lease_store (32 threads concurrents)          │
│    • cynic-intel::futarchy::cpmm (Bail à 100 milliards, pari à 0)           │
│    • cynic-intel::escore::merkle (Falsification de feuilles et de chemins) │
│    • cynic-host::audit_engine (Résistance aux évasions de règles)          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que prouvent ces audits adversariaux ? Invariants de Sécurité)

### 1. Invariant d'Exclusivité sous Contention Concurrente Extrême
* **Test** : 32 threads tentent simultanément de revendiquer le même domaine (`TargetClaimOutcome::Claim`).
* **Résultat Garanti** :
  $$\text{Claimed} = 1, \quad \text{Conflicts} = 31$$
  Aucune condition de course (*race condition*) ne permet à deux agents d'obtenir un bail concurrent sur le même verrou.

### 2. Invariant de Rejet des Écritures Zombies (*Zombie Write Prevention*)
* Tout `LeaseGuard` falsifié (mauvais agent, mauvais lease_id ou token de fencing périmé) est rejeté immédiatement avec `LeaseMutationOutcome::Rejected`.

### 3. Invariant de Finitude & Bornes CPMM ($u128$)
* **Résistance au débordement** : Les calculs de parts $dy = \frac{y \cdot dx}{x + dx}$ sont exécutés en arithmétique `u128` saturante, interdisant tout overflow arithmétique même avec des volumes colossaux ($10^{11}$ unités).
* **Rejet des paris nuls** : Les montants à 0 unité sont rejetés à l'entrée.
* **Rejet post-résolution** : Tout pari soumis sur un marché déjà résolu échoue avec une erreur explicite.

### 4. Invariant de Non-Falsifiabilité Merkle (BLAKE3)
* Toute altération d'un seul bit sur le hash de feuille ou sur le chemin Merkle (`sibling_hash`) rend la fonction `verify_proof` strictement fausse.

---

## III. HOW (Comment exécuter la suite adversariale ?)

```bash
# Exécution de la suite Red Team
cargo test -p cynic-host --test adversarial_red_team_suite
```

---

## IV. WHY (Pourquoi cette rigueur adversariale ?)

1. **Passage de l'Optimisme au Déterminisme Réel** :
   * Les systèmes multi-agents défaillent généralement lors des cas limites (concurrence élevée, déconnexions partielles, tokens périmés). Tester agressivement ces cas garantit l'antifragilité du système.
2. **Protection Économique des Marchés Odysseum** :
   * Fuzzer les formules CPMM garantit qu'aucune combinaison de paris ne peut siphonner la liquidité d'un coffre au-delà de sa réserve allouée.
