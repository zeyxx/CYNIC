<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Pitch Unifié Futardio — v6

**CYNIC × B&C — Infrastructure Souveraine de Jugement**

---

## Le Problème

50 000 nouveaux tokens par jour sur Solana. 98% sont des arnaques.

Les outils existants (DexScreener, BirdEye, RugCheck) montrent des chiffres. Personne ne te dit **ce qu'ils veulent dire**. Et personne ne plafonne sa propre confiance.

L'explosion des agents IA aggrave le problème : qui juge les jugeurs ?

---

## La Solution : CYNIC

**Moteur de jugement souverain, pas un dashboard.**

### Architecture

```
Stimulus → [3 Dogs indépendants] → Consensus φ-bounded → Verdict
                                         ↓
                              Crystal Coherence Machine
                              (mémoire inter-session)
```

- **3 modèles IA** évaluent indépendamment sur 6 axiomes (Fidelity, Phi, Verify, Culture, Burn, Sovereignty)
- **Consensus phi-bounded** : confiance plafonnée à φ⁻¹ = 61.8%
- **EPOCHÉ** : suspension pyrrhoniste quand les Dogs sont en désaccord
- Verdicts : HOWL (fiable) → BARK (danger) → EPOCHÉ (indécidable)

### En Production

| Métrique | Valeur |
|----------|--------|
| Verdicts rendus | 20 000+ |
| Tests | 1 873 (340 Rust + 1533 Python) |
| Lignes de code | 82 000+ |
| Langages | 3 (Rust, Python, TypeScript) |
| Uptime | 99.7% |
| Cloud | **0%** — hardware propre |

---

## La Stack Complète

### CYNIC Kernel (Rust)
Moteur de jugement. API REST + MCP. Verdicts, cristaux, mémoire.

### KAIROS (Python)
Data pipeline. Hyperliquid, Binance, Phoenix. Candles, funding, OI. Organism d'analyse continue.

### Hermes (Python + Chrome)
Intelligence sociale. Capture passive X/Twitter (mitmdump). Navigation organique UCB1. Extraction Gemini. Dataset JSONL → kernel /observe.

### KYON (Rust)
Askesis — suivi d'activité et conscience de soi du système.

---

## L'Équipe

| Membre | Rôle | Heritage |
|--------|------|----------|
| **T.** | Kernel CYNIC (Rust), architecture, vision | CYNIC, GASdf |
| **S.** | B&C (TypeScript), intégration communauté | B&C |
| **G.** | ASDelegate, HolDex, CultScreener | HolDex (766 commits, 50/50 avec T.) |

### Track Record

- **HolDex** : 766 commits, JavaScript — monitoring on-chain temps réel (collaboration T.+G.)
- **CYNIC ecosystem** : 3 000+ commits, 3 repos majeurs
- **GASdf** : communauté $ASDFASDFA active
- **CultScreener** : détection de cult tokens (G.)
- **ASDForecast** : prédiction de marché (G.)

---

## Modèle Économique

### Phase 1 — API (court terme)
- Verdict-as-a-Service : facturation par jugement
- Tiers : Free (10/jour) → Pro (1000/jour) → Enterprise (illimité)
- Cible : DEX, wallets, bots de trading

### Phase 2 — On-chain (moyen terme)
- Programme Solana (Pinocchio/Native) pour settlement des verdicts
- Réseau de Dogs décentralisé
- Token de gouvernance pour la calibration des axiomes

### Phase 3 — Agentic Economy (long terme)
- Infrastructure de jugement pour agents IA autonomes
- "Qui juge les agents ?" — CYNIC est la réponse
- MCP (Model Context Protocol) server pour intégration native

---

## Différenciateurs

| | CYNIC | RugCheck | GoPlus |
|---|---|---|---|
| Jugement multi-modèle | ✅ 3 Dogs | ❌ Heuristique | ❌ Règles |
| Confiance plafonnée | ✅ φ⁻¹ | ❌ 0-100% | ❌ Score absolu |
| Souveraineté | ✅ Zero cloud | ❌ AWS | ❌ Cloud |
| Mémoire | ✅ Cristaux | ❌ Stateless | ❌ Stateless |
| Suspension | ✅ EPOCHÉ | ❌ Force verdict | ❌ Force verdict |
| Open source | ✅ | ❌ | ❌ |

---

## L'Ask

1. **Partenaires techniques** — protocoles Solana voulant intégrer le jugement
2. **Beta testeurs** — wallets, DEX, bots qui veulent filtrer les rugs
3. **Investisseurs** — pre-seed pour infrastructure et équipe
4. **BPI** — accompagnement innovation (SAS en structuration)

---

## Demo Live

```bash
# Juger un token
curl -X POST http://cynic:3030/judge \
  -H "Content-Type: application/json" \
  -d '{"content":"<MINT_ADDRESS>","domain":"token-analysis"}'

# Santé du système
curl http://cynic:3030/health

# Cristaux (mémoire)
curl http://cynic:3030/crystals?domain=token-analysis
```

---

## Contact

- GitHub : github.com/zeyxx/CYNIC
- API : cynic:3030 (Tailscale)
- Email : cynic@agentmail.to
- X : @CynicOracle

---

*"Sovereign infrastructure making the cost of lying visible through the geometry of calibrated doubt."*
