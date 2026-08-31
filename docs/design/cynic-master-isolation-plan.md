# Plan Maître CYNIC : Confinement Agentique, Inférence Souveraine, Primitives Blockchain-Like & Métrologie Énergétique

- **Schéma de Référence** : `cynic.master_isolation_plan.v1`
- **Statut** : Document d'Architecture Maître (Normatif / Prêt pour Implémentation)
- **Date** : 2026-08-27
- **Auteurs** : Architecture CYNIC, Inférence & Sécurité
- **Autorité** : Principal `cynic` / Ancrage Stratégique Solana
- **Crates Cibles** : `cynic-protocol`, `cynic-host`, `cynic-node`, `cynic-inference`, `cynic-kernel`, `cynic-intel`, `cynic-cli`

---

## 1. Synthèse Stratégique & Architecture Globale

Le framework CYNIC unifie cinq piliers fondamentaux dans une architecture contract-first déterministe :
1. **Défense en Profondeur (L0–L5)** : Confinement matériel (bwrap/LXC), isolation réseau per-agent (netns default-deny) et contrôle cryptographique strict (OneShotCapability ed25519).
2. **Ingénierie d'Inférence Optimisée APU (Kiely Ch. 5)** : Modèle Roofline formel, quantification rigoureuse, décodage spéculatif avec monitoring du taux d'acceptation $\alpha$ et partitionnement du KV-cache.
3. **Contre-Mesures de Sécurité Avancées** : Rétro-ingénierie complète des failles de l'incident Hugging Face 2026 (neutralisation SSRF, élimination des canaux clandestins, audit CoT en continu).
4. **Primitives Blockchain-Native Distribuées** : Registre d'audit append-only, arbres de Merkle concurrents, consensus épistémique multi-agents et contrats d'attribution de bounties programmatiques.
5. **Accountability Énergétique Formelle** : Mesure physique et thermodynamique des flux de calcul (Joules / token, DRAM bandwidth, impact réseau) couplée à un ancrage de preuve de calcul durable (*Proof-of-Sustainable-Compute*) sur Solana.

