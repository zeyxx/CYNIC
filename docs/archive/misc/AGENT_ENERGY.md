# CYNIC Agent Energy — Décision de capacité native

Statut : proposé. Sans ratification, aucune implémentation.

## 1. Problème

Les budgets d'agents sont déclarés partout (`MandateAccount`,
`AgentBudget`, enveloppes de délégation) et mesurés nulle part :
`cpu_ms` reste None, les tokens consommés sont inconnus, les appels
d'outils ne sont comptés par personne. Une accountability sans mesure
de consommation est une autorisation sans facture. Le compteur doit
donc devenir une capacité native : observable, attribuable, journalisée.

## 2. Ce qui est mesurable nativement (et ce qui ne l'est pas)

Mesurable sans fabrication :

- Appels d'outils servis : `cynic-mcp` connaît outil, session, octets
  entrants/sortants, durée murale par appel. Comptabilité exacte.
- Exécutions argv : `ArgvRunner` observe déjà codes de sortie ; ajouter
  durée et octets relève du même port.
- Charges build : le runner connaît durée et actions consommées — déjà
  partiellement dans les enregistrements.

Non mesurable nativement, donc inconnu explicite :

- Tokens modèle des sessions externes (Hermes et autres) : jamais
  devinés, jamais extrapolés des appels. `None`, pas zéro.
- Tokens d'inférence servie via CYNIC : mesurables aux compteurs du
  serveur local, phase ultérieure, même schéma avec champs renseignés.

## 3. Architecture cible

- Types purs dans `cynic-protocol` : `cynic.agent_energy.v1`
  (`EnergyRecord` : ids session/charge/agent, appels par outil, octets,
  durées, tokens optionnels). Les champs non mesurés sont `Option`,
  jamais zéro silencieux.
- Journal en fichierJSONL sous racine d'état, un enregistrement par
  appel ou par session. Append-only, horodaté, attribué au mandat.
- Producteur phase 1 : `cynic-mcp` comptabilise chaque appel servi.
  Producteurs suivants : runner argv, build runner, inférence locale.
- Consommateur : rapprochement budgets contre consommation (dépassement
  = violation typée), et registre de cas de la meta-skill
  (énergie réelle par run au lieu d'estimations).

## 4. Utilité et non-objectifs

Utilité : fermer la boucle déclarer-mesurer pour chaque budget, fournir
aux runs de recalibration leur coût réel, et donner au scheduler futur
sa matière première (consommation passée par agent et par outil).

Non-objectifs : facturation, quotas durs, mesure des sessions externes,
optimisation de modèles. Aucune décision automatique sur ces données
en phase 1 — observation d'abord, politique après, avec sa propre
décision.

## 5. Preuves exigées

Types testés en protocole, un appel servi = un enregistrement journal
(test live existant étendu), zéro `None` inventé en zéro, suite
`cynic-mcp` verte. La phase 1 ne touche ni au scheduler ni à l'admission.
