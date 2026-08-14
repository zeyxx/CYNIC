# Genesis Opérationnelle & "Quantum Treasury Bot"

Ce document formalise la doctrine de lancement (Legal Genesis) et la mécanique programmatique d'accumulation de la trésorerie de TALARIA SAS.

## 1. Legal Genesis : L'Opérateur Originaire
Avant que le réseau (le CYNIC World Order) ne soit massivement distribué et que d'autres opérateurs indépendants (Noeuds, Fondations tierces) ne rejoignent l'infrastructure, l'amorçage exige une gestion centralisée.

*   **Structure d'Amorçage :** Le Fondateur (Jean) agit temporairement comme l'Opérateur Légal (Director/Signatory) des entités offshore créées via **Futard.io** (structures Metalex aux Îles Caïmans).
*   **Responsabilité Limitée & Traçabilité :** Metalex fournit l'enveloppe corporative on-chain. Bien que le Fondateur soit l'opérateur, le code métier (Aura, Odysseum) est distinct de la personne. TALARIA reste l'éditeur logiciel.
*   **Progressive Decentralization :** Dès que le "Nationkit" gagne en traction, le contrôle des entités Metalex est progressivement dilué vers la communauté (via le vote par E-Score ou des délégations multi-sig), effaçant le fondateur de la boucle de responsabilité légale directe.

---

## 2. Le "Quantum Treasury Bot"
TALARIA SAS capte des revenus en monnaie fiduciaire ou stablecoins (B2B, licences, royalties des branches). Pour accumuler la réserve de valeur racine (`$ASDF`) sans manipulation de marché ni exposition au risque de ruine, TALARIA déploie un **Bot de Trésorerie Quantique**.

### Principes de Base
L'objectif n'est pas de faire "pomper" le jeton, mais de l'accumuler silencieusement au meilleur prix, en rendant la pression d'achat imprévisible pour les spéculateurs.

### Algorithme & Paramètres Jouables

1.  **Jitter Temps & Volume (L'Aléatoire Quantique) :**
    *   Au lieu d'acheter `X USDC` tous les vendredis à 14h, le bot utilise une fonction pseudo-aléatoire (VRF) pour fragmenter ses achats.
    *   *Exemple :* Si le budget mensuel est de 10 000 USDC, le bot fera 42 transactions un mois, et 18 transactions le mois suivant, avec des montants allant de 10$ à 1500$, lancés à des heures chaotiques.

2.  **Gating de Sécurité "Runway" (Pare-feu Financier) :**
    *   Le bot est interfacé avec la comptabilité de TALARIA.
    *   *Condition :* `IF (Trésorerie_USDC < 18_mois_de_runway) THEN {Pause_Buybacks()}`.
    *   Cela garantit légalement que TALARIA agit en "bon père de famille" et ne met pas l'entreprise en péril pour un crypto-actif.

3.  **Filtre de Volatilité (Anti-Hype / Anti-FOMO) :**
    *   Le bot observe la volatilité implicite et le RSI de `$ASDF` sur les DEX Solana.
    *   *Condition :* Si le prix a fait un +50% sur 24h, l'algorithme s'arrête de lui-même pendant X jours. L'entreprise ne fournit pas de liquidité de sortie (exit liquidity) aux spéculateurs impulsifs.

4.  **Routage Anti-Sandwich (Protection MEV) :**
    *   Les achats sont routés exclusivement via des agrégateurs sécurisés (Jupiter API).
    *   Le slippage est calculé dynamiquement (très serré) pour s'assurer qu'aucun validateur Solana malveillant ne siphone la trésorerie via une attaque *Sandwich*.

### Conformité Légale (MiCA / AMF)
Étant donné que `$ASDF` est un memecoin sans *Whitepaper* de rendement, TALARIA agit comme un simple investisseur corporatif appliquant une stratégie d'Accumulation TWAP (Time-Weighted Average Price) améliorée. L'absence de régularité prouve qu'il n'y a pas d'intention de *Wash Trading* ou de manipulation des cours.
