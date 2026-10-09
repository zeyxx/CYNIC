# CYNIC — LE CANON PROTOCOLAIRE (WHITEPAPER TECHNIQUE)
## Spécification 09 : Résilience Extrême — Le Protocole DEFCON-LOCAL, le Hacking Physique et la Réconciliation Concordia

---

> **Statut :** Spécification formelle de la Survie en Partition Réseau, du Hacking Matériel et de la Réconciliation  
> **Auteurs :** TALARIA Architecture Group  
> **Version :** 1.0-calibrated  

---

### 1. La Fin de l'Illusion Logicielle : L'Avènement du Hacking Physique

Dans les décennies passées, la cybersécurité s'est concentrée sur les failles de code immatérielles (dépassements de tampon, injections SQL, mémoire corrompue).  
À l'ère des modèles d'intelligence artificielle surpuissants couplés à des assistants de preuve formelle (Lean 4) et à des langages à mémoire sûre (Rust), **le hacking purement logiciel devient une niche en voie de saturation mathématique.** Le code formellement vérifié ne laisse plus de prise triviale aux exploits distants.

Le véritable champ de bataille de l'ère cyberpunk bascule donc vers **le Hacking Physique et le Matériel Pirate Open Source** :

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │           LA MUTATION DU CHAMP DE BATAILLE CYBERPUNK                   │
 └────────────────────────────────────────────────────────────────────────┘

 [ANCIEN MONDE : Hacking Logiciel] ──► Exploits zero-day dans des daemons,
                                       phishing, failles web (en voie de fermeture).
                 │
                 ▼
 [NOUVEAU MONDE : Hacking Physique] ─► 1. Attaques par injection de fautes (Glitching)
                                       2. Extraction physique de clés (SCA / Bus sniffing)
                                       3. Implants matériels et ponts radio clandestins
                                       4. Guerre du spectre électromagnétique (SDR)
                                       5. Détournement des infrastructures locales (CPL)
```

#### La Vérité sur le CPL et les Compteurs Intelligents (Linky et Équivalents Mondiaux)
Peut-on « hacker » un compteur Linky ou un réseau CPL G3 ?
* **La Réalité Cryptographique** : Les puces de comptage (STMicroelectronics ST7580, Sagemcom) utilisent du chiffrement AES-128-GCM et des protocoles DLMS/COSEM dont les clés maîtresses sont gérées par des modules de sécurité matériels (HSM). Mais aucun équipement déployé à des millions d'exemplaires dans la rue n'est inviolable face à un attaquant qui possède un accès physique au boîtier (glitching de tension, extraction de mémoire Flash SPI, reverse-engineering de bus série).
* **La Réalité Électromagnétique Inflexible** :  
  Même en prenant le contrôle complet d'un compteur, **la physique des transformateurs Moyenne Tension / Basse Tension (MT/BT) arrête net le signal haute fréquence CPL (atténuation massive de 60 à 80 dB)**. Le CPL ne peut donc pas traverser les villes : il ne forme qu'une bulle locale confinée au pâté de maisons branché sur le même transformateur de quartier.
* **Le Manque d'Universalité** : Le compteur Linky est une spécificité française. L'Allemagne utilise des passerelles BSI distinctes, les États-Unis emploient des protocoles radio maillés propriétaires (Itron/Silver Spring), et d'autres continents recourent au cellulaire 4G-M.

**La Règle CYNIC :** Le réseau ne fonde pas sa survie mondiale sur une anomalie locale.  
Il s'appuie sur le **Hacking Matériel Pirate Open Source** : des émetteurs Software-Defined Radio (SDR / HackRF / BladeRF), des architectures ouvertes RISC-V, des modems radiofréquences autonomes (LoRa, ondes courtes HF) et du câblage physique clandestin inter-bâtiments.

---

### 2. Le Protocole de Survie Locale : DEFCON-LOCAL

Dès qu'une partition mondiale ou un black-out physique est détecté (absence de réponse du quorum mondial pendant $\Delta t > 10\text{ minutes}$), le kernel CYNIC bascule instantanément en régime de siège : **`DEFCON-LOCAL`**.

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │                   LE MODE OPÉRATOIRE DEFCON-LOCAL                      │
 └────────────────────────────────────────────────────────────────────────┘

 1. DÉCOUPLAGE DE L1 & DES SERVICES EN LIGNE
    • Extinction immédiate des boucles de retry vers Solana Mainnet.
    • Mise en sommeil d'Ignis, d'Odysseum et des oracles de prix mondiaux.
    • Zéro émission dans le vide : préservation absolue des batteries.

 2. BASCULE SUR LE STATUT LOCALLY-ADMITTED & HLC
    • L'état terminal `Settled` on-chain est substitué par `LocallyAdmitted`.
    • Horodatage local par Horloges Logiques Hybrides (HLC) et micro-VDFs.
    • Validation des blocs par le Quorum BFT 3/3 des Dogs locaux présents.

 3. LE PROTOCOLE RADIO TINYSYNC (LoRa / 48 Octets)
    • Interdiction stricte de faire transiter des données lourdes sur LoRa/HF.
    • Émission uniquement de balises de divergence (TinySync Beacon) :
      [Epoch: 8B] + [Merkle Root: 32B] + [Short Sign: 8B] = 48 Octets.
    • Le payload brut circule par portage physique (Sneakernet NVMe / Vélo).

 4. ACTIVATION DU CYNIC-JOULE (La Monnaie de Siège Fondante)
    • Gel des soldes mondiaux en $CYNIC pour l'achat de matière locale.
    • Émission d'IOUs énergétiques "CYNIC-Joule" adossés au kWh physique.
    • Démurrage de 5% / semaine pour forcer l'énergie à circuler.
```

