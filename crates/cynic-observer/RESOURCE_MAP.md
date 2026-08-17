# CYNIC Observer — carte initiale des ressources

> Inventaire de conception, observé au 3 août 2026.
>
> Les dépôts publics et artefacts locaux prouvent l'existence de ressources
> publiées, pas leur déploiement, leur usage réel, leur propriété juridique ou
> leur maintenance future. Les statuts doivent être réévalués à chaque échelle
> temporelle.

## 1. Règle temporelle

Nous partons de l'hypothèse de travail suivante : à chaque horizon, une grande
partie des implémentations sera remplacée, améliorée ou abandonnée. La carte
doit donc enregistrer les primitives et les relations, pas sacraliser les
prototypes actuels.

```text
snapshot observé
  → primitive extraite
  → usage prouvé ou non prouvé
  → remplacement possible
  → nouvelle observation
```

Un dépôt est une ressource potentielle. Il devient une capacité runtime
seulement avec une intégration, un propriétaire, une preuve d'exécution et une
politique de maintenance.

## 2. Surfaces actuelles à observer

Selon l'opérateur, plusieurs prototypes publics sont morts ou ne sont plus des
priorités actives. Ils restent utiles comme archives de provenance, mais ne
doivent pas être présentés comme des dépendances actuelles.

Les surfaces publiques à traiter comme candidates actuelles sont :

