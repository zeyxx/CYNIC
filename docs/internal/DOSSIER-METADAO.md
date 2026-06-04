# Dossier MetaDAO / Talaria — Suivi central

> Tête de pont unique du dossier submission Futardio + stage. Mise à jour : 2026-05-31.
> Ce fichier = état d'avancement + décisions. Le contenu vit dans les autres fichiers `docs/internal/`.
> ⚠️ Ne JAMAIS mettre ici de PII / templates juridiques (cf. Exhibit A, hors repo).

---

## Chronos

- **Deadline submission Futardio :** 2026-06-02 (J-2 au 2026-05-31)
- **Stage Caplogy / Paris Ynov Campus :** 1 juin → 17 juillet 2026 (chevauche la submission)
- **Appel avec S. :** ce soir (2026-05-31) — décisions bloquantes ci-dessous

---

## Décisions — état

| Décision | Statut | Note |
|----------|--------|------|
| **Dox public ou pseudonyme** | ⏳ ce soir avec S. | A (identité→MetaDAO) quasi-obligatoire au CIIA ; B (site/Twitter) optionnel ; C (lien stage↔token) = risque le + concret. Levier propre : launch après le 17/07. |
| **Team package MetaDAO** (price-based) | ⏳ ce soir avec S. | Optionnel. Si pris : débloque seulement à 2×/4×/8×/16×/32× prix ICO, cliff 18 mois. Seul levier tokenomics réellement contrôlable. Champ probablement dans les 7 steps après connexion wallet. |
| **Fichier Exhibit A de S.** | ⏳ ce soir avec S. | À réviser ensemble. |
| **Niveau de retrait Exhibit A du repo** | ⏳ après décision dox | Niveau 1 (untrack, simple) vs Niveau 2 (rewrite histo + force-push). `.gitignore` déjà préparé (non commité). |

---

## Bloqueurs durs (submission)

- [ ] **Wallet Solana de S.** (treasury #2) — manquant
- [ ] **3ème wallet Solana** (backup / cold) pour multisig 2/3 — manquant
- [x] Wallet T. : `dcW5uy7wKdKFxkhyBfPv3MyvrCkDcv1rWucoat13KH4`
- [x] ETH wallet (Cayman) : `0xfD0759E929447c53143Df13278d822BE12dF9670`
- [x] Website + ToS live (vercel)
- [x] Token image + banner live
- Cayman entity : se crée en live pendant le submit via MetaLeX (~5 min, pas bloquant avant)

---

## Modèle Futardio/MetaDAO — ancré (vérifié 2026-05-31)

**Le formulaire `/create` ne demande AUCUNE tokenomics.** Prérequis réels (after "Connect Wallet", 0.5 SOL non-refundable) :
1. Project/token name + ticker  2. Token image ≥400×400  3. Tagline + description (markdown)
4. **Raise goal $10K–$2M USDC**  5. **ICO duration 1h–7j**  6. **Monthly spending limit**
7. **3+ wallets Solana**  8. **1 wallet ETH** (Cayman)  9. Website + ToS

**Supply/FDV/buckets = NON saisis → imposés par le protocole MetaDAO :**
- 10M tokens ICO (100% liquide au TGE, distribués pro-rata)
- ~2.9M LP auto-seeded (apparié à 20% de l'USDC levé)
- Team package optionnel : **jusqu'à 12.9M** (max 50% supply), price-based 2×→32×, cliff ≥18 mois, TWAP 3 mois
- **Pas de bucket "treasury tokens"** : la treasury = l'USDC levé, gouverné par futarchy
- Fonds NON versés aux fondateurs : **allowance mensuelle votée** par décision de marché
- Source : docs.metadao.fi/how-launches-work/sale + cas Ranger

**Conséquence :** la table "tokenomics 18M/21M + buckets %" des docs est À SUPPRIMER (faux problème ; on ne fixe pas le supply). On fixe : raise ($50k), duration (7j), monthly limit ($6,050).

---

## Métriques — mesurées (kernel live + repos, 2026-05-31)

| Métrique | CYNIC | B&C | Total |
|----------|-------|-----|-------|
| Verdicts | 2,195 (kernel `/health`) | — | **2,195** |
| Observations ingérées | 57,703 | — | **57,703** |
| Commits | 1,016 | 388 | **1,404** |
| Tests | 1,093 (972 Rust + 121 Py) | 952 | **2,045** |
| Crystals | 0 (`ever_crystallized: false`) | — | ⚠️ ne pas afficher |

→ Les chiffres "1,400+ commits" et "2,000+ tests" du doc combiné étaient EXACTS (CYNIC+B&C).
→ Verdicts à corriger : doc dit 1,874, réel = 2,195. Landing dit 50K obs, réel 57,703.

---

## Index des fichiers `docs/internal/`

**Submission (vivants) :**
- `futardio-7-steps.md` — plan des 7 étapes du formulaire (checklist bloqueurs en bas)
- `futardio-description-final.md` — description projet (markdown étape 3)
- `talaria-landing.html` — site (stats déjà alignées au réel mesuré)
- `talaria-tos.html` / `futardio-tos-draft.md` — ToS
- `talaria-token.svg` — token image

**Juridique (HORS repo — à sortir) :**
- `metalex-exhibit-a-T.md`, `metalex-exhibit-a-S.md` — Exhibit A CIIA (PII : Caplogy, Ynov, dates, handles)

**Archive / périmés (candidats nettoyage — 5+ versions de pitch) :**
- `futardio-pitch-unified-v6.md`, `-CYNIC-v3`, `-BC-v3/v4/v5`, `-draft-2026-05-28`
- `futardio-project-analysis-2026-05-28.md`, `discussion-S-structure-2026-05-28.md`
- `futardio-dm-0xMista-draft.md`, `futardio-website-plan.md`
- `research-solana-program-frameworks-2026-05-29.md`, `vision-agentic-economy-2026-05-29.md`

---

## Prochaines actions

1. **Ce soir avec S.** : trancher dox + team package + réviser Exhibit A de S.
2. Après dox : exécuter retrait Exhibit A du repo (niveau 1 ou 2)
3. Réécrire section "Tokenomics" des 2 docs → modèle MetaDAO réel (supprimer buckets inventés)
4. Corriger métriques : verdicts 1,874→2,195, obs →57,703
5. Obtenir wallets S. + backup (bloqueurs submission)
6. Archiver les pitches périmés (réduire le bruit des 20 fichiers)
