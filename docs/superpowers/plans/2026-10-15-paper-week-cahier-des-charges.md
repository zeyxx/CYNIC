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
