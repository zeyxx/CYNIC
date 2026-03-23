# CYNIC — Cahier des Charges par Version

*Créé 2026-03-22. Document vivant. Chaque gate est falsifiable.*

Ce document définit ce qui doit être **vrai** pour qu'une version soit déclarée.
Il ne prescrit pas de tâches — le chemin est organique, la destination est mesurable.

**Règle : ≤ 5 gates par version. Si tu en as besoin de plus, split la version.**

---

## Positionnement

CYNIC est un **système immunitaire épistémique** — des validateurs IA indépendants
atteignant un consensus sous doute mathématique.

### Ce qui est genuinely novel (aucun équivalent trouvé en production, mars 2026)

1. **Confiance φ-bornée** — plafond structurel à φ⁻¹ = 0.618, pas une policy
2. **Désaccord comme signal** — divergence > φ⁻² = anomalie, pas bruit
3. **Boucle de cristallisation** — juger → stocker → embarquer → mesurer Δ
4. **Dogs souverains** — inférence locale comme validateur de première classe
5. **6 axiomes comme dimensions** — scoring structuré, pas des métriques génériques

### Paysage concurrent (résumé)

| Projet | Stars | Chevauchement | Différence clé |
|--------|-------|---------------|----------------|
| llm-council (Karpathy) | ~16k | Multi-modèle peer review | Toy app, pas d'apprentissage, pas de bornes |
| MoA (Together AI) | ~3k | Sagesse des foules LLM | Séquentiel, pas de jugement, pas de mémoire |
| DeepEval | ~14k | Quality gate CI/CD | Outil dev, single judge, pas de consensus |
| LiteLLM | ~40k | Routage multi-provider | Plomberie infra, aucun jugement épistémique |
| OpenAI Evals | ~18k | Évaluation modèle | Benchmark offline, pas de runtime |
| Guardrails AI | ~7k | Validation output | Règles déterministes, pas de consensus |
| Prometheus-eval | ~1k | Rubrics LLM-as-judge | Single modèle, pas de confidence bornée |
| RouteLLM (LMSYS) | ~5k | Routage quality-aware | Coût/qualité, pas épistémique. Mort 2024. |

**Créneau vide :** aucun système en production ne combine consensus multi-modèle +
confiance mathématiquement bornée + apprentissage par cristallisation + inférence souveraine.

---

## v0.6 — État actuel (snapshot, pas spécifié rétroactivement)

