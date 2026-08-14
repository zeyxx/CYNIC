# CYNIC : La Doctrine du Catalyseur (*The Catalyst*)

**Status :** Document doctrinal & cosmologique (Non-runtime authority — voir `docs/README.md` et `contracts/foundation.json`).

---

## I. Définition Doctrinale

Dans l'architecture et la cosmologie de CYNIC, **« The Catalyst » (le Catalyseur)** n'est pas une simple ligne de code, mais **la force qui précipite le passage de l'état potentiel à l'état actif**.

Le substrat Rust (`cynic-kernel`, `cynic-host`, `cynic-protocol`) constitue une structure déterministe, formelle et froide. Sans catalyseur, cette structure demeure un ensemble inerte de contrats d'admission, de baux de coordination et de registres statiques.

Le catalyseur s'articule sur **quatre dimensions fondamentales** :

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                              THE CATALYST                                  │
├───────────────────┬────────────────────────────────────────────────────────┤
│ Économique        │ $ASDF (drain d'attention & amorçage de liquidité)      │
│ Thermodynamique   │ IGNIS (combustion & création de rareté déflationniste) │
│ Humain            │ L'Opérateur / Prométhée (l'injection d'intention)      │
│ Runtime           │ L'Actuator (la transition d'état Desired -> Applied)   │
└───────────────────┴────────────────────────────────────────────────────────┘
```

---

## II. Les 4 Dimensions du Catalyseur

### 1. Le Catalyseur Mémétique & Économique : `$ASDF` (L'Élément Eau)
Dans la cosmologie des 5 éléments (`docs/v2-architecture-canon/CYNIC_COSMOLOGY_AND_IGNIS.md`) :
* **Fonction** : L'océan chaotique et viral. Le memecoin `$ASDF` draine l'attention de la masse et agit comme la force primitive qui attire les flux de capitaux et les utilisateurs vers le réseau.
* **Effet de catalyse** : Il transforme le bruit externe du marché en énergie d'entrée pour les coffres de liquidité souverains sans compromettre la gouvernance du protocole.

### 2. Le Catalyseur Thermodynamique : `IGNIS` (L'Élément Feu)
* **Fonction** : Le moteur autonome de rachat et destruction (*Buyback & Burn*).
* **Effet de catalyse** : `IGNIS` capte la rente générée par l'activité économique réelle (Odysseum, Aura, licences logicielles Talaria) et détruit irréversiblement des tokens via le *Quantum Treasury Bot*. Il catalyse l'antifragilité et la déflation mathématique de la monnaie de réserve.

### 3. Le Catalyseur Anthropologique : L'Archétype « Prométhée »
Dans les registres d'âmes E-Score (`CYNIC_ESCORE_REGISTRIES.md`) :
* **Fonction** : L'initiateur humain (Dominance *Anaideia* + *Philanthropia*).
* **Effet de catalyse** : L'Opérateur qui prend le risque initial sans filet, apporte le capital d'amorçage, installe les premiers nœuds et injecte l'intention sans laquelle aucun calcul n'a de sens.

### 4. Le Catalyseur d'Exécution : L'Actuator / Hybrid Corrector
Sur le plan du runtime Rust (`docs/CYNIC_AGENTIC_RUNTIME_ROLES.md` et `crates/cynic-host/src/agent_matrix.rs`) :
* **Fonction** : L'agent de mutation bornée.
* **Effet de catalyse** : Alors que l'`Observer` et le `Planner` restent confinés dans l'espace des descriptions pures (`DesiredState` / `ObservedState`), l'**Actuator** précipite la décision dans le monde physique en exécutant la commande de mutation (`--apply` / `--yes`) et en générant la preuve de read-back post-action.
