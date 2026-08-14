# CYNIC Token / Equity Firewall

Ce document formalise la separation stricte entre `$CYNIC`, TALARIA SAS, les actions legales, les revenus corporate, les flux protocolaires et les droits de gouvernance.

Il ne constitue pas un avis juridique. Il sert de policy publique et interne pour eviter toute confusion entre un token d'acces/protocole et une action de societe.

Documents relies :

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`
- `docs/foundation/CYNIC_E_SCORE.md`
- `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`
- `docs/foundation/CYNIC_OWNERSHIP_CONSTITUTION.md`
- `docs/CYNIC_ECOSYSTEM_ARCHITECTURE.md`

---

## 1. Loi Centrale

```text
Token is not equity.
Equity is not protocol access.
Protocol rewards are not dividends.
Buyback is not yield.
Governance protocol is not legal control.
```

`$CYNIC` ne represente pas une action de TALARIA SAS. Il ne donne pas droit aux dividendes legaux, aux actifs de la societe, aux parts des filiales, au datacenter, a l'IP, ni a une creance sur TALARIA SAS.

`$CYNIC` est le token du framework/protocole CYNIC, pas le token de TALARIA SAS. Cette separation est intentionnelle : TALARIA SAS est l'entite legale, CYNIC Protocol est le reseau, `$CYNIC` est l'actif protocolaire et memetique.

La cap table legale est geree par les statuts, le pacte d'associes et le droit applicable. Le token est gere par ses smart contracts, ses policies publiques et ses usages protocolaires.

---

## 2. Ce Que `$CYNIC` Peut Etre

`$CYNIC` peut etre concu comme un actif d'usage, d'acces, de coordination et de reputation.

Usages autorisables si documentes :

| Usage | Fonction |
|-------|----------|
| Access | Acceder a certains moteurs, produits, APIs ou branches |
| Compute | Payer ou prioriser du compute, inference, RPC ou services reseau |
| Operator staking | Garantir un comportement d'operateur avec slashing possible |
| Reputation | Alimenter un E-Score ou une priorite reseau quand l'usage est verifie |
| E-Score | Signaler la reputation, la citoyennete reseau et la responsabilite gagnees par contribution verifiee |
| Governance protocol | Voter ou signaler sur des parametres protocolaires non-legaux |
| Rewards | Remunerer du travail, du risque, du compute, de la liquidite ou de la securite verifies |
| Burn | Reduire la supply selon des regles publiques |
| Buyback | Racheter des tokens uniquement apres couts, reserves et fiscalite |

Le token peut participer a l'economie protocolaire, mais ne doit jamais etre presente comme une propriete legale de l'entreprise.

---

## 3. Ce Que `$CYNIC` Ne Doit Jamais Etre

Interdits :

- action de TALARIA SAS ;
- part sociale ;
- titre financier presente comme tel ;
- droit legal aux dividendes ;
- droit automatique aux profits de TALARIA SAS ;
- droit automatique aux profits d'une filiale ;
- creance sur les actifs, serveurs, GPU, IP ou tresoreries du groupe ;
- promesse de rendement ;
- promesse de rachat ;
- promesse de prix plancher ;
- allocation cachee pour insiders ;
- acces privilegie non publie ;
- information de launch reservee a quelques proches ;
- instrument de contournement du droit des societes ;
- vote qui controle directement les organes legaux de TALARIA SAS.

Phrase courte :

```text
Holding `$CYNIC` does not make you a shareholder of TALARIA SAS.
```

---

## 4. Separation Des Droits

| Domaine | Source du droit | Exemple |
|---------|-----------------|---------|
| Actions TALARIA SAS | Statuts, pacte, droit francais | vote d'associe, cession d'actions |
| IP CYNIC | TALARIA SAS, contrats, licences | marque, code, standards |
| Flux corporate | Contrats, factures, comptabilite | B2B, B2G, royalties, management fees |
| Flux protocolaires | Smart contracts, treasuries, policies | fees on-chain, rewards, burn |
| Merite reseau | Kernel, observations, E-Score | uptime, compute, contributions |
| Gouvernance protocolaire | Contracts, votes, registries | parametres de branche ou operateur |

Une meme personne peut avoir plusieurs roles, mais les droits ne fusionnent pas.

Exemple :

```text
Un associe TALARIA SAS peut aussi hold `$CYNIC`.
Mais ses droits d'associe viennent de ses actions, pas de ses tokens.
Ses droits token viennent des smart contracts, pas des statuts.
```

---

## 5. Protocol Rewards Ne Sont Pas Des Dividendes

Un dividende legal est une distribution de benefice decidee par une societe selon le droit applicable.

Une reward protocolaire est un paiement pour une contribution verifiee :

- compute fourni ;
- inference servie ;
- uptime ;
- liquidite utile ;
- risque LP ;
- audit ;
- securite ;
- integration ;
- usage produit ;
- contribution technique ;
- signal de reputation.

Regle :

```text
No contribution, no protocol reward.
No legal share, no legal dividend.
```

Le protocole peut remunerer un service mesure. Il ne doit pas remunerer la simple detention passive comme si elle donnait droit aux profits de TALARIA SAS.

---

## 6. Buyback / Burn Policy

Un buyback ou un burn ne doit jamais etre presente comme un revenu garanti.

Ordre de priorite :

```text
branch revenue
  -> direct costs
  -> tax/accounting reserve
  -> operator/service payments
  -> security/audit reserve
  -> TALARIA SAS IP/governance fee if applicable
  -> protocol surplus
       -> buyback
       -> burn
       -> grants
       -> E-Score rewards
       -> liquidity incentives