**Ce qui marche :**
- 242 tests, 0 `#[ignore]`, 0 clippy warnings
- 5 Dogs configurés (deterministic, gemini, huggingface, sovereign, sovereign-ubuntu)
- 3-5 Dogs healthy selon disponibilité réseau/GPU
- Crystal loop techniquement validé (Δ = +0.02-0.04 sur /test-chess)
- Event Bus + SSE déployés (6 types d'événements)
- REST API avec auth Bearer, rate limiting 30/min, input validation
- MCP server avec 10 tools, tous fonctionnels (aucun stub)
- Architecture hexagonale : 43 fichiers, 9 modules, 0 `#[cfg]` en domain
- SurrealDB pour persistence (verdicts, crystals, observations, sessions)
- Liskov Substitution parfait (NullStorage/NullCoord/NullEmbedding câblés partout)
- Graceful degradation (DB down → NullStorage, Dog fail → circuit breaker, embed fail → skip)
- Timeouts sur chaque background `.await` (Rule #10 mécanique)

**Ce qui ne marche pas (diagnostiqué par audit architecture + organes) :**
- **`KernelEvent` défini dans `api::rest/`** — dépendance ascendante qui BLOQUE le MCP
  d'émettre des events (`event_tx: None` hardcodé). L'Event Bus est aveugle aux sessions MCP.
- **`serde_json::Value` dans 5 signatures de `StoragePort`** — le port domain est couplé
  à serde_json. Les observations n'ont pas de structs typées → impossible de les enrichir.
- **`Judge` possède `CircuitBreaker` (type infra)** — violation DIP, pas de trait en domain.
- **`main.rs` = 754 lignes, 10 `tokio::spawn` inline** — composition root saturée.
- **4x `expect()` dans 4 constructeurs** — même classe de bug, non catchée par clippy.
- **Observations = squelettes** (tool + file + status, pas d'intention/outcome).
- **Event Bus sans consommateur interne** — broadcast → SSE uniquement, aucun organe ne réagit.
- **Crystal provenance invisible** — verdict ne dit pas quels crystals ont été injectés.
- **Sens (observations) et mémoire (crystals) totalement séparés** — question ouverte
  nécessitant recherche industrielle (contamination RAG vs signal épistémique).

---

## v0.7 — "Le Socle Architectural" -- COMPLETE (2026-03-22)

**Thème :** Nettoyer et connecter l'architecture pour que les organes communiquent.
Pas de nouvelles features — fixer le squelette pour que les muscles puissent se greffer.

Les violations architecturales (DIP, ISP, SRP) CAUSENT les problèmes d'organes :
`KernelEvent` dans REST → MCP aveugle → Bus mort → Provenance impossible.
Fixer l'architecture débloque tout en cascade.

**Status:** All 5 gates verified. 242 tests, 0 clippy warnings. `make check` green.

### Gate 1 : KernelEvent dans domain/ — 0 dépendance ascendante dans pipeline

Le seed le plus compound. `KernelEvent` appartient au domain (c'est un événement métier),
pas au transport REST. Tant qu'il est dans `api::rest/`, le MCP ne peut pas émettre d'events.

```bash
# Vérification :
grep 'use crate::api::rest' cynic-kernel/src/pipeline.rs  # 0 résultats
grep 'KernelEvent' cynic-kernel/src/domain/                # ≥ 1 résultat
```

**Débloque :** Gate 2 (MCP events), Gate 3 (provenance), futur Bus interne (v0.8).

### Gate 2 : MCP émet des events — 0 event_tx: None

Une fois KernelEvent en domain/, le MCP peut recevoir le broadcast sender.
Tout appel `cynic_judge` via MCP doit émettre `VerdictIssued` sur le bus.

```bash
# Vérification :
grep 'event_tx: None' cynic-kernel/src/api/mcp/  # 0 résultats
# + test fonctionnel : appel MCP judge → event visible sur /events SSE
```

**Débloque :** Observabilité multi-transport, sessions MCP visibles, monitoring unifié.

### Gate 3 : StoragePort sans serde_json — types domain pour observations

Remplacer les 5 `serde_json::Value` dans `StoragePort` par des structs typées en domain/.
Les observations deviennent des objets domain enrichissables (intent, outcome, skill).

```bash
# Vérification :
grep 'serde_json::Value' cynic-kernel/src/domain/storage.rs  # 0 résultats
grep 'serde_json::Value' cynic-kernel/src/domain/            # 0 résultats
```

**Débloque :** Observations riches (v0.8), crystal provenance typée, session summaries structurées.

### Gate 4 : 0 expect() en production — deny(clippy::expect_used)

4 constructeurs paniquent si `reqwest::Client::builder().build()` échoue (TLS init,
seccomp sandbox). Même classe de bug dans 4 fichiers. Le gate mécanique : le compilateur
rejette tout `expect()` en code non-test.

```bash
# Vérification :
grep 'deny(clippy::expect_used)' cynic-kernel/src/lib.rs     # 1 résultat
grep '\.expect(' cynic-kernel/src/**/*.rs | grep -v '#\[cfg(test)\]' | grep -v 'tests/'  # 0
```

**Débloque :** Production safety mécanique (Rule #25), confiance en déploiement.

### Gate 5 : main.rs < 400 lignes — spawns extraits en modules

Chaque `tokio::spawn` avec business logic déplacé dans son module :
`spawn_remediation_watcher()`, `spawn_usage_flush()`, `spawn_ccm_aggregator()`, etc.
main.rs devient un pur fichier de câblage.

```bash
# Vérification :
wc -l < cynic-kernel/src/main.rs  # < 400
```

**Débloque :** Ajout de consumers Event Bus (v0.8), composition root lisible, onboarding.

### Graphe de dépendance

```
Gate 1 (KernelEvent → domain/) ───── LE SEED
     │
Gate 2 (MCP events) ──── nécessite Gate 1
     │                           Gate 4 (expect → Result) ─── indépendant
Gate 3 (StoragePort typé) ────── indépendant
     │                           Gate 5 (main.rs extract) ── indépendant
     ▼
v0.8 — Les organes communiquent, observations riches, crystal Δ mesurable
```

**Compound logic :** Gate 1 est le seed — elle débloque Gate 2 qui débloque la visibilité
multi-transport. Gates 3, 4, 5 sont parallélisables. TOUTES débloquent des features v0.8.

**Invariant :** `make check` passe avec ≥ 242 tests et 0 warnings à chaque gate.

---

## v0.8 — "L'Organisme Circule" (sketch)

**Thème :** Les organes communiquent — information riche qui circule entre sens, mémoire,
et système nerveux. Le socle v0.7 le rend possible, v0.8 le prouve.

| Gate | Vérifie | Débloqué par v0.7 |
|------|---------|-------------------|
| Observations riches (intent + outcome + skill) | ≥ 70% des obs ont `intent` non-vide | Gate 3 (types domain) |
| Crystal A/B Δ ≥ +0.02 | /test-chess with vs without crystals | Gate 2 (MCP events) + Gate 3 |
| Event Bus a ≥ 1 consumer interne | Background task qui réagit aux events | Gate 1 + Gate 5 |
| MCP tools retournent du contexte enrichi | cynic_health inclut events + crystal stats | Gate 2 (MCP events) |
| Session continuité | Session start affiche résumé session précédente | Gate 3 (types domain) |

**KPI source :** CRYSTAL-LOOP-KPIS.md §KPI 2, §KPI 3.
**Question ouverte :** Pont observations → crystals (E1). Recherche industrielle requise.

---

## v1.0 — "Souverain et Utile" (horizon)

**Thème :** CYNIC est indépendamment utile — quelqu'un peut le déployer et en tirer de la valeur.

| Gate | Vérification |
|------|-------------|
| Crystal Δ > 0.05 sur ≥ 3 domaines | Chess, code review, + 1 domaine externe |
| Sovereign ≥ 50% des évaluations | Logs : plus de la moitié des Dog calls sont locaux |
| API stable 30 jours | Aucun breaking change pendant 30 jours consécutifs |
| Docs complètes | README + API.md + AGENTS.md couvrent toutes les features déployées |
| Utilisateur externe | ≥ 1 personne non-auteur fait tourner une instance CYNIC |

---

## Principes du document

1. **Gates, pas tasks.** Ce document dit quand on est arrivé, pas comment y aller.
2. **Machine-vérifiable.** Chaque gate a une commande bash. Si tu ne peux pas scripter, c'est pas un gate.
3. **Les principes restent dans CLAUDE.md.** Ici c'est quantitatif uniquement.
4. **≤ 5 gates par version.** Plus = split.
5. **Les KPIs viennent de `docs/CRYSTAL-LOOP-KPIS.md`.** Ce document les wire aux versions.
6. **Organique mais orienté.** Chaque session devrait pouvoir répondre : "quel gate v0.7 j'avance ?"

---

## Références

- KPIs cristallisés : `docs/CRYSTAL-LOOP-KPIS.md`
- Architecture : `docs/CYNIC-ARCHITECTURE-TRUTHS.md`
- Convergence φ : `docs/CYNIC-PHI-CONVERGENCE.md`
- Roadmap infra : `docs/CYNIC-INFRASTRUCTURE-ROADMAP.md`
- Analyse novelty : mémoire `research_novelty_analysis.md`
- Constitution : `CLAUDE.md`
