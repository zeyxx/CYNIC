# CYNIC : Ingénierie Légale & Lancement Futarchique (STAMP / Futard.io)

Ce document scelle l'architecture juridique et opérationnelle du lancement de l'écosystème CYNIC (Odysseum, Aura, Ignis). Il résout définitivement la dichotomie entre la présence physique de l'Opérateur (en France) et la souveraineté décentralisée du réseau, en s'appuyant sur les standards éprouvés de MetaDAO et Colosseum.

## 1. Le Bouclier Légal : L'Évitement de la "Direction de Fait"
Le risque majeur pour un fondateur résidant en France est la qualification de "Direction de Fait" par l'administration fiscale et l'AMF.
La solution est la **délégation absolue du pouvoir décisionnel au Marché**.
*   **Futard.io (Le Launchpad) :** Le projet n'est pas lancé par une décision unilatérale du fondateur. Il est soumis sous forme de *Proposal* (Proposition) au marché prédictif de Futard.io.
*   **La Souveraineté de l'Algorithme :** Lors de l'ICO, la propriété intellectuelle (IP) et les fonds levés sont transférés sous le contrôle exclusif de la trésorerie gouvernée par la Futarchie (MetaDAO). Le smart contract est le seul directeur de l'entité. L'opérateur physique est ainsi protégé de toute responsabilité managériale directe.

## 2. Le Framework STAMP (Levée de Fonds Privée)
Pour financer la phase initiale et capter des partenaires institutionnels (ex: Bloomup) sans compromettre la structure, le projet utilise le contrat **STAMP** (Simple Token Agreement, Market Protected).
*   **Zéro Equity (Single Economic Unit) :** Le STAMP remplace les SAFEs. Les investisseurs n'achètent aucune action (equity) dans l'entreprise française de développement (TALARIA SAS). L'unique actif acheté est l'allocation de Tokens futurs.
*   **Lock-Up :** Les tokens des investisseurs et de l'équipe (10% à 40%) sont soumis à un *vesting* linéaire de 24 mois après l'ICO, garantissant l'alignement long-terme.
*   **Non-Custodial & Non-PSAN :** Les fonds sont envoyés en USDC sur un smart contract d'escrow. TALARIA SAS ne fait pas de garde (Custody) de fonds tiers, neutralisant le risque de qualification PSAN par l'AMF.

## 3. L'Entité Offshore Automatisée (Cayman SPC)
Il n'est pas nécessaire de passer par des cabinets d'avocats lents et coûteux pour créer la structure légale émettrice du token.
*   **Création Native :** L'interface de MetaDAO permet l'instanciation directe d'une **Cayman SPC** (Segregated Portfolio Company). Cette coquille juridique offshore est structurellement liée au smart contract du projet.
*   **Le Contrat B2B :** Une fois la Cayman SPC créée et l'ICO réussie, TALARIA SAS (l'éditeur logiciel en France) se contente de facturer ses prestations de développement et de maintenance à la DAO, générant un revenu légalement rapatriable et imposable à l'IS en France.

## 4. L'Épée de Damoclès : L'Escrow et la Liquidation
Le lancement via Futard.io impose une exigence d'excellence à l'Opérateur (TALARIA SAS).
*   **Le Trésor Contrôlé par le Marché :** L'argent levé lors de l'ICO n'est pas versé librement à l'Opérateur. Il reste sous la supervision du marché prédictif.
*   **La Menace de Liquidation :** Si l'Opérateur échoue à livrer le code, cesse de communiquer ses KPIs, ou viole le contrat moral, les détenteurs de tokens (via le marché) peuvent voter le remboursement pur et simple du trésor (Capital Return). C'est le mécanisme de protection ultime ("Market Protected") qui remplace la confiance humaine par la théorie des jeux.
