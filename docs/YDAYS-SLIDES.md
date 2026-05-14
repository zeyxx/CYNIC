# Ydays 2026 — Diaporama (15 min)
## T. + S. | B2 Ynov

> Mix A+B : le parcours comme voyage, les problèmes réels comme destination.

---

## Slide 1 — Titre

# CYNIC, Blitz & Chill, GASdf
### Comment une communauté crypto est devenue un laboratoire d'infrastructure

T. & S. — Ydays 2025-2026

---

## Slide 2 — Le Point de Départ (1 min)

### Juillet 2025 : une communauté, une question

- Communauté $ASDFASDFA formée le **7 juillet 2025** sur Solana
- Septembre : premier code — un tracker de burns
- On ne connaissait rien à la blockchain. On voulait comprendre.

> "On n'a pas planifié de construire un système de jugement IA.
> On a suivi les questions, une par une."

**Les questions ont changé :**

```
"Que fait mon token ?"     → Observer    (sept 25)
"J'ai besoin d'outils"     → Construire  (déc 25)
"Comment rendre ça utile?" → Abstraire   (jan 26)
"C'est fiable ou pas ?"    → Juger       (fév 26)
"Sur MON hardware"         → Souveraineté (mars 26)
"Ça vit ensemble"          → Organisme   (avr-mai 26)
```

---

## Slide 3 — Observer & Construire (1 min)

### De spectateur à bâtisseur (sept → déc 2025)

**Septembre** : 1 repo. Un tracker de burns. Scripts basiques.
On apprend : JavaScript, APIs, qu'est-ce qu'une transaction Solana.

**Décembre** : **16 repos en un mois.**

| T. | S. |
|----|----|
| ASDForecast (prédiction) | ASDF-Ecosystem |
| asdf-validator (creator fees) | Discord bot |
| ASDev (plateforme lancement) | HolDex-App |
| GASdf (gasless transactions) | ASDF-Web |
| HolDex (alt. DexScreener) | |

La plupart de ces repos sont morts aujourd'hui. C'est normal.
**On a appris Node.js, Express, Jupiter, Redis, React, Solana RPC en un mois.**

> Quantité > qualité. C'est là qu'on apprend le plus vite.

---

## Slide 4 — Abstraire & Juger (1 min 30)

### La question change (jan → fév 2026)

**Janvier — de scripts à frameworks :**
- `solana-keychain` — framework de signing
- `gasdf-sdk` — publié sur npm
- **CYNIC-legacy** — premier essai de jugement par IA

> On ne code plus "pour nous". On code pour que d'autres puissent utiliser.

**Février — le saut qualitatif :**
- La question n'est plus *"comment construire"* mais *"est-ce que c'est bien ?"*
- **CYNIC renaît en Rust** (23 février) — réécriture complète
- JS → Rust. Scripts → Kernel. Opinion → Consensus.

**Le principe** : au lieu de faire confiance à UN modèle IA,
on fait débattre PLUSIEURS modèles indépendants.
La vérité sort du désaccord.

---

## Slide 5 — CYNIC : le système de jugement (2 min, T.)

### "La confiance ne se décrète pas — elle se mesure."

```
         Un token Solana entre
                  │
                  ▼
          ┌───────────────┐
          │  Domain Layer  │  ← Logique pure, zéro I/O
          │  (6 axiomes)   │
          └───────┬────────┘
                  │
     ┌────────────┼────────────┐
     ▼            ▼            ▼
 [Dog GPU]    [Dog CPU]    [Dog Déterministe]
 Qwen 3.5     Qwen 2.5    Heuristiques
 RTX 4060 Ti  AMD APU     In-kernel (<1ms)
     │            │            │
     └────────────┼────────────┘
                  │
          ┌───────▼────────┐
          │   Consensus    │
          │ Trimmed-mean   │
          │ φ-bounded      │  ← max 61.8%
          └───────┬────────┘
                  │
                  ▼
    HOWL / WAG / GROWL / BARK
   (fiable)              (danger)
```

**3 modèles IA indépendants** jugent chaque token sur 6 dimensions :
Fidélité, Structure, Vérifiabilité, Culture, Efficience, Souveraineté

