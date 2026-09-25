# Spécification d'Ingénierie Système : Moteur E-Score & Preuves de Merkle

**Status :** Normatif & Contractuel (Layer 3 Economic & Identity Substrate — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TYPES & ÉCHELLES ATOMIQUES (crates/cynic-intel/src/escore/types.rs)      │
│    • ScoreUnit = u64 (1 point = 1_000_000 unités entières fixes).           │
│    • EvidenceTier (T1OnChain=100%, T2Attestation=75%, T3Oracle=50%, T4=25%). │
│    • StatType (Karteria, Parrhesia, Anaideia, Philanthropia, Autarkeia...). │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. DÉCROISSANCE & MODIFICATEURS (crates/cynic-intel/src/escore/decay.rs)    │
│    • calculate_decay : Décroissance temporelle exponentielle déterministe.  │
│    • calculate_quadratic_modifier : Pondération quadratique anti-baleine.   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. ARBRE DE PREUVES MERKLE (crates/cynic-intel/src/escore/merkle.rs)        │
│    • Arbre BLAKE3 256-bit générant merkle_root et MerkleProofStep.          │
│    • Preuves d'inclusion vérifiables off-chain et on-chain sans fuite.      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. AGRÉGATEUR MULTI-DIMENSIONS (crates/cynic-intel/src/escore/aggregator.rs)│
│    • Calcul du score vectoriel 7D par portefeuille et classification.       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Formules)

### 1. Invariant d'Arithmétique Entière Fixe (Zéro Float)
Pour garantir qu'un nœud Linux x86_64, un nœud ARM64 et un smart contract on-chain obtiennent **au bit près** le même score :
* **Interdiction absolue des floats (`f32`/`f64`)** dans le consensus et le calcul de réputation.
* Toutes les opérations utilisent des points de base (*basis points* : 10 000 bps = 100%).

### 2. Les 7 Dimensions Métaboliques
$$\vec{E} = (\text{Karteria}, \text{Parrhesia}, \text{Anaideia}, \text{Philanthropia}, \text{Autarkeia}, \text{Poiesis}, \text{Koinonia})$$

* **Pondération par Tier de Preuve** :
  $$\text{Score}_{\text{effectif}} = \text{Score}_{\text{brut}} \times \frac{\text{WeightBps}(\text{Tier})}{10\,000}$$
* **Décroissance Temporelle Journalière** :
  $$\text{Score}(t) = \text{Score}_0 \times \left(\frac{\text{RetentionBps}}{10\,000}\right)^t$$

---

## III. HOW (Comment s'exécute la vérification de citoyenneté ?)

```text
 1. Signaux de Travail / Staking / Contribution
            │
            ▼
 2. Pondération par Tier & Décroissance Temporelle (`calculate_decay`)
            │
            ▼
 3. Agrégation Vectorielle 7D (`compute_wallet_score`)
            │
            ▼
 4. Ancrage dans l'Arbre de Merkle BLAKE3 (`merkle_root`)
            │
            ▼
 5. Génération de la Preuve d'Inclusion (`build_proof`)
            │
            ▼
 6. Vérification cryptographique O(log N) (`verify_proof`)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi l'arithmétique entière fixe (`u64` et BPS) ?**
   * *Risque prévenu* : La non-associativité des flottants IEEE 754 produit des divergences infinitésimales entre architectures de CPU différentes, provoquant des *consensus forks*. L'arithmétique entière garantit un déterminisme à 100%.
2. **Pourquoi BLAKE3 plutôt que SHA-256 pour l'arbre de Merkle ?**
   * BLAKE3 est nativement structuré en arbre (*tree hashable*), supporte l'accélération SIMD/AVX-512 et s'exécute 5 à 10 fois plus vite que SHA-256 tout en conservant une résistance post-quantique supérieure.
3. **Pourquoi une décroissance temporelle exponentielle (*Decay*) ?**
   * Empêche l'établissement d'une "noblesse d'état" : un citoyen ou un nœud inactif perd progressivement son score s'il cesse de contribuer et de prouver son endurance (*Karteria*).
