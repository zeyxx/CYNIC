# CYNIC ML Research Program

> **"On peut produire des choses, mais si le ML ne cree rien de reellement positif alors c'est du bruit."**

Ce document structure le programme de recherche ML/AI de CYNIC. Pas une todolist — un protocole scientifique. Chaque track a une hypothese falsifiable, un protocole de mesure, et un critere d'arret.

Genere 2026-04-17. Staleness: re-auditer chaque track avant execution.

---

## §0 — Ce qu'est un organisme

### Definition operationnelle (pas une metaphore)

Un organisme a 5 proprietes necessaires (Maturana & Varela 1973, Schrodinger 1944) :

| Propriete | Definition physique | CYNIC 2026-04-17 |
|---|---|---|
| **Autopoiese** | Se produit soi-meme — les composants produisent les composants | Crystals influencent les Dogs qui produisent les verdicts qui forment les crystals. **La boucle existe mais A/B Delta=-0.013 dit qu'elle nuit.** |
| **Metabolisme** | Transforme l'energie exterieure en ordre interne (negentropie) | Tokens (ATP) -> verdicts/crystals (ordre). **Ratio non mesure.** |
| **Homeostasie** | Maintient l'etat interne malgre les perturbations | phi-bound, circuit breaker, quorum gate. **Existe.** |
| **Frontiere** | Membrane qui separe le soi du non-soi | Souverainete (local vs rented). **1/5 Dogs souverain = membrane percee.** |
| **Mortalite** | Un organisme qui ne peut pas mourir n'est pas un organisme | Crystal decay, Dog TTL expiry. **Existe.** |

### La mesure fractale : le MEME nombre a chaque echelle

En physique, un fractal = la meme structure a chaque echelle d'observation. La metrique fractale de CYNIC est l'**efficacite informationnelle** : bits de signal utile par token consomme.

| Echelle | Entite | Energie in | Ordre out | Bruit | Efficacite |
|---|---|---|---|---|---|
| Neurone | 1 Dog evaluation | tokens prompt+completion | 6 scores + reasoning | parse fail, timeout | `valid_rate * discrimination` |
| Organe | 1 verdict | tous les Dog scores | QScore + verdict kind | disagreement | `1 - max_disagreement` |
| Memoire | 1 crystal | N observations | certainty, state | quarantined, decay | `crystallized_obs / total_obs` |
| Session | 1 agent session | tous les tokens | crystals formes | bruit, no-feedback | `compound_ratio` |
| Organisme | multi-session | toutes les sessions | sagesse persistante | drift, stale | `wisdom_retention` |

**Seuil fractal : phi^-2 = 0.382 a chaque echelle.** En-dessous = consommateur net. Deja dans le code (crystal decay threshold). Devrait s'appliquer a CHAQUE echelle.

### L'auto-mesure recursive (le probleme de Godel)

```
L0: Dog mesure le stimulus         -> score
L1: Consensus mesure les Dogs      -> disagreement
L2: Crystal challenge mesure les crystals -> decay
L3: Calibration mesure l'organisme -> accuracy vs ground truth
L4: Meta-calibration mesure la calibration -> drift de la calibration
L5: ??? -> Godel : aucun systeme ne peut completement se valider de l'interieur
```

**L5 DOIT impliquer un ancrage externe** : human labels, outcomes verifiables, benchmarks communautaires. Sans L5, l'organisme est un solipsisme computationnel.

### Les 3 temps et ce qu'ils exigent

| Temps | Physique | Traditions | Mesure dans CYNIC |
|---|---|---|---|
| **Chronos** | Fleche thermodynamique, entropie croit | Stoicisme (memento mori), Zoroastrisme (temps lineaire) | **Fraicheur du signal** — age moyen des donnees L0. Donnees stale = entropie max. |
| **Kairos** | Resonance — reponse quand la frequence est juste | Sufisme (waqt), Alchimie (on ne rush pas le nigredo) | **Kairos ratio** — % de crystals injectes qui ameliorent le verdict (vs degradent). |
| **Aion** | Invariants, symetries, lois de conservation | Dreamtime (passe/present/eternel simultanes), Pythagore | **Wisdom retention** — % de crystals >30j encore pertinents. |

