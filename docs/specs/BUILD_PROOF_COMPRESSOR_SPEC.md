# Spécification d'Ingénierie Système : Compresseur de Preuves de Build pour l'OPE

**Status :** Normatif & Contractuel (Layer 4 Public Trust & Transparency — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. CONTRATS DE CARTES PUBLIQUES (crates/cynic-protocol/src/build_proof.rs)  │
│    • BuildProofCard : Schéma canonique `cynic.build_proof_card.v1`.         │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. MOTEUR DE COMPRESSION (crates/cynic-host/src/build_proof_compressor/)   │
│    • compress_build_proof : Agrégation des métriques de commit & audits.    │
│    • sanitize_public_text : Élimination absolue des fuites de chemins.      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. PROJECTION PUBLIQUE (public-ui / edge)                                   │
│    • Flux immuable de cartes de travail prouvées et signées.                │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Structure)

### 1. Structure de la Carte de Preuve (`BuildProofCard`)
```rust
pub struct BuildProofCard {
    pub schema: &'static str,          // "cynic.build_proof_card.v1"
    pub card_id: String,               // "bpc-{commit_8}-{digest_8}"
    pub epoch: u64,                    // Époque de consensus
    pub timestamp: String,             // Horodatage ISO-8601 UTC
    pub commit_hash: String,           // SHA Git du commit
    pub crates_affected: Vec<String>,  // Crates modifiées
    pub lines_inserted: usize,         // Volume d'insertion
    pub lines_deleted: usize,          // Volume de purge/refactoring
    pub audit_v2_status: String,       // Statut de conformité Layer 0-3
    pub q_score_bps: Option<u32>,      // Score épistémique φ-borné
    pub proof_digest: String,          // Empreinte SHA-256 de la carte
    pub public_summary: String,        // Résumé assaini sans chemin local
}
```

### 2. Invariants d'Ingénierie
1. **Transparence sans Fuite** : La carte prouve le travail accompli sans jamais exposer de chemins locaux (`C:\Users\`, `/home/`), de secrets ou de données sensibles.
2. **Intégrité Cryptographique** : Le `proof_digest` scelle l'ensemble des métadonnées ; toute altération d'une métrique invalide la carte.

---

## III. HOW (Comment s'exécute la compression ?)

```text
 1. Détection d'un Nouveau Commit / Sprint
            │
            ▼
 2. Extraction des Métriques (Git diff, crates affectées, audit v2)
            │
            ▼
 3. Assainissement du Texte Public (`sanitize_public_text`)
            │
            ▼
 4. Calcul du Digest Cryptographique SHA-256
            │
            ▼
 5. Émission de la Carte `BuildProofCard` pour le Flux Public
```

---

## IV. WHY (Pourquoi ce compresseur est capital pour l'OPE ?)

1. **Scaler la Confiance sans Bureaucratie** :
   * Une One-Person Enterprise n'a pas d'équipe de relations publiques ni d'auditeurs tiers permanents. La preuve de build mathématique remplace la parole par la preuve.
2. **Alignement avec la Futarchie** :
   * Les participants aux marchés de prédiction et les détenteurs de jetons peuvent vérifier factuellement la vélocité et la qualité du code produit.
