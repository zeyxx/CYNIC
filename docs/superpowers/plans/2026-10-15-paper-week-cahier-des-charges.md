# Paper Week Exit Evaluation — Cahier des charges v1

> Date: 2026-10-15. Compagnon: `2026-10-15-paper-week-prereg.md` (hypothèses gelées, fait foi en cas de conflit).
> Division: GCRtrd = détection/entrées. Nous = protocole d'évaluation des sorties. Read-only, 0 dépense, aucun trade réel.

## 1. Objectif et non-objectifs

- Évaluer quelle règle de sortie maximise une entrée donnée, à protection du capital égale.
- Semaine 1 = pilote + corpus (tous les chemins de prix stockés). Verdict en semaine 4 (n ≥ 60–80 par bras).
- Non-objectifs: aucun entraînement de modèle en semaine 1, aucune clé, aucun trade réel, pas de Render.

## 2. Sources gelées

- TrenchScanner @ cffd91c, CultScreener @ db0a9ae (dashboard /trenches = redirect 301 vers trenchscanner.app).
- HolDex: déprécié, hors scope.
- Registre: `infra/external-pins.json`. Manifests: `data_system/vault/captures/external-pins-2026-10-15/`.
- Checkouts: `~/.local/share/cynic/src/external/` (dev), `/var/lib/cynic/source/repositories/external/` (installé).

## 3. Contrat données GCRtrd (message prêt à envoyer)

