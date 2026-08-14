# Spécification d'Ingénierie Système : Client Bindings Anchor & Découplage Off-Chain

**Status :** Normatif & Contractuel (Layer 4 SVM Integration — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. BINDINGS CLIENT PURS (crates/cynic-protocol/src/anchor_client.rs)        │
│    • Invariant : Zéro dépendance vers le SDK Solana lourd (solana-sdk).     │
│    • Schéma : cynic.anchor_client.v1                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. INSTRUCTIONS BÂTIES                                                      │
│    • build_record_poh_ix : Construction du payload d'ancrage PoH.           │
│    • build_create_futarchy_market_ix : Création de marché conditionnel.     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. CONTRAT ON-CHAIN CIBLE (programs/cynic-anchor/)                          │
│    • cynic_anchor (ID: CYNiC11111111111111111111111111111111111111)        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Discriminators)

### 1. Invariant de Découplage des Dépendances
* Le SDK Solana complet (`solana-sdk`, `solana-client`) est une dépendance volumineuse (~150 crates transitives) qui alourdit les temps de compilation des agents.
* `cynic-protocol::anchor_client` construit les instructions de manière purement déterministe en calculant les discriminateurs Anchor 8-octets ($\text{SHA256}(\text{"global:"} + \text{name})[0..8]$) et en émettant un `AnchorInstructionPayload` léger et sérialisable.

---

## III. HOW (Comment s'exécute la construction d'instruction ?)

```text
 1. Appel du Builder : `build_record_poh_ix(program_id, auth, epoch, root, digest)`
            │
            ▼
 2. Calcul du Discriminateur 8-octets Anchor
    └── sha256("global:record_poh_checkpoint")[0..8]
            │
            ▼
 3. Dérivation Déterministe du PDA de Compte
    └── sha256(b"cynic_poh" + auth + epoch_le_bytes + program_id)
            │
            ▼
 4. Émission du Payload Typé (`AnchorInstructionPayload`)
    └── Prêt pour soumission RPC / CPI
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Compilation Ultra-Rapide** :
   * Les crates `cynic-protocol`, `cynic-host`, `cynic-cli` n'ont pas besoin de dépendre du compilateur BPF ou de crates Solana lourdes pour interagir avec le smart contract.
2. **Honnêteté et Traçabilité** :
   * Chaque payload d'instruction généré est sérialisé en JSON clair, permettant aux agents de valider les comptes et les montants avant signature.
