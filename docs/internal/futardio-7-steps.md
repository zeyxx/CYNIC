# Futardio Submit — 7 Étapes Préparées

**Deadline :** June 2, 2026
**URL :** https://futard.io → "Launch a project"
**Durée estimée :** 15-20 min si tout est prêt

---

## Étape 1 — Identité du projet

| Champ | Valeur |
|-------|--------|
| **Project name** | Talaria |
| **Token name** | Talaria |
| **Ticker symbol** | $TALARIA |

> ⚠️ Utiliser "Talaria" (pas "TALARIA" en majuscules) — Futardio formate lui-même.

---

## Étape 2 — Token image & Banner

| Champ | Valeur |
|-------|--------|
| **Token image** | https://talaria-deploy.vercel.app/talaria-token.svg ✅ |
| **Banner (optional, 16:9)** | https://talaria-deploy.vercel.app/banner.svg ✅ |
| **Fichier local token** | `docs/internal/talaria-token.svg` (400×400px) |
| **Fichier local banner** | `/tmp/talaria-deploy/banner.svg` (1200×630px) |

> ⚠️ Si Futardio n'accepte pas SVG : ouvrir dans browser, screenshot + recadrer en PNG.

---

## Étape 3 — Description

### Short Description (tagline onchain)
```
Is the proposal sound? Are the voters human? We answer both.
```

### Project Description (markdown)

```markdown
## The Problem

Futarchy lets markets decide. But who's in the market? Bots? Sybils? Nobody checks if voters are human or if proposals are sound before the vote starts.

## The Solution: TALARIA

Two live products. One trust layer.

**CYNIC — AI Judgment Engine**
- 4 independent AI validators evaluate proposals on 6 axioms
- Confidence capped at φ⁻¹ = 61.8% — the architecture refuses certainty beyond the evidence
- EPOCHÉ: judgment suspended when models disagree (no forced verdicts)
- < 2s latency · Rust kernel · sovereign hardware · zero cloud

**Blitz & Chill — Game-based Proof of Humanity**
- Play chess. Prove you're human. No KYC, no biometrics.
- QR code → verified in 60 seconds
- Puzzles, tribes, inter-community tournaments, soulbound NFT mint
- Stockfish 18 WASM engine

**The community flywheel**
Inter-community tournaments: "$TOKEN_A DESTROYED $TOKEN_B 7-3" — self-propagating content, zero ad spend. Communities onboard each other through rivalry.

## By the Numbers

| Metric | Value |
|--------|-------|
| Verdicts rendered | 1,874+ |
| Observations processed | 56,000+ |
| Tests (CYNIC + B&C) | 2,000+ |
| Commits (CYNIC + B&C) | 1,400+ |
| Cloud dependency (AI engine) | 0% |

## Token Utility

**Access token:** hold $TALARIA to unlock judgment API tiers beyond free quota.
**Burn:** tournament entry burns $TALARIA — inter-community events = deflationary pressure at scale.

## Use of Funds

Monthly burn: $6,050/month × 6 months = $36,300 (73% of raise)

- T. — CYNIC kernel, judgment API: $3,000/month
- S. — Blitz & Chill, community: $3,000/month
- Infrastructure (sovereign hardware, electricity): $50/month

Reserve: $13,700 (27%) — contingency + future hiring

## Roadmap

- **M1–2:** Public judgment API · server-side PoH · first inter-community tournament
- **M3–4:** PoH on-chain mainnet · burn-as-a-service · checkers/dames · first external client
- **M5–6:** sBPF optimization · Go support · SDK · Season 1

## Why TALARIA?

Talaria — the winged sandals of Hermes. Speed, trust, and the messenger who never lies.

Built in the open: github.com/zeyxx/CYNIC
```

---

## Étape 4 — Paramètres de la raise

