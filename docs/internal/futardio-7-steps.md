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
| Verdicts rendered | 2,195 |
| Observations processed | 57,000+ |
| Tests (CYNIC + B&C) | 2,045 |
| Commits (CYNIC + B&C) | 1,404 |
| Cloud dependency (AI engine) | 0% |

## Token Utility

**Access token:** hold $TALARIA to unlock judgment API tiers beyond free quota.
**Burn:** tournament entry burns $TALARIA — inter-community events = deflationary pressure at scale.

## Use of Funds

Funds sit in a futarchy-governed treasury. The team requests a **monthly allowance** ($6,050/month, market-approved) — not a salary. Unspent USDC stays in the treasury, not a private reserve.

- T. — CYNIC kernel, judgment API: $3,000/month allowance
- S. — Blitz & Chill, community: $3,000/month allowance
- Infrastructure (sovereign hardware, electricity): $50/month

At $6,050/month, 6 months of runway draws ~$36,300 — 73% of a $50K raise. The rest stays treasury-controlled, releasable only by market vote.

## Roadmap

- **M1–2:** Public judgment API · server-side PoH · first inter-community tournament
- **M3–4:** PoH on-chain mainnet · burn-as-a-service · checkers/dames · first external client
- **M5–6:** sBPF optimization · Go support · SDK · Season 1

## Why TALARIA?

Talaria — the winged sandals of Hermes. Speed, trust, and the messenger who never lies.

Built in the open:
- CYNIC: github.com/zeyxx/CYNIC
- Blitz & Chill: github.com/Ragnar-no-sleep/blitz-and-chill
```

---

## Étape 4 — Paramètres de la raise

Ce qu'on saisit réellement (le reste est imposé par le protocole MetaDAO) :

| Champ | Valeur |
|-------|--------|
| **Raise goal** | $50,000 USDC |
| **ICO duration** | 7 days |

Dérivés (non saisis — fixés par le protocole) :

| Dérivé | Valeur | D'où |
|--------|--------|------|
| Tokens ICO | 10,000,000 | standard MetaDAO |
| Prix ICO | $0.005 / $TALARIA | $50,000 ÷ 10,000,000 |
| LP auto-seeded | 2,900,000 | apparié à 20% de l'USDC levé |
| Float au TGE | 12,900,000 | ICO + LP (le team package est verrouillé) |

> ⚠️ On ne saisit PAS le supply total ni le FDV. Le FDV émerge du marché (raise discrétionnaire MetaDAO : on peut capper et rembourser le surplus).

---

## Étape 5 — Monthly spending limit (allowance)

C'est le **plafond d'allowance mensuelle** soumis au vote du marché — pas un salaire garanti, pas un budget qu'on contrôle directement.

| Champ | Valeur |
|-------|--------|
| **Monthly spending limit** | $6,050 |

### Détail de l'allowance demandée

```
Allowance mensuelle: $6,050/mois (plafond, votable)
- T. (CYNIC kernel, API)         : $3,000  ← ⚠️ à justifier (voir note)
- S. (B&C, community)            : $3,000  ← ⚠️ à justifier (voir note)
- Infrastructure (élec ~180 kWh/mois @ €0.25) : $50  (mesuré ≈ €45 ≈ $50)

Sur 6 mois: ~$36,300 tirés (73% d'un raise $50K).
Le reste NON dépensé reste dans la treasury futarchy (pas une "réserve" privée).
```

> ⚠️ **Le $3,000/personne n'est PAS dérivé** — c'est un montant rond posé, aucune base documentée. À justifier avant submit :
> - coût de la vie réel (Paris) ? full-time vs stage Caplogy en parallèle ?
> - comparable Futardio (ex: Umbra $34K/mois — on est 5× en dessous = "lean", défendable)
> - **compatibilité stage Caplogy** : toucher une allowance Talaria en plus du stage rémunéré ?
> L'allowance étant votée par le marché, un montant injustifié peut être rejeté.

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

## Tokenomics (modèle MetaDAO — imposé par le protocole)

Le supply n'est pas un champ : il découle des règles MetaDAO. On contrôle uniquement le raise, la durée, et si on prend un team package.

**Liquide au launch (fixé par le protocole) :**

| Bucket | Tokens | Unlock |
|--------|--------|--------|
| ICO (public raise) | 10,000,000 | 100% liquide au TGE |
| LP pool (auto-seeded) | 2,900,000 | seedé à la clôture (apparié à 20% de l'USDC) |

**Team performance package (optionnel, price-based, verrouillé) :**

| Titulaire | Tokens | Unlock |
|-----------|--------|--------|
| T. | 3,600,000 | 5 tranches à 2× / 4× / 8× / 16× / 32× prix ICO · cliff 18 mois |
| S. | 3,600,000 | 5 tranches à 2× / 4× / 8× / 16× / 32× prix ICO · cliff 18 mois |

> Plafond protocole : team package ≤ 12,900,000 (50% du supply initial). Aucun token team au TGE — déblocage uniquement sur performance prix soutenue (TWAP 3 mois).
> ⚠️ **DÉCISION EN ATTENTE (appel S.) :** prendre le team package ou non, et le montant exact. Les 3,6M + 3,6M ci-dessus sont la structure envisagée, pas figée.

**Trésorerie & fonds :** pas de bucket "treasury tokens" — l'USDC levé va dans une treasury gouvernée par futarchy. Les fondateurs tirent une allowance mensuelle votée par le marché (≠ versement direct).

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
- [x] Tokenomics ancrée au modèle MetaDAO réel (supply imposé, pas saisi)
- [ ] Décision team package (oui/non + montant) ← **avec S.**
- [ ] Adresse wallet S. (étape 6) ← **BLOQUEUR**
- [ ] 3ème adresse Solana (étape 6) ← **BLOQUEUR**
- [ ] Cayman entity MetaLeX (étape 7, en live)

---

*Mis à jour le 2026-05-31 — en attente wallets S. + backup*