**Confiance plafonnée à φ⁻¹ = 61.8%** — le système ne prétend jamais être certain.

**Crystal Coherence Machine** : le 50ème token est jugé avec la mémoire des 49 précédents. Le système apprend de ses propres verdicts.

49 000 lignes Rust · 340+ tests · 20 000+ verdicts produits

---

## Slide 6 — Souveraineté (mars 2026) (30 sec)

### "Tout doit tourner sur notre hardware"

En mars, on achète du matériel :

| Machine | CPU/GPU | Rôle |
|---------|---------|------|
| cynic-core | AMD APU | Kernel + Dog CPU |
| cynic-gpu | RTX 4060 Ti | Dog GPU (55 tok/s) |

- **Tailscale mesh** : réseau privé entre les machines
- **Zéro API cloud** pour l'inférence IA
- **systemd** : tout tourne en service, redémarre automatiquement

> Si OpenAI coupe l'accès demain, CYNIC continue de tourner.

---

## Slide 7 — Blitz & Chill : Proof-of-Humanity (2 min 30, S.)

### "The chess game you send to your friend who's never played"

**Le problème** : Comment prouver qu'un wallet appartient à un humain ?
Les CAPTCHAs se contournent. Les KYC centralisent.

**Notre réponse** : jouer aux échecs.

```
  Jouer une partie (B&C)
         │
         ▼
  6 signaux comportementaux
  (temps de décision, agressivité,
   variance, taux d'abandon,
   longueur de partie, répertoire)
         │
         ▼
  Classification en archétype
  (9 personnalités : Fantôme, Barbare,
   Sniper, Philosophe, Chirurgien...)
         │
         ▼
  Mint Personality Card
  (Soulbound NFT, Solana devnet)
```

**Pourquoi les échecs ?**
- Données comportementales riches et vérifiables
- Un bot joue différemment d'un humain — signature unique
- Jeu universel, accessible, pas besoin d'expérience
- Le résultat importe moins que la façon de jouer

**Stack** : Next.js 16, Socket.IO (Fly.io Paris), PostgreSQL (Neon), Stockfish WASM
29 475 lignes · 792+ tests · i18n FR/EN · Live sur Vercel

---

## Slide 8 — L'intégration : tout se connecte (1 min)

### Trois projets, un seul flux de confiance

```
     B&C                    CYNIC                 GASdf
  Proof-of-Humanity    Proof-of-Intelligence    Trust → Access
       │                     │                      │
       │    /mint-permit     │                      │
       └────────────────────►│     K-Score gate     │
            "est-il humain?" └─────────────────────►│
                                "ce token est-il     │
                                 fiable?"            │
                                                     ▼
                                          Transaction exécutée
                                          sans SOL requis
```

**B&C** prouve que tu es humain (comportement aux échecs)
**CYNIC** prouve que le token est fiable (consensus multi-modèle)
**GASdf** réduit la friction si la confiance est établie (gasless tx)

**GASdf** : tu payes les frais réseau avec n'importe quel token.
76.4% des frais sont burnés, 23.6% en trésorerie — ratios dérivés de φ.
SDK publié sur npm · 14 versions · 3 568 lignes JS