« Pour la semaine de test, j'ai besoin de rejouer tes appels à l'identique. Telegram seul ne suffit pas
(une partie des appels, sans prix d'entrée ni résultats). Peux-tu me donner:
1. Tous tes appels, de tous tes modèles — pour chacun: quel coin, jour et heure (naissance, pas envoi),
   prix d'entrée (frais si dispo), quel modèle a appelé.
2. Pour chaque appel, le résultat après coup: x2? x4? tombé à −50% avant?
3. Les prix minute par minute de chaque coin appelé.
4. L'analyse TokenSage de chaque coin appelé + ta définition du filtre Gemerald (seuils).
5. Un accès en lecture: compte abonné pour mon robot, ou endpoint read-only, ou dump historique 30j.
   Le plus simple pour toi. »

Détail technique (noms exacts du pin cffd91c — si GCRtrd préfère construire l'endpoint lui-même):
1. Curated calls, TOUS models: id, tokenId (+ mint, symbol), createdAt (ms), source, model, modelName,
   confidence, tier, calibratedPct, reasons[], anchorPriceUsd, anchorMcapUsd, liveMcapUsd, snapshotId,
   candidateOutcomeId, narrativeVerdict/NotedAt/Rationale, verdicts (hit2xIn15m, hit2xIn1h, hit4xIn1h,
   hit10xIn1h, disqualified, peak24hReturnPct, runPeakMinutes, outcomeFinalizedAt, simReturnPct,
   peakMcapUsd/At). Index existant (model, createdAt): zéro charge.
2. CandidateOutcome full-population (pas seulement curatées): ligne complète (ancre, labelRule,
   sampleKind, agrégats fenêtre, simReturnPct, état trailing).
3. TokenSnapshot: lignes complètes par token (takenAt, prix, mcap, holders...) sur fenêtre + watch étendue.
4. TokenSage: shape GET /tokens/:mint/sage (status, read{depth, analyzedAt, rulesVersion, launchpad,
   curveProgress, pair, logo, creatorFee, summary, tickerExplanation, referent}, marketCap[], track, narrative).
5. Matchs + Gemerald: lignes Match (filterId, tokenId, snapshotId, matchedAt ms, score, peak, alertMcapUsd)
   + définition du filtre Gemerald (seuils).
6. Meta: champion (modelId + depuis quand), governor (target/heure, cooldown), roster modèles, training run ids.
7. Accès au choix: (a) route read-only nouvelle, (b) session abonné pour le robot, (c) dump 30j.
   JSON, liste de champs gelée au pin, curseur createdAt, rate-limit existant OK. Poll 1/min depuis notre infra.

## 4. Bras A (référence, figé)

- Heure = CuratedAlert.createdAt (ms). Jamais réception Telegram (latence 1s–minutes mesurée).
- Entrée = freshQuote si présent et plausible (ratio ≤ 5 vs prix scan), sinon prix scan.
- Sortie = EXIT_PLAN du pin (profitSim.ts): moitié à x2, reste suivi −35% au prix vu,
  stop −50% avant vente, clôture 30 min si jamais vendu, plafond suivi 3h.
- Coûts: −1% entrée, −1% par patte de vente. Identiques aux deux bras.
- Lignes rule-4 uniquement. Champion + seuils gelés (J-1, modelId nommé).

## 5. Bras B (mesure, figé)

- Mêmes entrées que A. Sortie suivi confirmée par clôture bougie au lieu du tick.
- Timeframe: 1m par défaut (TO-FREEZE avant premier signal).
- Chaque appel note les deux sorties (tick et clôture) pour comparaison appariée.

## 6. Hypothèses et stats (détail = prereg)

- H1 primaire (clôture > tick, Wilcoxon apparié, p<0.05 ET |Δ|≥5pp). H2 topslice, H3 Gemerald: A vs B
  DANS le sous-ensemble (Holm). Comparaisons inter-populations = descriptives uniquement.
- Médiane + IQR primaires. Trades clos dans les deux bras uniquement + taux de clôture publié (<90% = limite).
- Fidélité: |paper − grade GCRtrd| ≤ 5pp acceptable.

## 7. Ledger et rapports

- JSONL append-only, ligne 0 = SHA-256 de la prereg. Capture brute figée + hashée AVANT traitement.
- n signaux = n lignes évaluées-ou-exclues. Manquant → NON_ÉVALUABLE, jamais imputé.
- Rapport quotidien: format fixe (n, clôtures, médianes A/B, écart fidélité, lien preuve). Chaque chiffre cite capture + ledger.
- Distribution X avec crédits existants: texte + lien preuve complète.

## 8. Hébergement (0 dépense)

- Infra propre, boucle de lecture locale du feed, SQLite canonique, captures hors git. Pas de Render.

## 9. Findings 4 prismes intégrés

- ML: champion/seuils/rule-4 gelés; regrade historique = risque leakage (hors scope paper, noté).
- Data: puissance (semaine 4), Wilcoxon preregistré, survie/censure, granularité fixée, tolérance fidélité 5pp.
- Quant: slippage+fees −1%/−1%, suivi au prix vu (déjà son code), wicks intra-minute invisibles acceptés.
- Scientifique: prereg datée, ledger chaîné, createdAt corrigé, captures par signal, revue GCRtrd + 10% recalculé.

## 10. Planning

- J-1: freeze (champion, timeframe, accès données, Gemerald, heure rapports) + test fidélité sur historique.
- J1–J7: pilote + corpus total des chemins.
- J8–J28: run, verdict J+28 relu par GCRtrd.
- Go bot réel: seuils du verdict + funding, décidés avant mesure (pas après les chiffres).

## 11. Inconnues à geler avant premier signal

Voir prereg § Explicit unknowns (5 points). Aucun signal exploité avant leur gel.

## 12. Autonomie données (v2, 2026-10-16 — ne touche ni hypothèses ni stats)

On tire nous-mêmes, sans GCRtrd :
- Bougies 1 min par mint : API publique Pump.fun (sans clé, 1000 dernières minutes tradées).
  Script testé : `scratch/paper-week/pumpfun_candles.py` (backfill + ingest, parser vérifié, erreur HTTP gérée).
- Replay A/B, ledger, rapports, X : notre côté.

Irreductible GCRtrd (impossible à reproduire) :
1. Liste d'alertes : mint, createdAt (naissance), anchor, modèles (labels = son edge).
2. Champs TokenSage par alerte : narrative, fee recipient (gate allowlist).
3. CSV paper-trades : stratégie, entrée/sortie (heure + prix), taille, fee/tip/penalty, modèles,
   narrative, fee recipient, motif de sortie (baseline fills + calibration w).
   Fallback alertes : scrape de nos propres messages Telegram (partiel, sans prix ni labels).
Ordre : bulk alertes+outcomes d'abord (replay B), CSV ensuite (calibrage coûts/w).

## 13. Findings bottom-up L0–L3 (2026-10-16, corpus Telegram 1797 alertes 12→16 oct + 169k bougies)

Protocole : couches, split train (12–14) / test (15–16), règles dérivées du train seul.
Moteur : `scratch/paper-week/replay.py` (bras A tick-proxy vs B close, apparié).
- L0 corpus : jointure alertes→bougies 1755/1797. Fidélité : 21,3 % touchent 2x (labels ~22 %). w mesuré = 0,63.
- L1 isolé CORRIGÉ (l'« overlap 0 » était un artefact : je comparais les premiers modèles seuls) :
  Agreement solo 30,6 % (n=565) > Agreement+Trees#2 26,7 % (n=439, 439 co-appels réels) > T2 solo 18,9 %.
  T2 DILUE Agreement. Modèles test-only (2e moitié, config feed changée) : filter-only 25 %,
  Recent 14 %, Blend 24 % — chiffres test-only, pas comparables au train.
- L1 GELÉ : préférer Agreement solo ; T2 seul ou en co-appel = dilution mesurée.
  Test-only exclus des comparaisons train.
- L2 confluence : ANTI-confluence. 1 modèle −0,06 > 2 modèles −0,12 > 3+ (−0,23).
  Les tokens que tout le monde appelle sont déjà cuits.
- L3 élimination : holders_null (−0,18), conviction <40 (−0,16) → second gate
  (holders connu ET conviction ≥40 ET 1 seul modèle) : +0,04 (n=419), survit au split
  (+0,039 train n=381 / +0,067 test n=39 — fragile, à confirmer en live).
- H1 close>tick FALSIFIÉE sur ce corpus (z=−11,3 ; close-5 pareil). Grille 9 cases :
  fraction haute (0,75) + trail serré (25) gagne — inverse du MC synthétique (queue réelle fine).
- Grille 54 cases (tp×trail×tick/c1/c3/c5/c15/c30) : loi de LATENCE — c1m > tick > c3 > c5 > c15 > c30
  dans 8/9 cellules ; tp 0,75 > 0,5 > 0,25 partout ; trail 25 > 35 > 50 partout.
  Best = tp0,75/trail25/close-1m (−0,132). Spread best–worst 17pp par la sortie seule.
  Le close-1m filtre le bruit intra-minute sans perdre la vitesse : ce n'est pas tick-vs-close,
  c'est la latence qui paie.
- Stacking G×heures×taille : +0,17 in-sample → −0,23 au test (n=9). EFFONDRÉ.
  Leçon : aucun stacking sans split strict. Seul G survit.
- Limites : 4 jours, un régime (baseline test −0,22 vs train −0,06) ; ancre = mcap réception
  (biais partagé, apparié) ; fills −1 %/−1 % supposés ; narrative inutilisable (coverage ~0).
- Tops (ancre entrée, 180 min) : P(≥2x)=26 %, ≥3x=16 %, ≥5x=7 %, ≥10x=2,3 %, jamais 100x.
  Médiane pic 1,3x, p90 4,2x. Temps médian au pic : 2 min (la latence d'entrée domine).
  Stop −50 % tue 14 % des futurs ≥2x (coût de protection mesuré).
- Grille sans trail (TP×fraction, reste au cap 3h) : TP 1,5 > 2 > 3 > 5 ; full > partiel.
  Best = sortie totale à 1,5x (−0,104, wr 36 %). TP 1,3 redescend (−0,116, les coûts mangent).
- L4 par segment : gate × best-exit = +0,077 (n=421, wr 42 %) vs +0,04 (gate+Rule4)
  vs −0,104 (best-exit seul). Les couches se multiplient.
- REVALIDATION exits-sur-G1 en split (8 cases TP×fraction, bras A) : TOUTES positives train
  (+0,05 à +0,15) et TOUTES négatives test (−0,03 à −0,15). Inversion complète :
  train-best = TP1,5/full (+0,152), test-least-bad = TP5/full (−0,031).
  Le GATE transfère entre régimes, les EXITS non. Sorties fixes = invalides inter-régimes ;
  exits conditionnelles au régime requises. L'ancien « TP1,5 best » était une moyenne qui
  cachait le flip.
- L5 heures strict (heures+ dérivées du train seul : 9/22/18/7/0) : train +1,08 (n=28, tiré par outliers),
  test +0,11 (n=47 ; médiane +0,065, ex-max +0,05 — survit sans le meilleur trade).
  Directionnel, n faible, à confirmer en live.
- REVALIDATION heures en split P(2x) : rien de stable sauf 22h (40→31 %) et 17h (39→33 %, hypothèses faibles) ;
  21h s'effondre (30→8 %, n=143 — le mauvais régime habite 21h UTC) ; 7h flippe (41→19 %).
  Ancien set d'heures partiellement invalide (9h, 7h tombent ; 22h, 0h, 18h tiennent).
  Heures = marqueurs de régime, pas des gates.
- AUDIT pré-méthodologie (2026-10-16) : exits globales INVALIDÉES (refaites sur G1 : flip train→test) ;
  heures partiellement invalidées ; E1/second-gate ARCHIVÉES (formules jamais enregistrées, non reproductibles) ;
  stacking/H1/WF-signe conservés (déjà en split) ; weather rejet conservé avec caveat (non revalidable hors-ligne).
- RÉGIME (3 proxys testés, tous échouent au split) : heures train-mauvaises → meilleures au test (flip) ;
  trailing hit-rate résolu : froid 34 % ≈ chaud 34 % (aucune séparation) ; composition du flux : dégénéré.
  Conclusion : PAS de marqueur de régime endogène transférable. Le régime est EXOGÈNE
  (config du feed + marché) — à mesurer en live, pas dans le corpus.
- Coûts d'entrée (G1 test, n=386) : P2x insensible aux pénalités plates (31,1 %→30,8 % à 5 % ;
  effet de seuil) ; médiane 1,45→1,38. Les coûts tuent par la math de sortie, pas le hit-rate.
- Bourrasques : 1–3 alertes/min ~28 % partout ; ≥4/min s'effondre (15 %, 20 %, 0 % — n fins,
  caveat artefacts de réémission). Hypothèse : les vagues massives = slop ou dumps.
- Conv-100 EXPLIQUÉ : drift médian +7 % (arrive trop tôt, non prouvé) vs +52 % (40–99) ;
  raisons = que des constats génériques (âge, band, croissance), JAMAIS les raisons qui prédisent
  (snipers, dev, concentration). Le modèle score sa confiance sur CE QUI DÉCRIT, pas ce qui prédit.
  Preuve par notre règle sémantique.
- SURVIE (courbes du 2x, ancre mcap, sans coûts) : 70 % des 2x avant 5 min, 87 % avant 15 min,
  95 % avant 30 min ; après = +1pp pour 2h30 de capital bloqué. Horizon optimal ≈ 30 min max.
  G1 strictement au-dessus à chaque t (20/24,5/26,8/28/28,3/28,7 % vs jetés 15–21 %).
- VOLATILITÉ : vol 5 min pré-signal, quartile chaud sous-performe les deux moitiés
  (train 19 % vs 31–34 % ; test 21,5 % vs 21–29 %, couverture 30 %). HYPOTHÈSE.
- PENTES (2026-10-16) : pente de régression 15 min pré-signal — quartile raide GAGNE les deux moitiés
  (train 37 % vs 22 % plat ; test 36 % vs 0 %, n=27/cellule — PISTE FRAGILE, couverture 13 %).
  Nuance vs drift : la montée RÉGULIÈRE paie, le déplacement brutal (spike) tue.
  Bêta vs marché pump.fun : flippe (train plat, test inversé) — pas de signal.
  Sharpe G1+TP1,5/full par trade : train +0,08 (std 1,93 — loterie) / test −0,14.
  Distribution en loterie : petits saignements + rares gros — la moyenne ment, la queue paie.
- SPEC RUN LIVE (alpha réel — ce que le corpus ne contient pas) : fills exacts (prix, slippage, tip) ;
  latence tick (réception→décision→fill, ms) ; config du feed (flag par alerte) ; régime marché
  (tendance SOL, volume pump.fun) ; bourrasques (compte/min) ; labels résolus à 30 min (pics en 1–2 min).
  Entrée : G1 + retard≈0 ; sortie : TP1,5/full en flux propre (hypothèse à valider, pas un gate).
- Walk-forward 3 plis (desk v0.2.0, sélection re-dérivée par pli, tout au registre) :
  wf0 ne retient RIEN (test non filtré −0,04) ; wf1 size=small → test +0,25 (n=38) ;
  wf2 size=small → test −0,21 (n=23). Le signe FLIPPE selon le régime.
  Verdict protocole : le gate mono-feature ne survit pas au walk-forward. Il faut du n
  (run live) ou des règles conditionnelles au régime — pas plus de stacking.
- Buckets loyaux : micro (<20k$, 89 %, P(2x)=25 %) vs établis (≥20k$, 11 %, P(2x)=38 %).
  Falaise réelle à 20k (23 %→38 %). Ancien bucket large (≥60k$, n=12) supprimé : fantôme.
  Seuil absolu fragile au régime → version percentile (top 10 % du jour) prévue.
- Couche sémantique (raisons ▸, 1306/1797 alertes — HYPOTHÈSES, pas gates) :
  H-sem1 « first 25 buyers still holding » 0,30→0,28 (test n=39, stable).
  H-sem2 « top-10 concentration » 0,33→0,31 (test n=13, stable mais fin).
  Rejeté : « move since dex price » comme gate poubelle (0,12→0,23, retour moyenne).
  Neutres confirmés : age, holder growth, band (~0,29 partout).
  Pistes sans n : snipers 64 % (n=14), dev buy 50 % (n=12).
  Verrouillé raisons (split) : snipers 80→56 % (n=5/9, piste) ; dev buy 44→67 % (n=9/3, piste) ;
  first-25 30→28 %, top-10 32→31 % (hypothèses) ; masse ~28 % neutre ;
  move-since et 5m-move REJETÉS. Règle : qui GARDE prédit, ce qui BOUGE décrit.
- Latence : délai détection médian 2,2 min ; dérive naissance→alerte +45 % (micro +40 %).
  Établis = ancre tick-réception (naissance on-chain inutile).
- Coût du retard d'entrée (ancre bougie réception, n≈1750) : +0 min 31 % · +1 min 26 % ·
  +2 min 23 % · +5 min 19 % · +10 min 14 %. Chaque minute ≈ −2pp ; 5 min = −40 % d'edge.
  La latence est le premier paramètre de la stratégie, devant la sortie.
  Gate plafond de dérive micro (split strict) : ≤+25 % → 30,0 %→30,1 % (n=270/309, STABLE,
  résiste au régime test pourri) ; ≥+50 % s'effondre au test. Verrouillé : drift ≤ +25 %.
- Combiné G1 = établis OU (micro ET drift ≤25 %) : train 32,7 % (n=385) → test 31,1 % (n=386),
  +7pp vs baseline test (23,7 %). G2 (+sem1) : 31,9 %→30,5 % (plus de volume, un peu dilué).
- CORPUS V2 (export-01, 1909 alertes, 1858 mesurées, jointure par seq — 88 paires même-seconde
  gardées distinctes) : baseline 26,5 % identique ; G1 33,0 % (n=409) → 29,7 % (n=407) ;
  WF 34,1/33,5/26,6. G1 tient, légèrement atténué sur régime frais.
- Erreur d'ancre : bougie-réception vs mcap-signal médiane −16 % (p10 −72 %, p90 +59 %) — ancre bruitée.
  CORRIGÉE : ancre exécutable (close bougie-réception) → baseline 31,2 %, G1 35,5 % (n=816).
  Le gate survit aux deux ancres ; seule la hauteur change.
- Survie : 50 mints sans bougies (40 micro, appels T2-family) — biais flatteur pour T2, quantifié.
  Note d'honnêteté : le niveau +25 % a été confirmé (pas découvert) au test — walk-forward requis
  pour la stricte validité (fait ci-dessous, acquis).
- Censure : 16 % d'alertes <2h d'observation (pics non formés) tirent tout vers le bas.
  Corrigé (cover ≥2h) : baseline 26,5 %→30,3 % (n=1472) ; G1 → 36,8 % (n=644).
  Tous nos chiffres publiés sont conservateurs.
- Walk-forward G1 (seuils verrouillés) : f0 33,7 % vs 30,5 % · f1 34,5 % vs 23,5 % · f2 28,4 % vs 23,9 %
  (n≈200/pli). Edge positif aux 3 plis. VALIDITÉ STRICTE ACQUISE.
- WF TOUS CANDIDATS (2026-10-16) : G1 seul survivant (+3/+11/+4,5pp).
  AgreeSolo f2 n=0 (Agreement a DISPARU du feed en fin de corpus — feed-dépendant, invalide) ;
  Holders f2 sous baseline ; Conv f2 sous baseline ; Age f0+f2 sous baseline ;
  G3/G4 f2 n=0 (inemesurables). Les gates à features de feed meurent avec le feed ;
  G1 survit car taille+drift sont feed-INDÉPENDANTS. C'est le pourquoi de sa robustesse.
- CONFLUENCES EXHAUSTIVES (127 sous-ensembles, 2⁷−1, n≥30/pli, positifs aux 3 plis) : 2 survivants,
  G1 (33,7/34,5/28,4) et G1+âge (33,3/34,3/29,3 — identique à n−10 près, aucun gain).
  Aucune confluence ne bat G1. Par parcimonie : G1 seul. G1+âge = hypothèse (sélectionnée sur 127,
  pas pré-enregistrée).
- NULS (audit) : mcap/âge/taille = 0 nul (G1 évaluable à 100 %, aucune exclusion) ;
  holders nuls 827, conv-0 467 (la mort par feed, chiffrée). Règle : une alerte inévaluable
  pour un gate = REJETÉE (sémantique décision-temps), comptée à part, jamais supprimée du corpus.
  Pas de normalisation (seuils, pas distances) ; pas d'imputation (les nuls sont informatifs).
- IMAGES (export-01, 1648 photos) : présence 27,1 % vs absence 20 % plein-corpus MAIS flippe au split
  (train 29/27, test 26/35, n≈50 — pas de signal, REJETÉ) ; dupes exactes 27,6 vs 27,6 (n=174, MORT) ;
  quasi-dupes perceptuels (dhash, 185 clusters, 441 fichiers) 27,4 vs 27,7 (MORT).
  La réutilisation visuelle ne prédit pas l'échec. Reste le niveau 3 : motifs des gagnants (codage manuel).
- ARBRES (SI-ALORS prof.2, appris train → test → WF) : 2 feuilles candidates, toutes TUÉES au WF.
  ageETest : 40/53/14 % (f2 sous baseline) ; vieuxETconv : 38/7/0 % (f0 AU-DESSUS de baseline —
  même le sens flippe). La structure conditionnelle sur-ajuste ; seul le OU simple de G1 survit.
  L'ordre des gates ne crée pas d'alpha : le ET est commutatif, le SI-ALORS ne transfère pas.
- Paramètres verrouillés (un par un, granularité maximale = 3 crans utiles, au-delà c'est du bruit) :
  holders : inconnu 23 % (stable, à jeter) < 0–150 ~25–28 % < 150+ ~35 % ; établis+connus 39 %.
  drift micro : ≤25 % = 30 % stable les deux périodes ; >25 % s'effondre à 17 % en mauvais régime.
  conviction : <30 à jeter (~23 %), 40–99 plateau (~28 %), 100 à jeter.
  âge : 0–1 min tient partout ; 2–4 min régime-dépendant ; ≥5 min meurt en mauvais régime.
  Forme générale : la fraîcheur (temps, drift, âge) protège du mauvais régime.
- G1 GELÉ (2026-10-16) : établis (mcap ≥20k$) OU (micro ET drift-naissance ≤+25 %).
  Plus aucun tuning sans données nouvelles. Référence registre : gates-v1 + WF ci-dessus.