| Surface | Observation initiale | Statut à formaliser |
|---|---|---|
| [this-is-fine-shop.com](https://this-is-fine-shop.com/) | storefront Shopify accessible | public surface observed; commerce/live capability unknown |
| [alonisthe.dev/robinhood](https://alonisthe.dev/robinhood) | page publique accessible | public surface observed; function/ownership attestation needed |
| [alonisthe.dev/burns](https://alonisthe.dev/burns) | page publique accessible | public surface observed; on-chain linkage needed |
| [holdex.live](https://holdex.live/) | application web accessible | public surface observed; runtime/data freshness needed |
| [asdf-forge/sdk](https://github.com/asdf-forge/sdk) | dépôt public non archivé, tooling Solana partagé | active candidate; release/use proof needed |

L'existence HTTP d'une page ne prouve ni son déploiement complet, ni ses
transactions, ni la continuité de son opérateur. Chaque surface doit être
observée avec un timestamp, un statut HTTP, un digest de contenu, une capacité
déclarée et une preuve d'usage.

## 3. Ressources économiques

### Actifs et flux

- `$ASDFASDFA` : actif baseline, laboratoire de preuve et récit d'origine ;
- flux Pump.fun/PumpSwap : création, trading, liquidité, frais et migrations ;
- buybacks et burns : flux à documenter par preuves on-chain, sans confondre
  intention annoncée et événement exécuté ;
- creator fees et mécanismes de redistribution ;
- prediction markets et mécanismes de vote ;
- coûts RPC, stockage, indexation, calcul, appareils et temps humain ;
- potentiel futur de frais, grants, licences ou services — actuellement à
  distinguer des revenus réellement observés.

### Prototypes économiques publics

| Ressource | Fonction annoncée | Relation CYNIC/ODYSSEUM | Statut initial |
|---|---|---|---|
| [ASDForecast](https://github.com/sollama58/ASDForecast) | prediction market Solana lié à ASDFASDFA | expérience futarchique | dépôt public observé, live unknown |
| [ASDFBurnTracker](https://github.com/sollama58/ASDFBurnTracker) | suivi buyback/burn | preuve économique et publication | dépôt public observé, source on-chain à vérifier |
| [Inferno](https://github.com/sollama58/Inferno) | Pump.fun fee-to-XMoney | flux de frais / pont économique | dépôt public observé, déploiement unknown |
| [TokenVotingUtil](https://github.com/sollama58/TokenVotingUtil) | utilitaire de vote par token | gouvernance / proposal | dépôt public observé, autorité unknown |
| ODYSSEUM | creator capital market et application économique | consommateur de primitives CYNIC | code local et prototypes observés |

Les prix, la valeur de marché, les revenus et la viabilité ne doivent jamais
être déduits du nombre de dépôts, d'étoiles ou du récit.

## 4. Ressources techniques

### Portefeuille public historique / compte de référence

Le compte public `sollama58` expose des prototypes utiles pour comprendre la
généalogie du système. Selon l'opérateur, plusieurs sont désormais morts ; ils
sont donc classés `historical_prototype` jusqu'à preuve contraire.

| Projet | Fonction annoncée | Primitive réutilisable |
|---|---|---|
| [ASDev](https://github.com/sollama58/ASDev) | launchpad Pump.fun avec airdrop | lancement, distribution, intégration Pump |
| [HolDex](https://github.com/sollama58/HolDex) | alternative open source à DexScreener | indexation, marché, projection publique |
| [CultScreener](https://github.com/sollama58/CultScreener) | analyse de cultes Solana | signaux culturels, holders, communautés |
| [PumpPet](https://github.com/sollama58/PumpPet) | compagnon pour traders Pump.fun | gamification, continuité comportementale |
| [asdf_grinder](https://github.com/sollama58/asdf_grinder) | génération/recherche de clés ASDF | outil spécialisé ; surface de sécurité à auditer |

Le portefeuille public ne doit pas être confondu avec le portefeuille actuel.
Les surfaces vivantes ci-dessus ont priorité pour l'observation et l'ingestion.

### Infrastructure déjà disponible

- `cynic-observer` : WAL, parser Pump.fun, watcher, fédération et telemetry ;
- `cynic-intel` : scores, risque, E-Score et preuves Merkle ;
- `cynic-protocol` : enveloppes d'évidence, lineage et relations ;
- ODYSSEUM warehouse : données Pump.fun brutes, compactées et dérivées ;
- Helius et RPC public : deux chemins d'observation à comparer ;
- ordinateur et téléphone de l'opérateur : futurs capteurs opt-in ;
- CYNIC local : premier nœud de calibration, actuellement partiellement
  qualifié et avec couverture runtime incomplète.

Chaque dépendance doit être inscrite avec : fournisseur, coût, secret requis,
alternative, fraîcheur, rate limit, conditions d'usage et preuve de continuité.

## 5. Ressources humaines

### Autorité et travail

- humain souverain : objectifs, consentement, curation, arbitrage et gouvernance
  finale ;
- CTO / mainteneur déclaré : conception et livraison des prototypes ; le rôle
  humain ne doit pas être déduit automatiquement d'un compte GitHub ;
- opérateur CYNIC : premier annotateur et utilisateur de référence ;
- agents et LLM : extraction, comparaison, proposition d'hypothèses et
  recherche de contradictions, sans autorité finale ;
- futurs contributeurs : code, données, vérification, modération et opérations ;
- communautés et utilisateurs : producteurs de signaux culturels et
  participants à la gouvernance, jamais matière première silencieuse.

### Ressources humaines à mesurer

```text
temps de curation
temps de maintenance
attention décisionnelle
compétence technique
confiance sociale
capacité de contestation
```

La rareté principale du prototype n'est probablement pas le stockage, mais le
temps humain capable de distinguer un fait, un motif, une hypothèse et un récit.

## 6. Ressources culturelles

- lore ASDFASDFA et statut de token baseline ;
- easter eggs et généalogie Pump.fun ;
- langage de communauté : cult, OG, vamp, pill, burn, sacrifice ;
- mythologie des creator capital markets ;
- futarchie comme mécanisme de décision et non simple branding ;
- Solana comme infrastructure de règlement et de coordination ;
- one-person enterprise at planetary scale ;
- privacy by default et human-in-the-loop ;
- ethos du builder, du grinder et du contributeur ;
- désaccord comme signal de qualité et contestation comme droit.

La couche culturelle doit conserver les sources, dates, auteurs déclarés,
reprises, transformations et contradictions. Une phrase répétée par une
communauté prouve sa circulation ; elle ne prouve pas son contenu factuel.

## 7. Ressources à ne pas confondre

```text
prototype public       ≠ produit live
compte GitHub          ≠ identité juridique
wallet lié             ≠ personne prouvée
intention culturelle   ≠ résultat économique
score                  ≠ vérité
volume de données      ≠ couverture correcte
preuve locale          ≠ attestation publique
```

## 8. Fiche d'ingestion obligatoire

Avant d'intégrer une ressource, créer une fiche :

```text
resource_id
kind: economic | human | technical | cultural
source_ref
owner_or_custodian
declared_role
observed_capability
runtime_status
dependencies
cost_profile
secret_requirements
license_or_terms
privacy_scope
proof_refs
unknowns
replacement_candidates
last_observed_at
```

Cette fiche est la réponse au biais top-down : nous pouvons ingérer largement,
mais chaque ressource garde son statut, son coût, sa provenance et ses limites.

## 9. Première convergence

Le premier dossier à remplir est `ASDFASDFA / node-0` :

1. lister les ressources économiques réellement observables ;
2. relier les dépôts publics aux primitives, sans les déclarer live ;
3. relier le warehouse ODYSSEUM aux preuves `cynic-protocol` ;
4. annoter humainement les relations importantes ;
5. mesurer le coût réel d'une analyse complète ;
6. enregistrer ce qui a été remplacé, corrigé ou abandonné.

La carte sera utile si elle évite de répéter la même conversation : le code
pourra changer, mais les ressources, propriétaires, contrats, preuves,
contraintes et inconnues resteront rejouables.
