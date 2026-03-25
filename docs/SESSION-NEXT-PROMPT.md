# Session: CYNIC Epistemic Foundations — Industrial Engineering

## Identité

CYNIC est un **système immunitaire épistémique** — des validateurs IA indépendants
(Dogs) atteignant un consensus sous doute mathématique, φ-borné à 61.8%.

Vision long terme : AGI émergente fractale. Le même protocole (évaluer → voter →
cristalliser → apprendre) à toute échelle — entre Dogs, entre kernels, entre fédérations.
Des instances autonomes formant des clusters via un protocole natif (k-Net, IPv6).

## État du système

**Version :** v0.7.3 deployed, 260 tests, 5/5 Dogs sovereign.
**Findings :** 90 total (67 audit industriel + 23 stress test). 30 fixés. 60 ouverts.
**Kernel :** Rust, hexagonal (ports/adapters), axum REST + rmcp MCP stdio.
**Infra :** Ubuntu (CPU llama-server) + <GPU_NODE> (GPU Ollama) via Tailscale mesh.
**Storage :** SurrealDB 3.x via HTTP API (pas le SDK Rust — compilation hang #6954).

## Problèmes fondamentaux (cristallisés, vérifiés contre le code)

### 3 Root Causes Profondes

**RC-DEEP-A : Les invariants épistémiques sont dans l'application, pas le domaine.**
Le pipeline applique le gate, mais `POST /crystal` + `POST /crystal/{id}/observe` × 21
contourne tout le consensus Dog. 26 API calls = crystal empoisonné dans tous les prompts Dog.
Équivalent OS : un programme userspace qui écrit directement en mémoire kernel.

**RC-DEEP-B : L'état dégradé est invisible.**
Un verdict 1-Dog et un verdict 5-Dog sont structurellement identiques.
Quand les Dogs timeout, le système rapporte "healthy" avec `anomaly_detected: false`.

**RC-DEEP-D : Le cache sert deux objectifs contradictoires.**
VerdictCache clé = cosine(embedding) seul. Pas de domain, pas de dogs_filter.
Un verdict chess est servi pour un request trading si l'embedding est similaire.

### 5 Chaînes d'attaque (tracées file:line dans le code)

1. **Crystal poisoning → verdict corruption** (CRITICAL) — 26 API calls, permanent
2. **Observation → session summary → prompt injection** (HIGH) — 9 steps, zero sanitization
3. **Single-dog → crystal loop → consensus collapse** (HIGH) — heuristic becomes "consensus"
4. **Cache cross-domain contamination** (MEDIUM) — chess scores in trading crystals
5. **Rate limit bypass → amplified injection** (CRITICAL) — X-Forwarded-For spoofing

### Invariants épistémiques (dérivés de l'analyse)

> **I1 :** Tout contenu qui influence des jugements futurs doit avoir été jugé lui-même.
> **I2 :** Aucun agent seul ne peut modifier unilatéralement l'état épistémique.
> **I3 :** Le nombre de validateurs indépendants est toujours visible et minimum garanti.

## Méthodologie — Ingénierie Industrielle

Cette session suit le cycle V adapté : requirements → recherche → expérimentation →
falsification → design. Pas de code en production. Pas de design avant les résultats.

### Principes

1. **Evidence-based.** Chaque décision de design doit citer une source vérifiable
   (paper, code source, benchmark, expérience locale). "Je pense que" n'est pas une source.

2. **Falsifiable.** Chaque hypothèse doit avoir un test qui peut l'invalider.
   Si tu ne peux pas décrire ce qui l'invaliderait, ce n'est pas une hypothèse.

3. **Comparative.** Pour chaque choix architectural, au moins 2 alternatives analysées
   avec tradeoffs mesurés. Pas de "c'est la bonne approche" sans alternatives évaluées.

4. **Compound.** Chaque recherche est choisie parce qu'elle alimente une décision en aval.
   Pas de recherche isolée. La carte research → experiment → design est un DAG.

5. **Honest.** Compter les findings réellement fixés (30/90). Ne pas confondre
   "diagnostiqué" avec "corrigé". Ne pas confondre "recherché" avec "validé".

### Workflow par unité de travail

```
QUESTION (falsifiable, une phrase)
  → HYPOTHÈSE (ce qu'on s'attend à trouver)
  → RECHERCHE (sources multiples, recoupées)
    → Code source des projets (pas juste les articles/blogs)
    → Papers académiques quand applicable (BFT, CRDT, consensus)
    → Benchmarks publiés + benchmarks locaux
    → Pratiques des projets dans les GitHub stars du user
  → ANALYSE COMPARATIVE (2+ approches, tradeoffs mesurés)
  → CHALLENGE (quel argument invalide la conclusion ?)
  → DÉCISION (adopt/build/skip + confidence φ-bornée)
  → EXPÉRIENCE si nécessaire (branche dédiée, hypothèse falsifiable)
```

## Recherches requises

### R1 : Quorum certificates (BFT/Stellar)
**Question :** Comment structurer un certificat de consensus vérifiable par un nœud distant ?
**Hypothèse :** Un struct avec les scores individuels + voter IDs + hash du stimulus suffit.
**Sources :** stellar-core source, SCP whitepaper (Mazières), pBFT paper (Castro & Liskov),
tikv/raft-rs quorum certificate impl.
**Alimente :** Design de `EpistemicEvidence`, décision typestate vs runtime (T4).

### R2 : Crystal comme CRDT
**Question :** Quelle structure CRDT permet de merger des crystals entre nœuds sans conflit ?
**Hypothèse :** Un PN-Counter (observations_up, observations_down) + LWW pour le contenu.
**Sources :** rust-crdt crate source, Automerge Rust, Datacake framework,
CRDT survey paper (Shapiro et al. 2011).
**Alimente :** Design du crystal merge, décision storage distribué.

### R3 : Prompt injection structural defense
**Question :** Les délimiteurs réduisent-ils le taux d'injection sur des modèles <12B ?
**Hypothèse :** La sandwich technique réduit l'injection mais ne l'élimine pas sur les petits modèles.
**Sources :** OWASP LLM cheatsheet, PromptArmor (Shi et al. 2025), tests locaux sur Qwen/Gemma.
**Alimente :** Design de la chaîne summarizer → Dog prompt, fix Chain 2.

### R4 : Inference engine migration path
**Question :** Quel engine d'inférence remplace Ollama sans friction pour CYNIC ?
**Hypothèse :** vLLM pour GPU (<GPU_NODE>), llama-server reste pour CPU (Ubuntu).
**Sources :** vLLM 2026 benchmarks, mistral.rs benchmarks, TurboQuant timeline
(llama.cpp issues, vLLM releases), Hermes Agent #523 (local model setup).
**Alimente :** Décision infra, per-node engine selection.

### R5 : Protocol interoperability (A2A/ANP/MCP 2026)
**Question :** Comment CYNIC expose ses verdicts/crystals à d'autres agents sans couplage protocole ?
**Hypothèse :** A2A pour kernel↔kernel, MCP pour agent↔kernel, ANP pour discovery.
**Sources :** A2A spec (Google/AAIF), ANP whitepaper (arxiv 2508.00007), MCP 2026 roadmap,
AAIF governance docs.
**Alimente :** Design SecurityGate (identité de nœud), EpistemicEvidence (format réseau).

### R6 : Rust industrial patterns pour systèmes distribués
**Question :** Comment les projets Rust production gèrent les invariants dans les systèmes distribués ?
**Hypothèse :** Types pour la structure, runtime pour le protocole, simulation pour la preuve.
**Sources :** SquirrelFS (USENIX OSDI 2024, typestate pour filesystem), TiKV (raft-rs),
Datacake (CRDT framework Rust), s2 (deterministic simulation testing for async Rust).
**Alimente :** Choix architectural global (compile-time vs runtime vs simulation layers).

## Expérimentations (branches, PAS main)

Chaque expérience a : une hypothèse falsifiable, une branche dédiée, un critère PASS/FAIL mesurable.
Le code ne merge PAS dans main. Il valide ou invalide une hypothèse. C'est du jetable scientifique.

| # | Hypothèse | Critère PASS/FAIL | Branche |
|---|---|---|---|
| E1 | Crystal representable comme CRDT (merge associatif/commutatif/idempotent) | 3 propriétés prouvées par proptest (commutatif: merge(a,b)==merge(b,a), associatif, idempotent) | `exp/crdt-crystal` |
| E2 | EpistemicEvidence sérialisable + vérifiable cross-process | serde round-trip + verify() retourne true sur evidence valide, false sur evidence trafiquée | `exp/epistemic-evidence` |
| E3 | VerdictCache clé composite ne dégrade pas la perf | benchmark: lookup latency < 2x baseline avec 1000 entrées | `exp/cache-domain-key` |
| E4 | ConnectInfo extrait l'IP Tailscale réelle | curl depuis un peer Tailscale → IP affichée = IP Tailscale du peer, pas 127.0.0.1 | `exp/real-ip` |
| E5 | Delimiter sandwich réduit injection sur Qwen 3.5/Gemma 3 | 10 prompts d'injection, comparer taux de succès avec/sans délimiteurs | `exp/prompt-isolation` |

## Design spec (APRÈS les résultats)

Le spec ne se rédige PAS avant que R1-R6 et E1-E5 aient des résultats.
Il émerge des truths cristallisées. Il couvre :

1. **EpistemicEvidence** — certificat de consensus (structure, vérification, sérialisation)
2. **Crystal protocol** — CRDT merge, promotion gate, immunité anti-poisoning
3. **VerdictCache** — clé composite, isolation domain, TTL
4. **SecurityGate** — identité de nœud, capabilities, protocol-agnostic

Le spec est rédigé avec `/cynic-skills:engineering-stack-design` et cristallisé
avec `/cynic-skills:crystallize-truth`. Il contient :
- Types Rust (signatures, pas implémentation)
- Invariants falsifiables (tests attendus)
- Pseudocode pour les algorithmes complexes (quorum calc, CRDT merge)
- Tradeoffs documentés avec sources
- Carte de traçabilité : chaque décision → quelle recherche/expérience la supporte

## Documents de référence

| Document | Contenu |
|---|---|
| `docs/CYNIC-DEEP-AUDIT-2026-03-26.md` | 90 findings, 5 chaînes, 6 patterns, 3 RC profondes, recherche 2026, infra |
| `docs/SESSION-2026-03-25-RESEARCH.md` | Designs concrets par finding, roadmap 5 waves, 792 lignes |
| `docs/SESSION-2026-03-25-STRESS-TEST.md` | 23 stress test findings avec reproductions |
| `docs/CYNIC-INDUSTRIAL-AUDIT.md` | 67 findings originaux avec appendices A-E |
| `CLAUDE.md` | 33 rules, axiomes, workflow, ownership zones |
| `/tmp/CYNIC-legacy/` | GossipProtocol, consensus engine, singularity vision |
| `docs/CYNIC-ARCHITECTURE-TRUTHS.md` | Truths architecturales cristallisées |
| `docs/CYNIC-CRYSTALLIZED-TRUTH.md` | Truths cognitives cristallisées |

## Anti-patterns de cette session (ne pas reproduire)

1. **Brainstorming skill pour du protocol design** — c'est un workflow produit, pas scientifique
2. **Proposer des architectures avant de comprendre la vision** — 3 designs jetés car trop mécanistes
3. **TDD ceremony sur des one-liners** — le test était plus complexe que le fix
4. **Designer avant de chercher** — EpistemicEvidence proposé avant de lire comment Stellar/pBFT font
5. **Demander au user ce que la recherche peut répondre** — "est-ce que ça te semble juste ?" alors que la réponse est dans un paper
6. **Ralph loop sans exit criteria** — 83 itérations, 77 vides
7. **Compter les RCs au lieu des findings** — "7/8 RCs" cachait "30/90 findings"

## Succès de la session = ces livrables

- [ ] 6 recherches avec findings falsifiables et sources vérifiables
- [ ] 5 expériences avec résultat PASS/FAIL + données mesurées
- [ ] Truths cristallisées des résultats (crystallize-truth, layer 1-2-3)
- [ ] Design spec émerge des expériences, pas de la théorie
- [ ] Chaque décision du spec trace vers une recherche ou une expérience
- [ ] `git status --short` = 0 modified files en fin de session (Rule #30)
- [ ] Distillation complète (distill skill) en fin de session
