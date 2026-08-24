# TALARIA Legal Genesis Plan

Ce document definit ce qui doit etre vrai a la sortie de stage du **22 juillet
2026**, au checkpoint du **23 juillet 2026**, puis pendant la zone tampon
J+3/J+4 avant signature, communication publique forte ou lancement token.

Il ne remplace pas les statuts, l'avis d'un avocat, ni le travail d'un expert-comptable. Il sert de cahier de contraintes pour eviter la dispersion : le genesis doit etre clair, defensible, auditable et limite aux besoins reels.

Documents relies :

- `docs/foundation/TALARIA_CYNIC_NAMING.md` pour la separation entre TALARIA SAS, marque publique, framework CYNIC et `$CYNIC`.
- `docs/CYNIC_ECOSYSTEM_ARCHITECTURE.md` pour l'architecture ecosysteme.
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md` pour l'ownership, les wallets, les reserves et le token/equity firewall.
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md` pour la policy publique separant `$CYNIC` des actions, dividendes et droits legaux.
- `docs/foundation/CYNIC_E_SCORE.md` pour le modele de reputation, citoyennete reseau, merite operateur et contribution ecosysteme.
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md` pour la hierarchie des preuves legales, protocolaires, operationnelles, marche et Kernel.
- `docs/foundation/CYNIC_PROTOCOL_REVENUE_WATERFALL.md` pour le routage des revenus protocolaires, reserves, rewards, E-Score et surplus.

---

## 1. Principe Genesis

Le 23 juillet 2026 n'est pas une date de lancement token. C'est un checkpoint
de readiness apres la fin de stage du 22 juillet 2026.

Au 23 juillet 2026, TALARIA SAS ne doit pas prouver que CYNIC a deja tout
deploye. Elle doit prouver qu'elle sait qui possede quoi, qui decide quoi, qui
encaisse quoi, qui paie quoi, ce qui reste ouvert, et ce qui ne sera jamais
promis au public.

Une zone tampon de 3 a 4 jours apres le 23 juillet est explicitement normale
pour formaliser les derniers details :

- role exact de Ragnar ;
- statut holder / associe / contributeur de Ragnar ;
- niveau de reconnaissance, part ou statut d'Erin ;
- cap table finale ;
- whitepaper technique ;
- paper legal / MiCA route memo ;
- coherence entre communication, token paper et statuts.

Naming canonique :

```text
TALARIA SAS = legal genesis entity.
CYNIC = framework/protocol/kernel.
```

Doctrine :

```text
France first.
Usage before expansion.
Substance before entity.
Survival before burn.
Token is not equity.
No jurisdiction before necessity.
```

La France est le genesis legal parce que l'IP, la gouvernance initiale, les fondateurs et l'administration de depart y sont ancres. Les autres juridictions ne sont pas des prerequis du checkpoint du 23 juillet.

---

## 2. Livrables Minimum Du Checkpoint 23 Juillet 2026

Le checkpoint du 23 juillet est un point de consolidation interne. Il peut
autoriser la finalisation des statuts et des papers, mais il ne doit pas etre
presente comme une date de lancement public de `$CYNIC`.

### 2.1 Entite

A preparer :

- denomination sociale : TALARIA SAS ;
- siege social ;
- objet social ;
- duree ;
- capital social initial ;
- exercice social ;
- president ;
- associes initiaux ;
- modalites de decision ;
- pouvoirs du president ;
- clauses de cession d'actions ;
- clause de confidentialite raisonnable ;
- clause IP / contribution des fondateurs ;
- pacte d'associes si necessaire.

Objet social attendu :

```text
Edition de logiciels, developpement open source, intelligence artificielle,
infrastructure numerique, services informatiques, recherche et developpement,
detention de participations, exploitation et licence de propriete intellectuelle,
services aux entreprises, institutions et protocoles numeriques.
```

L'objet doit rester assez large pour couvrir IA, Solana, infrastructure, B2B/B2G, IP et holding, sans promettre une activite financiere reglementee non encadree.

### 2.2 Cap Table Genesis

La cap table genesis reste compacte.

Reference actuelle :

| Partie | Role | Pourcentage indicatif |
|--------|------|-----------------------|
| Jean | Architecte principal | 56.0% |
| Pere | Pilier identitaire | 22.0% |
| Ragnar | Bootstrapper | 13.6% |
| Erin | Reconnaissance fondatrice | 8.4% |

Regles :

- pas de cinquieme personne symbolique dans la cap table ;
- le cinquieme pole est le Kernel, pas un associe social ;
- les angels, LPs, compute providers, RPC providers et partenaires rejoignent le reseau par contribution mesuree, pas par pouvoir cache ;
- toute future entree au capital doit etre separee des droits protocolaires du token.

### 2.3 IP Et Contributions

Doit etre clair au checkpoint du 23 juillet, ou explicitement inscrit comme
decision de la zone tampon :

- quelle IP est apportee ou licenciee a TALARIA SAS ;
- quels repos, marques, noms, domaines, documents, architectures et standards sont couverts ;
- qui conserve quels droits moraux ou droits preexistants ;
- comment les futures contributions sont acceptees ;
- comment les branches peuvent utiliser l'IP CYNIC ;
- comment une entite future paie licence, royalty ou management fee a TALARIA SAS.

Minimum viable rule :

```text
TALARIA SAS controle la marque, les standards, les policies, la gouvernance et les droits d'exploitation de l'IP CYNIC.
```

### 2.4 Wallets Genesis

Les wallets du checkpoint genesis ne doivent pas etre nombreux. Ils doivent etre lisibles.

Wallets minimum :

| Wallet | Owner | Fonction |
|--------|-------|----------|
| SAS Treasury | TALARIA SAS | Revenus corporate, taxes, frais legaux, compta |
| Protocol Treasury | Smart contract / multisig | Flux on-chain non encore affectes |
| Tax Reserve | TALARIA SAS | Reserve stablecoin/fiat pour impots et charges |
| Burn Wallet | Smart contract | Burns publics et verifiables |
| Operator Payments | TALARIA SAS ou branche | Paiements de services verifies |

Regles :

- aucun wallet d'entite ne doit etre un wallet personnel ;
- les wallets critiques doivent etre multisig ou avoir une procedure documentee ;
- chaque wallet a un owner, une fonction, un signataire responsable et une politique de mouvement ;
- les documents prives peuvent rester off-chain, mais les hashes/snapshots peuvent etre publies.

### 2.5 Politique Fiscale Minimum

Au checkpoint du 23 juillet, il faut une politique simple, pas une optimisation internationale.

Doit exister :

- categorie des revenus : fiat, stablecoins, tokens, royalties, services, grants ;
- regle de conversion comptable pour les revenus crypto ;
- reserve fiscale minimale ;
- procedure d'off-ramp si necessaire ;
- distinction entre tresorerie SAS, tresorerie protocole et wallets operateurs ;
- expert-comptable identifie ou shortlist ;
- liste des questions fiscales ouvertes.

Regle fondatrice :

```text
No burn before tax reserve.
No reward before direct costs.
No distribution before accounting clarity.
```

### 2.6 Token / Equity Firewall

Le firewall doit etre pret avant toute communication publique sur `$CYNIC`.

La version autonome et publiable est maintenue dans `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`.

Interdits :

- presenter `$CYNIC` comme une action ;
- promettre des dividendes ;
- promettre un rendement ;
- dire `1 token = 1 action` ;
- lier automatiquement le token aux profits de TALARIA SAS ;
- donner une allocation cachee a des insiders ;
- vendre un acces privilegie avant publication des regles.

Autorises si documentes :

- acces au moteur ;
- paiement de compute ;
- staking operateur ;
- slashing ;
- E-Score ;
- rewards pour contribution mesuree ;
- buyback/burn apres couts, taxes et reserves ;
- gouvernance protocolaire non confondue avec gouvernance legale.

### 2.7 Public Transparency Policy

Doctrine :

```text
Public by default for power.
Private by default for ordinary execution.
Auditable always.
```

Doivent etre publics :

- dirigeants ;
- associes significatifs ;
- entites liees ;
- wallets officiels ;
- regles de treasury ;
- regles token/equity firewall ;
- conflits d'interets materiels ;
- operateurs ayant un poids protocolaire important ;
- changements de gouvernance majeurs.

Peuvent rester non publics :

- avocats ;
- experts-comptables ;
- freelances sans pouvoir de gouvernance ;
- clients sous NDA ;
- fournisseurs de securite ;
- petits prestataires ;
- informations qui exposent un risque physique, cyber ou commercial.

Mais tout doit rester auditable par la SAS, et les exceptions doivent avoir une justification.

---

## 3. Zone Tampon Post-Stage

La periode J+3/J+4 apres le checkpoint du 23 juillet sert a transformer les
decisions encore humaines en documents signables et publiables.

### 3.1 Decisions Humaines A Verrouiller

Avant signature ou communication publique forte :

1.  Ragnar est-il associe legal, holder protocolaire, contributeur reconnu,
    operateur, ou combinaison explicite de ces roles ?
2.  Si Ragnar est holder, quelle allocation, quel vesting, quelle justification
    publique et quel lien avec une contribution mesurable ?
3.  Erin recoit-elle une part legale, une reconnaissance fondatrice, une
    allocation token future, une place narrative, ou aucun droit economique ?
4.  Si Erin recoit une part ou allocation, quelle base : contribution passee,
    reconnaissance symbolique, service futur, pacte familial, ou autre
    justification documentee ?
5.  Les pourcentages de cap table restent-ils ceux du draft ou doivent-ils etre
    recalcules avant statuts ?
6.  Quelle partie de l'IP est apportee a TALARIA SAS et quelle partie reste
    licenciee ?
7.  Quels documents publics sortent au genesis, et lesquels restent internes ?

### 3.2 Whitepaper Technique

Le whitepaper technique doit expliquer CYNIC sans promettre de rendement.

Minimum :

- probleme traite ;
- architecture CYNIC Framework / Kernel / Dogs / agents / organs ;
- role du E-Score ;
- preuve d'historique et sources de verite ;
- flux de donnees ;
- modele operator / compute ;
- securite ;
- limites connues ;
- roadmap technique sans promesse commerciale garantie ;
- relation entre CYNIC Framework et CYNIC Protocol ;
- ce que `$CYNIC` active techniquement, si et seulement si les mechanics sont
  finalisees.

### 3.3 Paper Legal / MiCA Route Memo

Le paper legal n'est pas du marketing. Il doit qualifier le chemin possible.

Minimum :

- entite offeror ou personne demandant admission a la negociation, si applicable ;
- qualification MiCA preliminaire ;
- statut non-equity de `$CYNIC` ;
- separation TALARIA SAS / CYNIC Protocol ;
- revue des communications publiques ;
- traitement des wallets et allocations ;
- risques AML/CFT ;
- fiscalite et comptabilite crypto ;
- si AMF notification / white paper MiCA est requis ;
- ce qui reste interdit tant que counsel n'a pas valide.

### 3.4 Communication Pendant La Zone Tampon

Autorise :

```text
TALARIA SAS / CYNIC are finalising legal and technical genesis papers after the
22 July stage end. $CYNIC is a planned protocol asset, not equity, not launched.
```

Interdit :

```text
$CYNIC launches on 23 July.
Buy before the whitepaper.
Perp market soon.
Buyback confirmed.
Ragnar/Erin allocations confirmed before final docs.
```

---

## 4. Ce Qui N'Est Pas Necessaire Au Checkpoint 23 Juillet

Ces sujets sont importants, mais ne doivent pas bloquer le genesis.

### 4.1 Holding Northern Ireland / UK

Pas necessaire au genesis.

Trigger K15 :

```text
Besoin reel de cap table internationale, investisseurs common law,
contrats groupe ou structuration corporate hors France.
```

### 4.2 InfraCo Texas

Pas necessaire sans compute physique ou revenus infra reels.

Trigger K15 :

```text
Serveurs, GPU, datacenter, contrats energie, clients compute,
ou revenus infra necessitant une entite operationnelle US.
```

### 4.3 Suisse

Pas necessaire au genesis.

Trigger K15 :

```text
Besoin de banking crypto, opinion token, SRO AML, gouvernance protocolaire neutre,
association, fondation ou entite crypto operationnelle.
```

### 4.4 Asie

Pas necessaire au genesis.

Trigger K15 :

```text
Utilisateurs, clients, operateurs, distribution ou contrainte reglementaire locale.
```

### 4.5 ZK-KYC / Quantum-Proof KYC / PoH Avancee

Objectif long terme : remplacer le KYC intrusif par une preuve minimale, souveraine et privacy-preserving.

Mais au checkpoint du 23 juillet :

- ne pas pretendre remplacer legalement le KYC reglementaire existant ;
- ne pas promettre une solution quantum-proof tant qu'elle n'est pas implementee, auditee et comprise ;
- utiliser PoH comme primitive produit/reputation, pas comme passeport legal universel ;
- documenter la direction comme recherche CYNIC, pas comme obligation deja satisfaite.

Doctrine :

```text
KYC legal today where required.
ZK/PoH as progressive replacement target.
No compliance theater.
No privacy theater.
```

---

## 5. Decisions A Prendre Avant Signature

### Obligatoires

1.  Qui est president de TALARIA SAS ?
2.  Quelle adresse de siege social ?
3.  Quel capital social initial ?
4.  Les quatre parties fondatrices sont-elles toutes associees a la signature ?
5.  Les pourcentages de cap table sont-ils fixes ou encore indicatifs ?
6.  Quelle IP est apportee/licenciee a la SAS au genesis ?
7.  Qui signe les wallets genesis ?
8.  Quel expert-comptable ou cabinet est responsable du premier exercice ?
9.  Quelle reserve minimale est bloquee avant tout burn/buyback ?
10. Ragnar est-il associe, holder, operateur, contributeur, ou autre role public ?
11. Quel est le niveau exact de reconnaissance ou de participation d'Erin ?
12. Le whitepaper technique est-il coherent avec les claims publics ?
13. Le paper legal / MiCA route memo est-il pret pour counsel review ?
14. Quels documents publics sont publies le jour du genesis ?

### A Ne Pas Decider Trop Tot

1.  Creation d'une holding UK/NI.
2.  Creation d'une InfraCo Texas.
3.  Creation d'une entite suisse.
4.  Creation d'une entite asiatique.
5.  Vente de tokens.
6.  Instruments de rendement.
7.  Droits de gouvernance tokenises.
8.  Migration d'IP hors France.

---

## 6. Pack Public Genesis

Le jour du genesis, CYNIC peut publier un pack simple :

- CYNIC Legal Genesis Note ;
- CYNIC Technical Whitepaper ;
- CYNIC Legal Paper / MiCA Route Memo ;
- CYNIC Protocol Token Paper ;
- Token / Equity Firewall ;
- Wallet Ownership Registry v0 ;
- Protocol Revenue Waterfall v0 ;
- Fair Launch Covenant ;
- Federated Operator Charter v0 ;
- K15 Jurisdiction Expansion Doctrine ;
- liste des questions legales ouvertes.

Ces documents ne doivent pas tout promettre. Ils doivent montrer que CYNIC sait distinguer ce qui existe, ce qui est prevu, ce qui est interdit et ce qui dependra de l'usage reel.

---

## 7. K15 Jurisdiction Expansion Doctrine

Formule :

```text
Usage -> friction -> proof -> entity.
```

Une nouvelle juridiction n'est justifiee que si elle resout une friction prouvee :

| Juridiction | Fonction possible | Trigger |
|-------------|-------------------|---------|
| France | Genesis, IP, gouvernance, UE | Toujours, au checkpoint du 23 juillet 2026 |
| UK / Northern Ireland | Holding, cap table, common law | Investisseurs ou structuration groupe |
| Texas / USA | InfraCo, compute, datacenter | Actifs physiques et revenus infra |
| Suisse | Crypto clarity, banking, governance protocol | Opinion token, SRO, fondation/association |
| Asie | Distribution et operations locales | Usage local prouve |

Regle :

```text
CYNIC ne choisit pas une juridiction pour fuir une contrainte.
CYNIC choisit une juridiction parce qu'elle est le meilleur environnement pour une fonction precise du reseau.
```

---

## 8. Definition Of Done

Le Legal Genesis Plan est pret quand :

- les statuts peuvent etre rediges sans ambiguite majeure ;
- la cap table est comprise par tous les fondateurs ;
- les roles Ragnar et Erin sont explicites, documentes et non contradictoires ;
- l'IP genesis est identifiee ;
- les wallets genesis sont nommes ;
- les reserves fiscales et couts directs passent avant burn/rewards ;
- le token/equity firewall est publiable ;
- le whitepaper technique est suffisamment clair pour ne pas dependre du narratif token ;
- le paper legal / MiCA route memo est pret pour relecture counsel ;
- les relations de pouvoir sont publiques ;
- les relations ordinaires sont auditables ;
- les expansions UK/Texas/Suisse/Asie sont explicitement conditionnelles ;
- aucune communication publique ne promet rendement, dividendes, equity ou asymetrie cachee.