**Le fil conducteur** :
- **φ** (nombre d'or) — confiance bornée (CYNIC), économie (GASdf), scoring (B&C)
- **Souveraineté** — hardware propre, zéro dépendance cloud
- **Anti-sybil** — chaque couche vérifie quelque chose de différent

---

## Slide 9 — Démo Live (2-3 min)

### 1. B&C — La plateforme d'échecs (S.)

→ **blitz-and-chill-web.vercel.app**
- Montrer l'interface, lancer une partie contre Stockfish
- Montrer le profil de personnalité / archétype

### 2. CYNIC — Juger un token en live (T.)

```bash
curl -X POST $CYNIC_REST_ADDR/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"content":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"}'
```
→ Verdict (HOWL/WAG/GROWL/BARK) + confiance ≤ 0.618

### 3. CYNIC /health — L'organisme vivant (T.)

```bash
curl $CYNIC_REST_ADDR/health \
  -H "Authorization: Bearer $CYNIC_API_KEY" | jq .
```
→ 19 background tasks, Dogs actifs, status temps réel

---

## Slide 10 — Les problèmes réels (1 min 30)

### Pourquoi tout ça compte

**Problème 1 — Les scam tokens :**
- 50 000 nouveaux tokens Solana par jour
- 98% des tokens pump.fun sont des rug pulls
- Les outils existants (DexScreener, BullX) montrent des chiffres bruts
- **Personne ne te dit ce que les chiffres veulent dire**
→ CYNIC juge et donne un verdict, pas un tableau de bord

**Problème 2 — Les bots :**
- Sybil attacks : un humain crée 1000 wallets
- Les CAPTCHAs se cassent, les KYC centralisent
→ B&C prouve l'humanité par le comportement, pas par un formulaire

**Problème 3 — La dépendance cloud :**
- Si OpenAI/Anthropic coupe l'accès, ton système meurt
- Si Helius change ses tarifs, ta data disparaît
→ 3 modèles IA sur hardware propre, zéro API d'inférence cloud

---

## Slide 11 — Les chiffres (30 sec)

### 11 mois, 2 personnes

| | B&C (S.) | CYNIC (T.) | GASdf (T.) | **Total** |
|---|---|---|---|---|
| Lignes de code | 29 475 | 49 204 | 3 568 | **82 247** |
| Tests | 792+ | 340+ | 741 | **1 873+** |
| Versions | — | 10 | 14 | **24** |
| Langage | TypeScript | Rust | JavaScript | **3** |

**41 repos** créés sur 2 GitHub depuis septembre 2025
**3 services** déployés (Vercel, Fly.io, Tailscale mesh)
**20 000+** verdicts produits par CYNIC
**2 personnes** + agents IA comme outils de développement (Claude Code, Gemini CLI)

---

## Slide 12 — Ce qu'on a appris (1 min)

### L'arc d'apprentissage

```
JS scripts → Node.js services → npm SDK → Rust kernel → Organisme IA
```

1. **Commencer par la curiosité** — un tracker de burns, pas un business plan
2. **Accepter que la plupart des projets meurent** — 16 repos en décembre, la plupart morts
3. **Abstraire quand le pattern se répète** — SDK et frameworks dès janvier
4. **Changer d'outil quand le problème l'exige** — JS → Rust pour le type safety
5. **La souveraineté a un coût** — maintenir son hardware est plus dur que payer un cloud. Mais on ne dépend de personne.

### Ce qu'on ferait différemment

- Moins de repos, plus de profondeur plus tôt
- Tester sur des vrais utilisateurs avant de construire l'infrastructure
- Écrire les tests avant le code (on a appris ça tard)

---

## Slide 13 — Questions

# Questions ?

**Code** : github.com/zeyxx · github.com/ragnar-no-sleep
**Live** : blitz-and-chill-web.vercel.app · CYNIC kernel (Tailscale)

---

# NOTES POUR PRÉSENTATEUR

## Structure narrative

**Acte 1 — D'où on vient** (slides 1-3, ~2 min 30)
Le parcours : communauté → observer → construire → abstraire
*Message* : on a suivi les questions, pas un plan.

**Acte 2 — Ce qu'on a construit** (slides 4-7, ~6 min 30)
Les trois projets : CYNIC (juger), B&C (prouver), souveraineté
*Message* : chaque projet résout un problème précis.

**Acte 3 — Pourquoi ça compte** (slides 8-12, ~6 min)
L'intégration, la démo, les problèmes réels, les chiffres, les leçons
*Message* : la confiance ne se décrète pas, elle se mesure.

## Répartition T. / S.

| Slide | Qui | Durée |
|-------|-----|-------|
| 1-2 (Titre + Départ) | T. | 1 min |
| 3 (Observer + Construire) | T. + S. | 1 min |
| 4 (Abstraire + Juger) | T. | 1 min 30 |
| 5 (CYNIC) | T. | 2 min |
| 6 (Souveraineté) | T. | 30 sec |
| 7 (B&C) | **S.** | 2 min 30 |
| 8 (Intégration + GASdf) | T. | 1 min |
| 9 (Démo) | T. + S. | 2-3 min |
| 10 (Problèmes réels) | T. | 1 min 30 |
| 11 (Chiffres) | T. ou S. | 30 sec |
| 12 (Leçons) | T. | 1 min |
| 13 (Questions) | Ensemble | — |
| **Total** | | **~15 min** |

## Transitions clés (ce que tu DIS entre les slides)

**2→3** : "On a commencé par regarder. Puis on a eu besoin d'outils."
**3→4** : "Décembre c'était le chaos. En janvier, on a commencé à structurer."
**4→5** : "Et c'est là que CYNIC est né. Laissez-moi vous montrer comment ça marche."
**5→6** : "Mais un système de jugement qui dépend du cloud, c'est pas souverain."
**6→7** : "Et pendant ce temps, S. construisait quelque chose de complètement différent. S. ?"
**7→8** : "Merci S. Ce qui est intéressant c'est que ces projets se connectent."
**8→9** : "Plutôt que d'expliquer, on va vous montrer."
**9→10** : "Maintenant que vous avez vu ce que ça fait — pourquoi c'est important ?"
**10→11** : "En résumé, les chiffres."
**11→12** : "Et ce qu'on en retient."

## Checklist avant présentation

- [ ] `source ~/.cynic-env && bash docs/DEMO-SCRIPT.sh` passe
- [ ] B&C live (blitz-and-chill-web.vercel.app) — tester sur téléphone aussi
- [ ] Connexion internet à l'école testée
- [ ] **Fallback** : screenshots de /judge et /health prêts (si pas d'internet)
- [ ] S. briefé sur son timing (slide 7, 2 min 30)
- [ ] S. a testé B&C sur le réseau de l'école
- [ ] Terminal ouvert avec les commandes curl prêtes (copier-coller)

## Questions probables de l'intervenant

**"Pourquoi Rust ?"**
→ Type safety + performance. cargo clippy rejette les bugs avant le runtime. Zero unwrap autorisé = zero crash silencieux. On a commencé en JS, mais quand le problème est devenu "évaluer la fiabilité", on avait besoin d'un langage qui ne ment pas sur les types.

**"C'est quoi le nombre d'or (φ) là-dedans ?"**
→ φ⁻¹ = 0.618 = plafond de confiance. Le système ne dit jamais "sûr à 100%". C'est un choix épistémique : le doute est structurel, pas un bug. Aussi dans GASdf : 76.4% burn = 1 - 1/φ³.

**"Ça sert à quoi concrètement ?"**
→ Protéger les gens des scam tokens. 50K tokens/jour sur Solana, 98% sont des rugs. Les outils existants montrent des métriques brutes, CYNIC te dit ce que ça veut dire.

**"Comment les échecs prouvent l'humanité ?"**
→ Un bot joue différemment d'un humain. Temps de réflexion, variance, taux d'abandon = signature comportementale unique. On ne regarde pas QUI gagne, mais COMMENT tu joues.

**"Vous avez travaillé à combien ?"**
→ 2 personnes. Des agents IA (Claude Code, Gemini CLI) comme outils de développement — pas comme remplaçants. On écrit la logique, l'IA aide à itérer plus vite.

**"C'est déployé ?"**
→ Oui. CYNIC sur hardware physique (AMD + RTX 4060 Ti), B&C sur Vercel + Fly.io (Paris). 20 000+ verdicts produits. Tout est open-source.

**"Qu'est-ce qui manque ?"**
→ Honnêtement : des vrais utilisateurs. On a l'infrastructure, on a les tests, on a la théorie. Ce qui manque c'est la validation par l'usage réel. C'est la prochaine étape.

**"C'est pas juste un projet scolaire ?"**
→ Le code est en production. Le kernel tourne 24/7. Les verdicts sont réels. Les tests passent. C'est un projet scolaire qui a dépassé le cadre scolaire — et c'est ça qui est intéressant.

## Risque démo et fallback

Si le kernel ne répond pas demain :
1. Montrer les screenshots (préparer ce soir)
2. Montrer le code source dans VS Code (architecture hexagonale visible)
3. Lancer `cargo test --lib` en live (340+ tests, ~2s)

Si B&C ne charge pas :
1. Screenshots de l'interface
2. Montrer le repo GitHub (commits, tests)