```

Regles :

- pas de burn avant reserves de survie ;
- pas de buyback avant clarte fiscale ;
- pas de promesse de montant, frequence ou rendement ;
- pas de communication laissant croire a un dividende ;
- toute automation smart contract doit etre publiee et auditable ;
- toute discretion humaine doit etre encadree par une treasury policy.

Phrase autorisee :

```text
Some protocol surplus may be routed to buyback or burn after costs, taxes, reserves and security requirements.
```

Phrase interdite :

```text
Holders will receive protocol profits through buybacks.
```

---

## 7. Fair Launch Et Information Symmetry

Le firewall exige que le launch ne cree pas d'asymetrie cachee.

Interdits :

- presale cachee ;
- allocation angel cachee ;
- annonce privee du timing de launch ;
- insider wallets non declares ;
- market making opaque controle par insiders ;
- vesting off-chain invisible ;
- promesse verbale differente de la documentation publique.

Autorises si publics :

- allocation equipe avec vesting ;
- airdrop retroactif selon criteres publies ;
- burn-to-mint selon contrat public ;
- LP ou operator programs avec risques publics ;
- contribution angels par compute, RPC, hosting, audit ou liquidite productive ;
- lockups on-chain ;
- wallets identifies par fonction.

Doctrine :

```text
No hidden allocation.
No hidden timing.
No hidden power.
```

---

## 8. Investors / Angels / Operators

Un investisseur ou angel peut etre aligne avec CYNIC sans recevoir de privilege token cache.

Voies propres :

- investissement legal dans une entite, documente par contrats ;
- pret ou financement corporate, si legalement structure ;
- fourniture de compute ;
- fourniture de GPU ;
- fourniture de RPC ;
- fourniture de hosting ;
- fourniture de liquidite productive ;
- audit et securite ;
- integration commerciale ;
- distribution B2B/B2G.

Regle :

```text
Capital power stays in legal documents.
Protocol power is earned by public contribution.
```

Un angel ne gagne pas parce qu'il sait avant les autres. Il gagne si son capital, son infrastructure, son travail ou son risque rendent le reseau plus utile.

---

## 9. Communication Policy

### Phrases Autorisees

- `$CYNIC` is designed as a protocol utility and coordination token.
- `$CYNIC` is not equity in TALARIA SAS.
- `$CYNIC` is the CYNIC Protocol token, not the TALARIA SAS token.
- `$CYNIC` does not grant legal dividends.
- Protocol rewards require verified contribution.
- Buyback and burn, if any, happen only after costs, taxes, reserves and security needs.
- Legal governance and protocol governance are separate.
- TALARIA SAS may own IP and receive taxable corporate revenue.

### Phrases Interdites

- Buy `$CYNIC` to own part of TALARIA SAS.
- Buy `$CYNIC` to own TALARIA.
- `$CYNIC` holders receive company profits.
- `$CYNIC` is backed by the datacenter.
- Guaranteed yield.
- Guaranteed buyback.
- Guaranteed floor.
- Dividends to token holders.
- The token represents shares.
- Angels get early access before public rules.

### Phrases A Eviter Sans Relecture Juridique

- commodity ;
- security ;
- investment ;
- profit share ;
- passive income ;
- revenue share ;
- ownership ;
- equity-like ;
- backed by assets ;
- guaranteed return.

La qualification legale depend du design, de la distribution, de la communication, des droits accordes, des attentes creees et des juridictions concernees. Elle ne se decrete pas par une phrase marketing.

---

## 10. Smart Contract Design Guardrails

Les smart contracts doivent incarner le firewall.

Eviter :

- `claimDividends()` ;
- `shareholderReward()` ;
- `profitDistribution()` ;
- distribution automatique a tous les holders passifs ;
- poids legal simule par solde token ;
- contrats qui promettent un rendement fixe ;
- mapping token -> action SAS ;
- liens directs entre cap table et holder list publique.

Preferer :

- `claimOperatorReward()` ;
- `claimComputeReward()` ;
- `claimLiquidityReward()` ;
- `claimContributionReward()` ;
- `stakeForAccess()` ;
- `stakeForOperatorBond()` ;
- `slashOperator()` ;
- `recordContribution()` ;
- `routeProtocolSurplus()` ;
- `executeBurn()`.

Le nom des fonctions compte : il produit une lecture juridique et sociale du systeme.

---

## 11. KYC, ZK Et PoH

CYNIC vise a long terme un modele de preuve d'humanite et de conformite plus minimal, plus souverain et plus respectueux de la vie privee que le KYC intrusif traditionnel.

Mais le firewall impose une separation :

- PoH peut filtrer les sybils, reputations, airdrops ou acces produit ;
- ZK peut reduire l'exposition de donnees personnelles ;
- quantum-proof est une exigence de recherche, pas une promesse publique tant que ce n'est pas audite ;
- si une obligation KYC/AML legale s'applique, elle ne doit pas etre ignoree par marketing PoH.

Doctrine :

```text
Use PoH where it proves humanity.
Use ZK where it reduces disclosure.
Use legal KYC where law requires it.
Never pretend privacy tech cancels legal obligations by default.
```

---

## 12. Governance Boundary

`$CYNIC` peut gouverner des parametres de protocole, pas la societe elle-meme.

Peut relever de la gouvernance protocolaire :

- parametres de fees ;
- admission d'operateurs ;
- slashing rules ;
- grants ;
- registry weights ;
- E-Score formulas ;
- upgrade schedules ;
- liquidity incentives ;
- product-level policies.

Ne doit pas relever directement du token vote :

- nomination du president de TALARIA SAS ;
- cession d'actions ;
- modification des statuts ;
- distribution de dividendes legaux ;
- transfert d'IP ;
- vente du datacenter ;
- changement de controle legal ;
- decisions reservees aux organes sociaux.

La SAS peut ecouter le protocole. Elle ne doit pas pretendre que le token remplace le droit des societes.

---

## 13. Red Flags

Une decision doit etre bloquee et relue si elle cree un de ces signaux :

- holders passifs payes proportionnellement au solde ;
- promesse de rendement ;
- promesse de rachat ;
- allocation non publiee ;
- droit token sur actifs corporate ;
- confusion entre dividend et reward ;
- confusion entre action et token ;
- bridge automatique token -> equity ;
- marketing qui parle de profit attendu ;
- smart contract nomme comme une distribution de benefices ;
- absence de reserve fiscale avant burn ;
- insiders capables de sortir avant le public.

---

## 14. Definition Of Done

Le firewall est respecte quand :

- la documentation publique dit explicitement que `$CYNIC` n'est pas une action ;
- les smart contracts ne distribuent pas de dividendes masques ;
- les rewards sont liees a des contributions verifiees ;
- les buybacks/burns passent apres couts, taxes, reserves et securite ;
- les allocations equipe et vesting sont publics ;
- les operators et angels n'ont pas de privilege invisible ;
- la gouvernance protocolaire ne controle pas directement TALARIA SAS ;
- la communication evite les promesses de rendement ;
- les exceptions sont relues juridiquement avant publication.