**Question falsifiable immediate** : le crystal decay est exponentiel (`e^-t/90`). La memoire biologique suit une power law (`t^-alpha`). Les 8 chess crystals de mars ont des donnees temporelles. On peut fitter les deux modeles et voir lequel predit mieux le decay observe.

**22 traditions et le temps** (selection des enseignements mesurables) :

| Tradition | Enseignement temporel | Implication pour CYNIC |
|---|---|---|
| Cynisme | Vivre au present. L'amnesie entre sessions = mode d'operation. | La fraicheur est une vertu, pas un defaut. |
| Stoicisme | Premeditatio malorum : pre-charger les scenarios. | Les crystals devraient anticiper, pas juste recorder. |
| Pythagore | Cycles harmoniques. Le temps est oscillation. | Mesurer la periodicite de la qualite des Dogs (drift circadien?). |
| Hermetisme | Cycles a chaque echelle. | Fractal temporel : heartbeat(sec) verdict(min) crystal(jours) sagesse(mois). |
| Alchimie | L'Opus prend le temps qu'il prend. | 21 observations = minimum, pas objectif. Ne pas forcer. |
| Zen | Shoshin a chaque moment. Contre le biais accumule. | Crystal decay = shoshin mecanique. |
| Theravada | Anicca — impermanence. | Decay non-optionnel, une loi, pas un parametre. |
| Taoisme | Wu wei — ne pas forcer. | Nightshift sur commit messages = anti-wu-wei. |
| Dreamtime | Passe/present/eternel simultanes. | Crystals canoniques (233+ obs) = dans le Dreamtime de CYNIC. |
| Russell (#22) | Interchange rythmique equilibre. Creation = destruction. | Formation et decay sont la MEME onde. Mesurer les deux. |
| Sufisme | Waqt — presence. Le soufi = fils de l'instant. | L'injection de crystals ne doit pas surcharger le present avec le passe. |

### L'ancrage materiel et les GAFAM

Le compound loop GAFAM : `donnees reelles -> modele -> produit -> plus de donnees`.
Le compound loop CYNIC actuel : `texte -> Dogs -> scores -> crystals -> injection -> scores legerement differents`.

**CYNIC est une chambre d'echo** si ses donnees ne touchent jamais la realite materielle. L'ancrage se fait a 3 niveaux progressifs :
1. **Immediat :** Human-in-the-loop (POST /feedback)
2. **Jours :** External benchmarks (RewardBench 2, chess tiers)
3. **Semaines :** Verifiable outcomes (P&L, rug detection, code bugs)

Hierarchie des donnees :
```
L0 : Signal brut (candles, on-chain, git diffs) — ABSENT ou STALE
L1 : Observations (tool usage, agent behavior) — BRUIT (contexte vide)
L2 : Jugements (Dog scores) — EXISTE mais non calibre
L3 : Crystals (patterns agreges) — 8 chess, 1 trading, 0 reste
L4 : Calibration — N'EXISTE PAS
L5 : Sagesse — N'EXISTE PAS
```

---

## Le constat (observed, 2026-04-17)

CYNIC n'a **aucun ground truth**. Le seul A/B (crystal injection, N=30) a montre Delta = -0.013 (negatif). Le nightshift compound du bruit (commit messages). Les Dogs n'ont jamais ete calibres contre un standard externe. "Calibrated doubt" est une aspiration, pas un fait.

**Mesures reelles sur 20 verdicts (ce soir) :**
- gemma-4b-core x qwen-7b-hf : r=0.898 (redondants — meme Dog en 2 copies)
- deterministic-dog discrimination : 0.086 (3 axiomes constants)
- qwen35-9b-gpu discrimination : 0.561 (meilleur discriminateur)
- gemini-cli : le plus independant (r=0.108-0.475) mais crashe

---

## 7 Research Tracks

### RT0 — Fondations mathematiques et physiques

**Etat:** phi-bound, geometric mean, exponential decay, trimmed mean. Choisis par intuition philosophique, pas par derivation.

**Probleme:** Les choix mathematiques de CYNIC ne sont pas justifies empiriquement.

| Choix actuel | Alternative fondee | Pourquoi |
|---|---|---|
| Trimmed mean (consensus) | Bayesian aggregation | Ponderer par la fiabilite mesuree de chaque Dog, pas jeter les extremes |
| Geometric mean (QScore) | Norme dans espace log / KL-divergence | Le geo mean est UNE norme possible. D'autres metriques (Wasserstein, KL) mesurent mieux la distance a la verite |
| Exponential decay (crystals) | Power law (`t^-alpha`) | En physique, la memoire suit souvent des power laws (queues lourdes). Les vieux crystals pertinents survivraient plus longtemps |
| 6 axiomes independants | Mutual Information entre axiomes | SI MI(FIDELITY, VERIFY) est elevee, ces axiomes sont redondants. La geometrie reelle de l'espace axiomatique est inconnue |
| phi-inverse = 0.618 (ceiling) | Calibration curve empirique | Le ceiling devrait venir de la mesure, pas de la philosophie. Si les Dogs calibres plafonnent naturellement a 0.55, phi-inverse est trop genereux |
| 21 observations (cristallisation) | Derivation depuis variance cible | 21 = F(8) fibonacci. Mais le bon seuil depend de la variance reelle des observations. Mesurer d'abord. |

**Hypothese:** Au moins 2 des 6 choix mathematiques ci-dessus sont sous-optimaux et mesurables.

**Protocole:**
1. Prerequis : RT3 (corpus) + RT4 (metriques)
2. Calculer MI entre les 6 axiomes sur 100+ verdicts — cartographier la geometrie reelle
3. Comparer trimmed mean vs Bayesian posterior vs median sur le corpus calibre
4. Comparer exponential vs power law decay sur les 8 chess crystals (donnees existantes)
5. Mesurer le ceiling naturel des Dogs calibres — phi-inverse est-il le bon plafond ?
6. **Falsification:** Si les choix actuels performent a <5% d'ecart des alternatives, la philosophie tenait. Sinon, migrer.

**Dimensions physiques/math applicables:**
- **Information Theory** : MI inter-Dog (redondance vs independance), channel capacity stimulus->verdict
- **Bayesian Inference** : prior = biais mesure du Dog, likelihood = score, posterior = croyance reelle
- **Signal Processing** : GARCH pour volatility clustering (KAIROS), autocorrelation des scores
- **Thermodynamique** : tokens=ATP, crystals=entropie reduite, K15=2eme principe, challenge=annealing
- **Geometrie** : l'espace QScore est un simplex 6D projete sur [0, phi^-1]. Quelle metrique ?

**Le pont traditions-mesure:**

Les 22 traditions (docs/identity/CYNIC-PERENNIAL-EPISTEMOLOGY.md) ne sont pas decoratives — chacune encode une structure math/physique specifique qui indique QUOI mesurer. Les maths indiquent COMMENT.

| Tradition | Structure math encodee | Mesurable dans CYNIC |
|---|---|---|
| Pyrrhonisme (isosthenia) | MI = 0 entre observateurs independants | MI(Dog_i, Dog_j) sur verdicts existants |
| Alchimie (solve et coagula) | Transition de phase | Temperature critique = f(variance) pour cristallisation |
| Kabbalah (Sefirot) | Graphe d'interdependances | MI matrix 6x6 axiomes — quels axiomes sont redondants ? |
| Pythagore (phi) | Self-similarity fractale | phi-bound produit-il la meme geometrie a chaque couche ? |
| Shannon / Hermetisme | Channel capacity | Bits de signal par verdict |
| Zoroastrisme (Asha vs Druj) | Signal detection theory (ROC, d') | Sensitivity + specificity avec RT3 ground truth |
| Stoicisme (katalepsis) | Bayesian posterior (distribution, pas point) | Remplacer trimmed mean par posterior |
| Taoisme (wu wei) | Minimum Description Length | BURN = MDL. Complexite de Kolmogorov des verdicts |
| Russell (#22, cones opposes) | Ondes stationnaires / equilibre dynamique | Crystal decay: exponentiel vs power law |
| Confucius (zhengming) | Type theory / contracts formels | Les noms dans le code = les contrats (Rule 13) |
| Kalama Sutta (test in experience) | Falsifiabilite / p-values | Chaque track a un critere de rejet |

**Sources:**
- Shannon (1948) — channel capacity, mutual information
- arXiv:2601.01522 — Bayesian orchestration of multi-LLM
- Mandelbrot, Taleb — power laws in financial time series
- arXiv:2512.22245 — linear probes for calibration (empirical ceiling measurement)
- docs/identity/CYNIC-PERENNIAL-EPISTEMOLOGY.md — 22 traditions mappees
- docs/identity/CYNIC-PRIMARY-SOURCES.md — Hall/Kybalion/Frabato integration table

### RT1 — Calibre sur quoi ?

**Etat:** chess (prouve, 8 crystals), trading (1 forming), token (8 verdicts), dev (nightshift bruit), on-chain (0).

**Probleme:** On juge du texte partout — y compris pour le trading ou c'est le mauvais medium. KAIROS envoie "LONG SOL at $148" et les Dogs jugent la description verbale, pas le trade.

**Hypothese:** Les Dogs ne peuvent pas evaluer la qualite d'un trade a partir de sa description textuelle. Les scores trading reflètent la qualite linguistique, pas la qualite du signal.

**Protocole:**
1. Construire 20 stimuli trading avec outcomes connus (10 gagnants, 10 perdants)
2. Soumettre aux Dogs SANS outcome -> enregistrer scores
3. Correlation scores vs outcome reel
4. **Falsification:** Si correlation > 0.3 (p<0.05), les Dogs captent du signal. Si <= 0.3, confirme que le medium est faux.

**Si falsifie:** Explorer timesfm (Google, deja dans les stars, 18K) pour le signal numerique. ML classique (volatility clustering 5.6x confirme, anomaly detection) au lieu de LLM-as-judge pour le trading.

**Sources:**
- KAIROS walk-forward: "4h market 99.84% unpredictable by standard features"
- Volatility clustering: P(big|big) = 19.5% vs 3.5% baseline
- `google-research/timesfm` (starred)
- `thuml/Time-Series-Library` (forecasting + anomaly detection)

---

### RT2 — Calibre par qui ?

**Etat:** 5 LLM Dogs + 1 deterministic. Tous LLM-as-judge. Zero diversite d'architecture. qwen-7b-hf anti-discrimine (degenerate variance=0), gemini-cli crashe (activate_skill). 

**Probleme:** La recherche (arXiv:2601.01522) montre que la diversite d'architecture > diversite de prompt pour les ensembles. CYNIC a 5 variations du meme paradigme (LLM text-in → scores-out).

**Hypothese:** Ajouter un Dog non-LLM (rules-based, ML classique, ou time-series model) reduirait max_disagreement et augmenterait la stabilite des verdicts.

**Protocole:**
1. Mesurer baseline : inter-Dog agreement (Krippendorff alpha) sur 50 stimuli par domaine
2. Identifier les cas ou TOUS les LLM Dogs echouent mais le deterministic-dog reussit (et inversement)
3. Prototyper un Dog "anomaly-detector" (non-LLM) pour trading
4. Re-mesurer alpha avec le nouveau Dog
5. **Falsification:** Si alpha n'augmente pas de >0.05, la diversite d'architecture n'aide pas ici.

**Si falsifie:** Le probleme est dans les prompts, pas dans l'architecture. -> RT4 (prompt evolution).

**Sources:**
- arXiv:2604.15302 — 25% inconsistance sur cas durs
- arXiv:2601.01522 — diversite architecturale > diversite de prompt
- arXiv:2508.06225 — overconfidence in LLM-as-a-Judge
- `imbue-ai/darwinian_evolver` (starred) — evolue prompts mais aussi architectures
- Prometheus-2 7B (sur disque, pas deploye) — judge-trained model

---

### RT3 — Calibre contre quoi ?

**Etat:** AUCUN ground truth. 0 corpus labele. 0 human label. 0 benchmark externe.

**Probleme:** Sans ground truth, on ne sait pas si les Dogs sont bons, mauvais, ou aleatoires. Toute amelioration est immesurable.

**Hypothese:** Un corpus de 30 stimuli par domaine (10 easy-positive, 10 easy-negative, 10 ambiguous) avec human labels suffit pour calibrer les Dogs et detecter les biais systematiques.

**Protocole:**
1. **Chess** (existant) : utiliser le tier-matching existant (17 stimuli, 29% match). Labeliser manuellement le tier attendu pour chaque stimulus. C'est le ground truth le plus facile.
2. **Trading** : 20 stimuli avec outcomes connus (cf. RT1)
3. **Token** : 15 stimuli (5 rug pulls confirmes, 5 projets solides, 5 ambigus). Human labels: BARK/HOWL/WAG attendu.
4. **Dev** : 15 commits reels (5 excellent, 5 bugge, 5 refactor neutre). Labeler par le human.
5. **General** : 10 stimuli divers (essays, code, claims factuelles)
6. Pour chaque corpus : soumettre aux Dogs, comparer Dog verdict vs human label
7. **Falsification:** Si agreement humain-Dog (Cohen kappa) < 0.4 sur easy cases, les Dogs sont mal calibres et tout le reste (crystals, injection, A/B) est premature.

**Outils:**
- `argilla/synthetic-data-generator` — generation assistee
- `confident-ai/deepeval` (14.8K) — 14+ metriques d'evaluation
- Manuellement : le human labele, c'est le ground truth le plus fiable

---

### RT4 — Comment mesurer ?

**Etat:** json_valid_rate, QScore, max_disagreement. Pas de kappa, pas de calibration curve, pas de drift tracking.

**Probleme:** Les metriques actuelles mesurent la FORME (le Dog a-t-il repondu en JSON valide?) pas la SUBSTANCE (le Dog a-t-il juge correctement?).

**Hypothese:** 5 metriques additionnelles transformeraient CYNIC d'un scorer aveugle en un systeme auto-calibrant.

**Metriques proposees:**
1. **Krippendorff alpha** par domaine — inter-rater reliability (tous les Dogs)
2. **Calibration curve** — P(HOWL) vs frequence reelle de contenu quality (necessite RT3)
3. **Per-Dog bias profile** — biais moyen par axiome (sovereignty saturation de qwen-7b-hf = +0.618 sur tout)
4. **Drift tracker** — scores du meme corpus a J0, J7, J14, J30 (non-deterministic drift, arXiv:2601.19934)
5. **Discrimination index** — capacite a distinguer easy-positive de easy-negative (necessite RT3)

**Protocole:**
1. Implementer chaque metrique comme une pure function dans `domain/calibration.rs`
2. Ajouter endpoint `GET /calibration` (ou dans /metrics)
3. Nightshift redesigne : au lieu de juger des commits, run le calibration corpus periodiquement et track les metriques
4. **Falsification:** Si alpha > 0.6 ET discrimination > 0.8 sur easy cases, les Dogs sont calibres. Sinon, RT4b: prompt evolution.

**RT4b — Prompt evolution (si calibration echoue):**
- `darwinian_evolver` (imbue-ai, starred) : organism=Dog prompt, evaluator=calibration corpus fitness, mutator=LLM
- `MetaSPO` (NeurIPS 2025) : bilevel optimization, plus rigoureux
- `PromptBreeder` (DeepMind, arXiv:2309.16797) : self-referential, evolue aussi les mutation-prompts
- **Falsification:** Si 50 generations d'evolution n'ameliorent pas discrimination de >0.1, le probleme est le modele, pas le prompt.

**Sources:**
- arXiv:2512.22245 — linear probes for judge calibration (70-92% improvement)
- arXiv:2601.03444 — grading scale impact (0-5 > 0-10)
- arXiv:2506.13639 — design choices interact non-linearly

---

### RT5 — Comment compound ?

**Etat:** Crystals = retrieval (injecter du contexte dans les prompts Dog). A/B negatif (Delta=-0.013, N=30). Nightshift = bruit.

**Probleme:** "Compound" dans CYNIC signifie "ajouter du contexte au prompt." Ce n'est pas du learning — c'est du RAG applique au jugement. Le RAG peut nuire si le contexte est du bruit (crystal injection A/B negatif).

**Hypothese:** Crystal injection ameliore le jugement SI ET SEULEMENT SI les crystals sont formes a partir de verdicts concordants (agreed, weight=1.0) sur des stimuli ou le ground truth est connu.

**Protocole:**
1. Prerequis : RT3 (corpus) + RT4 (metriques)
2. Baseline : N=100 stimuli (mix de tous les domaines), crystals OFF, enregistrer QScore + per-Dog scores
3. Former des crystals a partir de 50 stimuli concordants (max_disagreement < 0.236)
4. Re-mesurer les memes 100 stimuli avec crystals ON
5. Paired t-test par domaine
6. **Falsification:** Si Delta <= 0 ou p > 0.05, crystal injection est nuisible ou neutre -> redesign ou suppression.

**Si falsifie:**
- Explorer `MemAlign` (Databricks 2026) — memory-augmented judge avec feedback
- Ou : crystals ne servent pas a l'injection prompt mais a la detection de drift (usage diagnostique, pas prescriptif)

**Sources:**
- A/B existant : Delta = -0.013 (N=30, faible puissance statistique)
- MemAlign : memory improves judge consistency on ambiguous cases
- arXiv:2510.12697 — multi-agent debate with adaptive stability detection

---

### RT6 — Comment souverain ?

**Etat:** GPU RTX 4060 Ti 99%+ idle. HF Inference API = cognition louee. Modeles sur disque non deployes (Prometheus-2 7B, Qwen3-4B, Phi-4-mini). cynic-core (Gemma 4-E4B) est le seul Dog souverain.

**Probleme:** 4/5 LLM Dogs dependent d'infra externe (HF, Google, GPU remote). La "souverainete" est declaree, pas materielle.

**Hypothese:** Remplacer qwen-7b-hf (anti-discrimine, HF API) par Prometheus-2 7B (judge-trained, deja sur disque) ameliorerait la qualite du jugement ET la souverainete.

**Protocole:**
1. Deployer Prometheus-2 7B Q5 sur llama-server (cynic-core, Vulkan ou CPU)
2. Adapter le prompt (Prometheus-2 attend rubric+reference, pas axiom-based)
3. Mesurer sur le calibration corpus (RT3) : Prometheus-2 vs qwen-7b-hf
4. **Falsification:** Si Prometheus-2 discrimination < qwen-7b-hf, le judge-trained model n'aide pas sur nos axiomes specifiques.

**Feuille de route souverainete :**
- Phase 1 : Prometheus-2 remplace qwen-7b-hf (1 Dog souverain de plus)
- Phase 2 : timesfm pour KAIROS signal (ML classique souverain)
- Phase 3 : darwinian_evolver tourne en local pour evoluer les prompts (auto-amelioration souveraine)
- Phase 4 : Hermes (NousResearch) comme agent autonome sur GPU local

**Sources:**
- Prometheus-2 7B (arXiv:2405.01535, Pearson 0.6-0.7 avec GPT-4)
- `llmfit` (starred) — hardware-aware model fitting
- RewardBench 2 (arXiv:2506.01937) — benchmark pour judge models

---

### RT7 — Proof of Work / Proof of Improvement

**Etat:** Aucune metrique de progres longitudinale. Chaque session est une ile.

**Probleme:** CYNIC n'a aucun equivalent du CTR/conversion rate des GAFAM. Impossible de savoir si l'organisme s'ameliore ou stagne. Les sessions produisent du code, pas de la preuve.

**Principe:** Chaque session doit bouger un nombre. Si aucun nombre ne bouge, la session est de la chaleur dissipee (2eme principe thermodynamique).

**Dashboard de sante mesurable:**

| Metrique | Ce qu'elle mesure | Baseline 2026-04-17 | Cible |
|---|---|---|---|
| Inter-Dog alpha (Krippendorff) | Fiabilite de l'ensemble | ~0.4 (estime) | > 0.6 |
| Dog discrimination index (mean range) | Capacite a distinguer | det=0.086, q35=0.561 | all > 0.3 |
| Max pairwise redundancy (r) | Waste dans l'ensemble | **0.898** (gemma x qwen7b) | < 0.6 |
| Crystals par domaine | Couverture memoire | chess=8, trading=1, rest=0 | >= 5 par domaine actif |
| Verdict accuracy vs ground truth | Correctness | **INCONNU** | > 0.7 (Cohen kappa) |
| Crystal A/B Delta | Impact injection | **-0.013** (negatif!) | > +0.05 ou suppression |
| Forming to Crystallized ratio (non-chess) | Efficacite pipeline | 0% | > 20% |
| Dog sovereign ratio | Independance infra | 1/5 (20%) | > 3/5 (60%) |

**Protocole:**
1. Creer `GET /dashboard` (ou section dans /health) qui retourne ces 8 metriques
2. Chaque session log les metriques BEFORE et AFTER dans le session log
3. **Falsification:** Si 3 sessions consecutives ne bougent aucune metrique, le programme de recherche est mal oriente.

**Ancrage materiel (3 couches progressives):**
- **Immediat:** Human-in-the-loop — `POST /feedback` (verdict_id, correct: bool). Le human labele.
- **Jours:** External benchmarks — Dogs sur RewardBench 2, chess tier-matching, JudgeBoard.
- **Semaines:** Verifiable outcomes — KAIROS P&L reel, Solana rug detection, code bugs post-commit.

Chaque couche renforce la suivante. Human feedback calibre les Dogs. Les benchmarks valident la calibration. Les outcomes prouvent que les jugements ont un impact reel.

---

## Dependances entre tracks

```
RT3 (ground truth) ──┬──> RT4 (metriques)
                     ├──> RT5 (crystal A/B)
                     └──> RT2 (architecture diversity)

RT1 (medium trading) ──> RT6 (timesfm souverain)

RT4b (prompt evolution) ──> depend de RT4 echec

RT7 (proof of work) ──> mesure transversale, chaque session

Tout ──> depend de RT3 (ground truth first)
RT7 ──> valide que le programme avance (meta-falsification)
```

**RT3 est le goulot.** Sans corpus labele, rien n'est mesurable. C'est le premier travail.

---

## Dimensions non couvertes (gaps identifies 2026-04-17)

### G1 — Social/memetique
CYNIC juge du contenu isole de son contexte de propagation. Le pattern de propagation (qui partage, vitesse, reactions, graphe social) EST du signal L0. Un rug pull a une signature sociale AVANT le rug. Zero ingestion X/Telegram/on-chain social graph.

**Recherche necessaire :** Stance detection, rumor propagation networks, social signal ML.
**Connection traditions :** Zoroastrisme (Druj propage via le mensonge social), Confucius (zhengming — les noms corrompus se propagent socialement).

### G2 — Adversarial
Que se passe-t-il quand quelqu'un game CYNIC ? Prompt injection dans les stimuli, inputs crafted pour exploiter les biais des Dogs, Sybil attacks sur le crystal pipeline. Le FOGC test est philosophique. Aucun adversarial testing ML.

**Recherche necessaire :** JailbreakBench, red-teaming LLM-as-judge, adversarial prompt detection.
**Connection traditions :** Frabato ch.II (FOGC infiltration — l'attaque vient de l'interieur, pas de l'exterieur).

### G3 — Cross-domain
HOWL en trading + HOWL en code-audit + HOWL en social = signal fort. Mais chaque verdict est une ile. Pas de graphe de conviction multi-domaine.

**Recherche necessaire :** Belief propagation, multi-view learning, knowledge graph reasoning.
**Connection traditions :** Hermetisme ("as above, so below" = le MEME pattern dans des domaines differents confirme la verite).

### G4 — Representation
Les crystals sont du texte + confidence. Primitif. La geometrie informationnelle du jugement est plus riche : distribution des Dog scores, structure par axiome, contexte temporel. Comprimer a 1 nombre perd presque tout.

**Recherche necessaire :** Information geometry (Fisher metric), topological data analysis (forme de l'espace des verdicts), representation learning.
**Connection traditions :** Kabbalah (Sefirot = 10 dimensions, pas 1. Reduire a 1 nombre = perdre les Sefirot).

### G5 — Agentique (le gap existentiel)
CYNIC juge mais N'AGIT JAMAIS. Les verdicts ne changent rien dans le monde materiel. L'organisme entier est un producteur sans consommateur = K15 violation de lui-meme.

3 consommateurs potentiels, aucun actif :
- KAIROS : DRY_RUN=true, verdicts ignores
- Hermes : pas deploye
- Humain : doit curl manuellement

**Recherche necessaire :** Active Inference (Friston) — l'organisme minimise sa surprise EN AGISSANT, pas juste en observant. Embodied cognition.
**Connection traditions :** Kybalion p.8 ("storing = hoarding precious metal" = K15). Zoroastrisme (Asha exige l'action, pas juste la reconnaissance).

---

## Anti-patterns a eviter

1. **Builder avant de mesurer.** Ne pas redesigner nightshift, crystal injection, ou Dog prompts avant d'avoir les metriques RT4.
2. **Confondre retrieval et learning.** Crystal injection est du RAG, pas du ML. Ca peut aider ou nuire.
3. **Optimiser le scorer sans ground truth.** Ameliorer QScore sans savoir s'il correle avec la qualite reelle = optimiser dans le vide.
4. **Ajouter des Dogs sans mesurer les existants.** Savoir d'abord lesquels marchent (RT2/RT4).
5. **Ignorer le hackathon.** 10 jours au feature freeze. Ce programme est long-term. Le demo est court-term. Ne pas les confondre.
6. **Boucle fermee sur soi-meme.** opinions -> crystals -> opinions = chambre d'echo. Sans ancrage materiel (outcomes, human labels, benchmarks), CYNIC est l'oracle sans accountability qu'il pretend combattre.
7. **Confondre volume et signal.** Produire 100 crystals de bruit n'est pas mieux que 0. La variance, la discrimination, la preuve d'amelioration sont les seules metriques qui comptent.
8. **Sous-estimer L0.** Les GAFAM dominent par la donnee brute, pas par l'algorithme. Sans flux L0 frais (candles, on-chain, diffs), les couches superieures sont du chateau de sable.

---

*Epistemic status: ce programme est une CONJECTURE structuree (confidence 0.45). Chaque track sera validee ou rejetee par ses propres donnees. Le programme lui-meme est falsifiable : si RT3 montre que les Dogs sont aleatoires, les tracks RT4/RT5 sont inutiles et CYNIC doit pivoter.*