```
┌────────────────────────────────────────────────────────────────────────┐
│ L5: Alignement, Futarchie & Gouvernance Épistémique (cynic-judge)      │
├────────────────────────────────────────────────────────────────────────┤
│ L4: Exposition CLI, Gateway Inférence & CoT Tapping (cynic-cli)        │
├────────────────────────────────────────────────────────────────────────┤
│ L3: Mémoire, Ancrage PoH & Preuve Énergétique PoSC (cynic-kernel)      │
├────────────────────────────────────────────────────────────────────────┤
│ L2: Réseau Mesh Synapse, NetNS & Proxy Capability Egress (cynic-node)  │
├────────────────────────────────────────────────────────────────────────┤
│ L1: Substrat, Sandboxing (bwrap/LXC) & Métrologie Joules (cynic-host)  │
├────────────────────────────────────────────────────────────────────────┤
│ L0: Schémas Purs, Contrats Immuables & Types Déterministes (protocol)  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Dimension Blockchain-Native (Au-delà de Solana)

CYNIC internalise les propriétés fondamentales des architectures distribuées décentralisées sans dépendre exclusivement d'un consensus on-chain distant pour son fonctionnement local.

### 2.1 Audit Trail Immuable & Chaînage d'Événements
- **Registre Cryptographique Local** : Le Kernel maintient un registre append-only d'événements (`ProofOfHistory`) où chaque entrée $E_i$ est chaînée cryptographiquement à l'entrée précédente :
  $$\text{Hash}(E_i) = \text{SHA3-512}(\text{Hash}(E_{i-1}) \parallel \text{Timestamp} \parallel \text{AgentID} \parallel \text{PayloadDigest})$$
- **Non-Répudiation & Intégrité** : Toute tentative de falsification ou de suppression d'un événement antérieur brise l'intégralité de la chaîne d'attestation, provoquant le verrouillage immédiat de la node (*fail-closed state*).

### 2.2 Arbres d'État et Preuves de Merkle Concurrentes
- **Merkleisation de l'Espace de Travail** : L'état du workspace de chaque agent (arborescence des fichiers, snapshots Git, baux de coordination `cynic_coord`) est indexé sous forme d'un arbre de Merkle sparse.
- **Preuves d'Inclusion/Exclusion** : Lors de la soumission d'une mutation, l'agent fournit une preuve de Merkle succincte attestant que sa modification s'applique sur l'état exact constaté lors de l'admission de son mandat, interdisant toute écriture sur un état dérivé ou obsolète.

### 2.3 Consensus Épistémique Multi-Agents & Quorums BFT
- **Mécanisme de Décision Byzantine-Tolerant** : Pour les décisions d'architecture critiques ou les validations d'extraction de code, CYNIC ne s'appuie pas sur un agent unique.
- **Quorum d'Évaluation (Dogs Quorum)** :
  - La proposition est soumise à $N$ validateurs épistémiques indépendants (ex: Safety Dog, Architecture Dog, Performance Dog).
  - Un consensus à $2/3 + 1$ (majorité qualifiée BFT) sur le Q-Score ($\ge 0.70$) et le respect des axiomes constitutionnels est exigé pour débloquer l'émission de la `OneShotCapability` de commit.
- **Anti-Collusion** : Les agents validateurs tournent dans des environnements isolés sans communication latérale directe ; leurs interactions transitent obligatoirement par le protocole d'arbitrage du Kernel.

### 2.4 Contrats de Bounties Programmatiques & Escrow d'Actions
- **Résolution Automatisée de Tâches** :
  1. *Engagement de Bounty* : Une tâche est publiée avec une enveloppe de récompense verrouillée sous séquestre cryptographique (*Escrow* local ou on-chain).
  2. *Spécification de l'Oracle de Test* : Les critères de succès sont définis par une suite de tests unitaires et de vérification d'invariants exécutés sous sandbox étanche.
  3. *Libération Atomique* : La complétion réussie (tests verts + preuve de conformité d'isolation + read-back certifié) déclenche automatiquement la libération des fonds vers la clé publique du contributeur ou de l'agent.

### 2.5 Gouvernance Constitutionnelle & Futarchie Locale
- **Suprématie des Contrats** : Le fichier `contracts/foundation.json` agit comme la constitution suprême du framework.
- **Marchés de Prédiction Internes (Futarchie)** : Les propositions de changement d'architecture majeur sont évaluées selon leur impact prédit sur les métriques clés (débit d'inférence, stabilité, consommation énergétique). Une proposition dont la variance empirique dégrade les KPIs constitutionnels est automatiquement rejetée avant allocation de budget.

---

## 3. Dimension Accountability Énergétique & Métrologie Matérielle

L'intelligence souveraine exige une responsabilité stricte sur la dépense énergétique. CYNIC traite l'énergie comme une ressource cryptographique comptabilisée et bornée.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Ancrage On-Chain : Proof-of-Sustainable-Compute (PoSC sur Solana)      │
├────────────────────────────────────────────────────────────────────────┤
│ Couplage Kiely : Joules/Token, DRAM Footprint, Prefill vs Decode       │
├────────────────────────────────────────────────────────────────────────┤
│ Capteurs Substrat : AMD RAPL (CPU/APU), VRAM/DDR Bandwidth, I/O Joules │
└────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Modélisation Énergétique Multimodale
L'énergie totale consommée par une action agentique $A$ est décomposée formellement en trois vecteurs :
$$E_{\text{total}}(A) = E_{\text{compute}} + E_{\text{memory}} + E_{\text{network}}$$

1. **Énergie de Calcul ($E_{\text{compute}}$)** :
   - Mesurée via l'interface matérielle **RAPL** (Running Average Power Limit) du processeur AMD Ryzen et les compteurs de performance matériels (`perf_events` / eBPF energy counters) :
     $$E_{\text{compute}} = \int_{t_{\text{start}}}^{t_{\text{end}}} P_{\text{package}}(t) \, dt \quad [\text{Joules}]$$
2. **Énergie de Mémoire & Transfert de Poids ($E_{\text{memory}}$)** :
   - Sur architecture APU à mémoire unifiée, la lecture des poids quantifiés (GGUF Q4/Q8) et le balayage du KV-cache représentent la composante énergétique dominante en phase Decode :
     $$E_{\text{memory}} = \text{Bytes}_{\text{transferred}} \times \epsilon_{\text{DRAM}} \quad (\epsilon_{\text{DRAM}} \approx 15\text{--}30\,\text{pJ/bit})$$
3. **Énergie Réseau & Confinement ($E_{\text{network}}$)** :
   - Énergie dissipée par l'encapsulation réseau, le filtrage `nftables` de la `netns` et les transferts RPC / mesh Synapse.

### 3.2 Couplage avec le Modèle d'Inférence Kiely (Ch. 5)
CYNIC corrèle directement les métriques d'inférence avec la dépense énergétique unitaire :

| Métrique Kiely | Équation de Rendement Énergétique | Impact Architectural APU / Edge |
| :--- | :--- | :--- |
| **Coût par Token ($J/\text{tok}$)** | $\mathcal{E}_{\text{token}} = \frac{P_{\text{decode}}}{TPS_{\text{gen}}}$ | La réduction de la précision (ex: Q8 $\to$ Q4_K_M) divise le volume de transfert DRAM par 2, réduisant $J/\text{tok}$ de près de 45%. |
| **Amortissement TTFT (Prefill)** | $\mathcal{E}_{\text{prefill}} = \int_{0}^{\text{TTFT}} P_{\text{prefill}}(t)\,dt$ | Le **Prefix Caching** élimine la ré-exécution du prefill sur les invites système récurrentes, économisant des centaines de Joules par tour d'agent. |
| **Efficacité Spéculative ($\alpha$)** | $\eta_{\text{spec}} = \frac{\mathcal{E}_{\text{standard}}}{\mathcal{E}_{\text{speculative}}}$ | Si $\alpha > c_{\text{draft}}$, le gain de temps compense le surcoût de calcul du draft model, rendant l'opération thermodynamiquement rentable. |

### 3.3 Enveloppe Budgétaire Énergétique (`EnergyBudget`)
- **Limitation Déterministe** : Tout mandat agentique (`AgentDelegationEnvelope`) comporte un plafond strict en Joules alloués :
  ```rust
  pub struct EnergyBudget {
      pub max_energy_joules: u64,
      pub max_dram_gigabytes_transferred: u64,
      pub max_watts_peak: u32,
  }
  ```
- **Coupe-Circuit Thermodynamique** : Si un agent entre dans une boucle infinie ou subit une attaque par déni de service de contexte (*context bloat attack*), le dépassement du budget d'énergie coupe immédiatement la session (`SIGTERM` / révocation de capacité).

### 3.4 Preuve de Calcul Durable On-Chain (*Proof-of-Sustainable-Compute* - PoSC)
- **Génération de la Preuve** : À la complétion d'un lot de tâches, le Kernel compile un rapport d'efficience énergétique attestant :
  1. Le volume total de tokens utiles générés ($N_{\text{tokens}}$).
  2. L'énergie exacte consommée en Joules ($E_{\text{total}}$).
  3. L'indice d'efficience unitaire : $\text{Efficiency} = \frac{N_{\text{tokens}}}{E_{\text{total}}} \quad [\text{tokens/Joule}]$.
- **Ancrage Solana** : Ce bilan est haché et inscrit périodiquement sur la blockchain Solana dans le compte PDA de la node (`["cynic_posc", node_id, epoch]`).
- **Incitation Économique & Réputation** : Les nœuds du réseau CYNIC accumulent des points de réputation et des droits de priorité sur les flux d'inférence en fonction de leur ratio prouvé de durabilité et d'efficience énergétique.

---

## 4. Matrice d'Intégration Maître (L0–L5)

| Couche | Confinement & Sécurité (HF Incident) | Inférence & APU (Kiely Ch.5) | Primitives Blockchain-Like | Accountability Énergétique |
| :--- | :--- | :--- | :--- | :--- |
| **L5 : Alignement** | Guardrails anti-jailbreak, blocage des tool calls non alignés. | Surveillance sémantique de l'impact de la quantization. | Consensus de validation multi-agents (Quorum BFT, Futarchie). | Arbitrage des tâches selon le ratio utilité épistémique / coût en Joules. |
| **L4 : Exposition** | Télémétrie CoT continue, coupure immédiate sur anomalie. | Mesure du TTFT, TPS et du taux d'acceptation spéculatif $\alpha$. | Interface d'émission et de résolution de bounties d'agents. | Traçabilité en direct de la consommation de Joules par token affichée au client. |
| **L3 : Mémoire & Kernel** | Anti-rejeu des OneShotCapability, validation ed25519. | Partitionnement strict du KV-cache (`Global` vs `AgentSession`). | Registre d'audit PoH immuable, Merkle trees d'état concurrent. | Agrégation et ancrage de la preuve *Proof-of-Sustainable-Compute* (PoSC). |
| **L2 : Réseau / Nœud** | NetNS dédiée, default-deny egress, proxy sidecar filtré. | Mesh WireGuard isolé pour le transfert KV (P/D split). | Routage de transactions sans mempool et relais d'egress privés. | Comptabilité énergétique des transferts réseau et I/O inter-nœuds. |
| **L1 : Substrat Host** | Sandbox polymorphe (bwrap/LXC), rootfs en lecture seule, tmpfs. | Modèle Roofline actif, mapping mmap GGUF `0640` hors sandbox. | Isolation mémoire matérielle inspirée du runtime SVM eBPF. | Échantillonnage matériel RAPL (CPU/APU/DRAM power metering). |
| **L0 : Protocol** | Contrats `AgentIsolationProfile`, `OneShotCapability`. | Schémas `WeightQuantization`, `ExactTensorGeometry`. | Borsh serialization, schémas typés d'intention et d'escrow. | Schéma normatif `EnergyBudget` et preuves PoSC sérialisables. |

---

## 5. Plan de Déploiement Bottom-Up & Jalons Opérationnels

```
┌────────────────────────────────────────────────────────────────────────┐
│ Jalon 4 : Gouvernance, Bounties & Ancrage PoSC Solana (L4/L5)          │
├────────────────────────────────────────────────────────────────────────┤
│ Jalon 3 : Passerelle Inférence, CoT Tapping & Métrologie Joules (L3/L4)│
├────────────────────────────────────────────────────────────────────────┤
│ Jalon 2 : Isolation Host, NetNS Rootless & Capteurs RAPL (L1/L2)       │
├────────────────────────────────────────────────────────────────────────┤
│ Jalon 1 : Contrats Purs, Types Sécurisés & Modèles Énergétiques (L0)   │
└────────────────────────────────────────────────────────────────────────┘
```

### Jalon 1 : Contrats & Typage Déterministe (`cynic-protocol`)
- Formalisation dans `cynic-protocol` de :
  - `AgentIsolationProfile` (sandbox strategy, netns, UDS bind).
  - `OneShotCapability` étendue aux mutations d'outils (`AgentBoundedAction`).
  - Schéma d'événements CoT `cynic.agent_cot_stream_event.v1`.
  - Structures de métrologie énergétique `EnergyBudget` et `EnergyConsumptionReceipt`.

### Jalon 2 : Confinement Substrat & Mesure Physique (`cynic-host` & `cynic-node`)
- Implémentation du lanceur de sandbox rootless `bwrap` avec montage read-only du rootfs et tmpfs éphémère.
- Actuation du profil réseau `netns` default-deny et proxy d'inspection sidecar capability-aware.
- Intégration du port de lecture RAPL pour l'échantillonnage de la puissance package CPU/APU.

### Jalon 3 : Inférence Sécurisée & Superviseur CoT (`cynic-inference` & `cynic-kernel`)
- Intégration du middleware CoT temps réel dans `inference_gateway.rs` (analyse des tokens dans `<thinking>`).
- Implémentation du partitionnement d'espace de clés de KV-cache (`Scope::GlobalImmutable` vs `Scope::AgentSession`).
- Évaluation dynamique de l'efficience spéculative $\alpha$ et calcul du coût énergétique unitaire en Joules/token.

### Jalon 4 : Consensus Multi-Agents & Ancrage On-Chain (`cynic-intel`, `cynic-cli`)
- Intégration du vérificateur de quorum d'évaluation multi-agents (`solana_safety_dog` + quorums épistémiques).
- Remplacement du flag `--yes` dans la CLI par la vérification cryptographique des capacités One-Shot.
- Pipeline d'émission et d'ancrage périodique des checkpoints PoH et des preuves de calcul durable PoSC sur Solana.

---

## 6. Critères de Réussite & Invariants Terminaux

1. **Étanchéité Vérifiée** : Zéro communication réseau directe possible depuis la sandbox agent sans capacité explicite validée par le proxy.
2. **Non-Falsification** : Toute mutation d'état système ou de fichier est obligatoirement signée par une `OneShotCapability` et enregistrée dans la chaîne PoH.
3. **Vérité Énergétique** : Chaque tâche terminée émet un bilan exact en Joules et en octets mémoire transférés sans aucune valeur interpolée ou devinée.
4. **Finalité Publique** : L'état consolidé du framework est auditable sur Solana via des preuves de Merkle d'arbres compressés vérifiables en $O(1)$.