| Champ | Valeur |
|-------|--------|
| **Raise goal** | $50,000 USDC |
| **FDV** | $90,000 |
| **Supply** | 18,000,000 $TALARIA |
| **ICO duration** | 7 days |
| **Prix ICO** | $0.005 / $TALARIA |

---

## Étape 5 — Plan de dépenses mensuelles

| Champ | Valeur |
|-------|--------|
| **Monthly spend** | $6,050 |

### Détail du plan

```
Month 1–2: $6,050/month
- T. (CYNIC kernel, API): $3,000
- S. (B&C, community): $3,000
- Infrastructure (électricité ~180 kWh/mois @ €0.25/kWh): $50

Month 3–4: $6,050/month
- Même split
- On-chain deployment costs: couvert sur budget opérationnel

Month 5–6: $6,050/month
- Même split
- SDK + Season 1 prize pool: financé sur protocol fees

Total 6 mois : $36,300 (73% de la raise)
Réserve      : $13,700 (27%) — contingency + future hiring
```

### Breakdown % raise

| Poste | Montant | % |
|-------|---------|---|
| T. salaire (6m) | $18,000 | 36% |
| S. salaire (6m) | $18,000 | 36% |
| Infra électricité (6m) | $300 | 1% |
| Réserve | $13,700 | 27% |
| **Total** | **$50,000** | **100%** |

---

## Étape 6 — Treasury wallets (3+ adresses Solana)

| # | Titulaire | Adresse |
|---|-----------|---------|
| 1 | **T.** | `dcW5uy7wKdKFxkhyBfPv3MyvrCkDcv1rWucoat13KH4` ✅ |
| 2 | **S.** | `⚠️ À REMPLIR — adresse Solana de ragnar-no-sleep` |
| 3 | **Backup** | `⚠️ À REMPLIR — cold wallet T. ou GCRtrd` |

> Multisig 2/3 recommandé pour tout mouvement de trésorerie.

---

## Étape 7 — Legal & Site web

| Champ | Valeur |
|-------|--------|
| **Website URL** | https://talaria-deploy.vercel.app ✅ |
| **Terms of Service URL** | https://talaria-deploy.vercel.app/tos ✅ |
| **Token image URL** | https://talaria-deploy.vercel.app/talaria-token.svg ✅ |
| **Ethereum wallet** | `0xfD0759E929447c53143Df13278d822BE12dF9670` ✅ |
| **Cayman entity** | Créer via MetaLeX pendant le submit (~5 min) |

---

## Tokenomics (MetaDAO standard — finalisée)

| Bucket | Tokens | % | Unlock |
|--------|--------|---|--------|
| ICO (public raise) | 10,000,000 | ~39% | à la clôture raise |
| LP pool (auto-seeded) | 2,900,000 | ~11% | à la clôture raise |
| T. — performance package | 3,600,000 | ~14% | price-based : 2×/4×/8×/16×/32× × $0.005, min 18 mois |
| S. — performance package | 3,600,000 | ~14% | price-based : 2×/4×/8×/16×/32× × $0.005, min 18 mois |
| Protocol treasury | 900,000 | ~3% | governance |
| **Total** | **~21,000,000** | **100%** | |

---

## Checklist finale avant de payer le 0.5 SOL

- [x] SOL reçu (GCRtrd)
- [x] Adresse wallet T. (étape 6)
- [x] ETH wallet prêt (étape 7)
- [x] Website live (étape 7)
- [x] ToS URL live (étape 7)
- [x] Token image live (étape 2)
- [x] Banner live (étape 2)
- [x] Description finalisée (étape 3)
- [x] Tokenomics finalisée (option A)
- [ ] Adresse wallet S. (étape 6) ← **BLOQUEUR**
- [ ] 3ème adresse Solana (étape 6) ← **BLOQUEUR**
- [ ] Cayman entity MetaLeX (étape 7, en live)

---

*Mis à jour le 2026-05-31 — en attente wallets S. + backup*
