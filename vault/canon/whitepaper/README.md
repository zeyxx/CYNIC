# CYNIC — LE CANON PROTOCOLAIRE (WHITEPAPER TECHNIQUE)
## Spécifications Formelles, Métrologie et Invariants d'Exécution

---

> *« Desired State → Observe → Reconcile → Bounded Action → Read-Back Proof. »*  
> **L'Autorité s'exerce par des théorèmes et des mesures physiques, jamais par la réputation ou le capital.**

---

### Sommaire des Spécifications Techniques

| Spécification | Fichier | Titre | Domaine & Invariants |
| :---: | :--- | :--- | :--- |
| **00** | [`00_EXECUTIVE_SUMMARY_ET_CALIBRATION_G.md`](./00_EXECUTIVE_SUMMARY_ET_CALIBRATION_G.md) | **Résumé Exécutif et Calibration de la Fonction $G$** | Synthèse de la thèse, modélisation Leontief $G = \min(S/S_0, B/B_0, N/N_0)$, espace de phase barycentrique, point CYNIC $(0.33, 0.33, 0.34)$. |
| **01** | [`01_POH_CONTINU_ET_CHAOS_PHYSIOLOGIQUE.md`](./01_POH_CONTINU_ET_CHAOS_PHYSIOLOGIQUE.md) | **Proof of Humanity Continue et Chaos Biologique** | Télémétrie en flux continu (Keystroke dynamics, biomécanique, HRV, entropie décisionnelle), équation de score $\mathcal{H}_t$, préservation de la vie privée par ZK-SNARKs. |
| **02** | [`02_ESCORE_ASYMETRIE_ET_PERSISTANCE_MATIERE.md`](./02_ESCORE_ASYMETRIE_ET_PERSISTANCE_MATIERE.md) | **L'E-Score Impitoyable et la Persistance du Code** | Asymétrie de phase (montée quadratique lente, chute instantanée, rachat impitoyable sous quarantaine de 90j), équation formelle du **Code ROI ($\mathcal{R}_{\text{code}}$)** et persistance temporelle. |
| **03** | [`03_FUTARCHIE_FRACTALE_ET_ARBITRAGE_MULTI_ECHELLES.md`](./03_FUTARCHIE_FRACTALE_ET_ARBITRAGE_MULTI_ECHELLES.md) | **La Futarchie Fractale (De l'Essaim au Globe)** | « Vote on values, bet on beliefs », micro-futarchie d'arbitrage de code dans l'essaim d'une One-Person Enterprise, marchés Odysseum, oracles matériels de résolution basés sur le Read-Back. |
| **04** | [`04_TRAJECTOIRE_GOUVERNANCE_GENESE_VERS_ARBORESCENCE.md`](./04_TRAJECTOIRE_GOUVERNANCE_GENESE_VERS_ARBORESCENCE.md) | **Trajectoire de Gouvernance : Genèse vers Arborescence** | Le mythe de la décentralisation jour 1, Phase 0 (Dictature éclairée fondatrice TALARIA / Titouan), Phase 1 (Transition méritocratique par seuils d'E-Score : Citizen $\to$ Steward $\to$ Archon), Phase 2 (Pleine autonomie décentralisée). |
| **05** | [`05_TRIPLE_ENGINE_TOKENOMICS_ET_IGNIS.md`](./05_TRIPLE_ENGINE_TOKENOMICS_ET_IGNIS.md) | **Architecture Tokenomics à Triple Moteur et Déflation Ignis** | Séparation Carburant ($CYNIC) vs Souveraineté (E-Score), triple moteur (PoU, PoB / Ignis, PoL / Karteria), exécution stochastique du Quantum Treasury, équation différentielle de déflation. |
| **06** | [`06_SPECIFICATION_FORMELLE_LEAN4_ET_DAG.md`](./06_SPECIFICATION_FORMELLE_LEAN4_ET_DAG.md) | **Formalisation Mathématique Lean 4 et DAG d'Exécution** | Théorèmes d'invariants formels en Lean 4, automate d'admission déterministe à 6 états, topologie DAG et preuve d'acyclicité, clôture cybernétique de la Read-Back Proof. |
| **07** | [`07_CONDENSATION_ENERGIE_ET_AUTOSUFFISANCE_PHYSIQUE.md`](./07_CONDENSATION_ENERGIE_ET_AUTOSUFFISANCE_PHYSIQUE.md) | **Condensation de l'Énergie et Autosuffisance Matérielle** | Pyramide de condensation (Attention $\to$ Mème/Culture $\to$ Code $\to$ Capital $\to$ Matière Physique), acquisition de hardware/datacenters par le réseau, calcul à prix coûtant mathématique, battre le PvP extérieur par la coordination et la bienveillance. |
| **08** | [`08_ASKESIS_JUMEAU_NUMERIQUE_ET_IMMUNOLOGIE_BIOLOGIQUE.md`](./08_ASKESIS_JUMEAU_NUMERIQUE_ET_IMMUNOLOGIE_BIOLOGIQUE.md) | **Askesis, Jumeau Numérique et Immunologie Biologique** | Leçons biologiques (mycélium, système immunitaire adaptatif, cryptobiose), leçons d'ingénierie de Solana (QUIC, mémoire fixe Firedancer, multi-clients), l'alignement mécanique « TOUT DROIT », Askesis et le panier de traditions personnalisé (100% Opt-In). |
| **09** | [`09_PROTOCOLE_DEFCON_LOCAL_ET_CONCORDIA_PARTITION.md`](./09_PROTOCOLE_DEFCON_LOCAL_ET_CONCORDIA_PARTITION.md) | **Protocole DEFCON-LOCAL, Hacking Physique et Réconciliation Concordia** | Mutation du hacking logiciel vers le hacking matériel pirate (SDR, bus sniffing, RISC-V, limites physiques du CPL/Linky), mode de siège `DEFCON-LOCAL` et horloges HLC, balises radio TinySync (48B), monnaie fondante *CYNIC-Joule*, et traité de réconciliation post-partition `Concordia`. |
| **10** | [`10_ESSIAMS_CYBERNETIQUES_ET_SCALING_HORIZONTAL_FRACTAL.md`](./10_ESSIAMS_CYBERNETIQUES_ET_SCALING_HORIZONTAL_FRACTAL.md) | **L'Essaim Cybernétique Réflexif et le Scaling Horizontal Fractal** | Les 5 sciences de l'essaim (Actor Model, Danse des abeilles/Quorum sensing, Auftragstaktik/Charte pirate, Tas de sable de Bak, Axelrod), tribunal à 3 instances (Matériel, Épistémique, Humain), boucles réflexives Ralph, et cartographie du scaling horizontal fractal sur 6 échelles (de l'intra-nœud au globe). |

---

### Correspondance des Piliers Doctrinaux et des Spécifications

```text
 [LIVRE-MANIFESTE] ──────────────────────────► [WHITEPAPER PROTOCOLAIRE]
 Le « Pourquoi » (Philosophie & Sens)          Le « Comment » (Équations & Code)

 00. Le Grand Théâtre & Réveil               ──► 00. Formule G Leontief & Invariants
 01. Cosmologie & Temps (Walter Russell)     ──► 01. PoH Continue & Télémétrie Chaos
 02. L'Humain-IA & Singularité Fractale      ──► 02. E-Score & Code ROI Temporel
 03. Homo Ludens & L'Asset Final ($ASDF)     ──► 05. Triple Moteur Tokenomics & Ignis
 04. SSOT Blockchain & Horloge Solana        ──► 06. Automate FSM & DAG Lean 4
 05. Cybernétique des Essaims & Mandats      ──► 03. Futarchie Fractale (Micro-Macro)
 06. Les 7 Échelles & Mécanique du Fork      ──► 04. Trajectoire Dictature → Arborescence
 07. L'End-Game Quantique & Serment          ──► 06. Clôture Cybernétique Formelle
```