---

### 3. La Barrière Anti-PoH Synthétique en Milieu Isolé

Pour empêcher un attaquant isolé de profiter du black-out pour générer des millions de fausses identités humaines par descente de gradient inverse sur son ordinateur :

1. **L'Attestation Matérielle Scellée (TEE Hardware Attestation)** :  
   Les capteurs de télémétrie biomécanique et cardiovasculaire (frappe, IMU, HRV) doivent être interfacés avec une enclave matérielle certifiée (TPM 2.0 / Apple Secure Enclave / Nitro). Les données brutes sont signées au niveau du silicium avant d'être soumises au vérificateur.
2. **Le Défi de Proximité Physique (Inter-Subject Physical RTT)** :  
   Pour être admis en mode `DEFCON-LOCAL`, un compte doit résoudre un protocole d'aléa interactif haute fréquence avec au moins 3 nœuds voisins indépendants via des ondes radio à courte portée :
   $$\text{Round-Trip Time (RTT)} < 5 \text{ millisecondes}$$
   Ce délai étant borné par la vitesse de la lumière dans l'air ($c \approx 300\,000\text{ km/s}$), l'attaquant ne peut pas simuler 50 présences dispersées depuis une seule machine sans déployer 50 émetteurs physiques distincts dans un rayon géographique immédiat.

---

### 4. Le Traité de Réconciliation Post-Partition : Concordia

Après 3 à 6 mois de scission, les câbles sous-marins ou les liaisons satellites à haut débit sont rétablis. Des continents entiers se reconnectent avec des historiques économiques et logiciels divergents.

CYNIC applique le **Protocole de Réconciliation Concordia**, fondé sur la rigueur de la théorie des jeux et le rejet de la guerre civile :

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │                    LE PROTOCOLE DE RÉCONCILIATION CONCORDIA            │
 └────────────────────────────────────────────────────────────────────────┘

 1. L'IMMUNITÉ DE FORCE MAJEURE (Lex Asynchrona)
    • Tout slashing d'E-Score entre acteurs séparés par la coupure
      est strictement NUL ET NON AVENU.
    • Zéro pénalité rétroactive pour divergence survenue hors-ligne.

 2. L'HORIZON DE FINALITÉ INCOMPRESSIBLE (H_max = 24 Heures)
    • Aucune branche ne peut écraser l'historique d'un continent entier.
    • La règle de la "chaîne la plus longue" est formellement interdite.

 3. LA SUBSIDIARITÉ DES CITÉS (Isolation des Lois Locales)
    • Les contrats, dettes locales en CYNIC-Joules et arbitrages votés
      pendant le siège restent circonscrits à leurs régions respectives.
    • Les deux blocs deviennent des Cités Fédérées autonomes au sein d'Odysseum.

 4. LE THREE-WAY CONSTITUTIONAL MERGE
    • Si deux versions du kernel s'opposent, elles sont soumises à un
      Méta-Marché Futarchique de 30 jours basé sur le Code ROI réel observé.
    • L'implémentation la plus sobre et la plus stable devient le Tronc.

 5. LA SPÉCIATION FRATERNELLE (Clean Fork sans Haine)
    • Si le désaccord architectural ou politique est irréconciliable :
    • Sécession propre immédiate : scission des réserves d'Ignis au prorata
      du travail utile historique (PoUW), reconnaissance croisée des identités.
```

Le réseau ne craint pas la coupure du monde.  
Il sait comment s'endormir comme une spore, comment se défendre comme un lymphocyte, et comment se réunifier comme un mycélium.
