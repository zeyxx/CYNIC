<!-- lifecycle: living -->
# CYNIC — Cahier des Charges par Version

*Créé 2026-03-22. Révisé 2026-03-27 (metathinking + SRE + AI infra reviews). Document vivant.*

Ce document définit ce qui doit être **vrai** pour qu'une version soit déclarée.
Il ne prescrit pas de tâches — le chemin est organique, la destination est mesurable.
Les gates émergent aussi du travail — ce document évolue.

**Règle : ≤ 3 gates par version. Si tu en as besoin de plus, split la version.**

## North Star

CYNIC est un **organisme**, pas une application. Chaque version est un organisme complet
à son échelle — self-similarité fractale du nœud unique à la fédération.
Les organes sont interconnectés : la boucle crystal nourrit les Dog prompts, les events
connectent les organes, les observations deviennent connaissance, la connaissance aiguise
le jugement. Aucun organe n'est une île.

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

**Créneau vide :** aucun système en production ne combine consensus multi-modèle +
confiance mathématiquement bornée + apprentissage par cristallisation + inférence souveraine.

---

## v0.6 — COMPLETE (snapshot)

242 tests. 5 Dogs. Crystal loop validé (Δ=+0.02-0.04 chess). SSE event bus.
Architecture hexagonale. SurrealDB. Graceful degradation.

## v0.7 — "Le Socle Architectural" — COMPLETE (2026-03-22)

5 gates vérifiés. KernelEvent en domain. MCP émet des events. StoragePort typé.
0 expect() en production. main.rs < 400 lignes. 242 tests, 0 clippy warnings.

---

## v0.8 — "Fondation Prouvée" — IN PROGRESS

**Thème :** La fondation est mécaniquement vérifiée. Sécurité assurée. StoragePort
prouvé agnostique. Workflow aligné. Les organes ont des contrats testés.

**Réalité actuelle (2026-03-29) :** 338 tests (255 unit + 57 contract + 26 integration). 43/90 findings fixed.
28 méthodes StoragePort (was 30 — dead path removed). InMemory adapter passes all 13 contract tests.
Cargo.toml = 0.7.5, tag = v0.7.5 (faa3a13). `make check` green (build + test + clippy + 3 lint gates + audit).
BURN cleanup: CCM aggregator disabled (Rule 3), event consumer stripped (double-counting), dead endpoint removed.

| Gate | Vérification | État |
|------|-------------|------|
| G1: Security closure | 0 CRIT open. Tous HIGH soit FIXED soit accepted-by-design (rationale dans tracker). | **GREEN.** lint-security passes. RC1-1 = PARTIAL (rate limit, stdio trust by MCP spec). F14 = ACCEPTED (fundamental LLM limitation). |
| G2: StoragePort agnostic | InMemory adapter passe les mêmes contract tests que SurrealDB. ≥12 `fn contract_`. | **GREEN.** 13 contract tests, parameterized via macros. InMemory + SurrealDB pass identically. |
| G3: Workflow alignment | Cargo.toml = git tag = VERSION.md. State dumps supprimés. Doc lifecycle tags. | **GREEN.** v0.7.5 everywhere. Scientific Protocol in workflow.md. API.md lint-gated. |

### Contraintes résolues

- **RC1-1** : accepted by MCP spec design (stdio = process-level trust). Rate limited at 10/min judge, 30/min other.
- **InMemory adapter** : Full implementation, passes 13 contract tests identically to SurrealDB.
- **Contract tests** : 13 (target was 12). Parameterized `contract_*` macro.
- **State dumps** : Purged (session 2026-03-29-organic).

### Contraintes restantes

- **Doc SoC** : lifecycle tags not mechanically enforced yet.
- **Backup/restore** : Round-trip SurrealDB → non testé.

---

## v0.9 — "L'Organisme Apprend" — NOT STARTED

**Thème :** La boucle crystal devient multi-domaine. La connaissance entre dans le système.
Le moat compound. Les organes commencent à se connecter via l'event bus.

| Gate | Vérification | Séquence |
|------|-------------|----------|
| G1: φ-convergence | Score ≠ Confidence séparés. HOWL = 0.528. Thresholds φ-dérivés. Benchmark before/after. | **Premier** (change la sémantique) |
| G2: KAIROS domain | Crystal Δ > +0.02 trading. n≥100 stimuli. Oracle = P&L réalisé. | **Deuxième** (mesuré sur nouvelle sémantique) |
| G3: cynic_learn | MCP tool live. Quarantine state. Dog eval obligatoire. Rate limit/agent. | **Dernier** (gated sur RC1 full closure) |

### Contraintes

- **φ-convergence = migration complète** : QScore, CrystalState SQL, pipeline, REST schema (breaking), frontend.
- **Crystal Δ rigoureux** : n≥100, paired design, oracle externe.
- **cynic_learn poisoning** : quarantine (source=agent, confidence=0.01), Dog eval avant crystallisation.
- **Crystal decay** : TTL + contradiction-driven. Sans ça, crystals stale nuisent.
- **Event bus consumers** : SSE streams to dashboard. Internal consumer is liveness-only. CCM aggregator deferred (Phase 2 — connect observations to crystal pipeline).
- **Observability** : /metrics (crystal transitions, Dog latency, cache hit rates).

---

## v1.0 — "Souverain et Ouvert" — NOT STARTED

**Thème :** Quelqu'un d'autre peut déployer CYNIC et en tirer de la valeur.

| Gate | Vérification |
|------|-------------|
| G1: Dogs as Data | DOG.toml manifest. Ajout Dog sans recompile. Quorum dynamique. Config swap atomique. |
| G2: API stable + observable | /metrics live. 0 breaking change 30j. Feature freeze explicite. |
| G3: Contributor-ready | QUICKSTART.md. UI connectée (S.). API.md complète. Outsider déploie. |

### Contraintes

- **Hot-reload** : nouvelle config au PROCHAIN cycle de jugement, jamais mid-inflight.
- **Observabilité avant stabilité** : /metrics doit exister avant le clock 30j.
- **Fédération-ready** : hexagonal strict vérifié par lint-rules. Peut être déjà green.

---

## Post-v1 (Horizon — émergera du travail)

- Boot integrity + crash recovery (verify_integrity, dirty flag, mode dégradé)
- Federation (CRDT crystals, GossipProtocol)
- Embed agent (code quality overnight)
- Inference metabolism natif (remplace rtk)
- Memory bootstrap (80+ fichiers → Forming crystals)

---

## Principes du document

1. **Gates, pas tasks.** Ce document dit quand on est arrivé, pas comment y aller.
2. **Machine-vérifiable.** Chaque gate a une commande bash. Si tu ne peux pas scripter, c'est pas un gate.
3. **≤ 3 gates par version.** Plus = split.
4. **Organique.** Les gates émergent du travail. Ce document évolue.
5. **Fractal.** Chaque organe fonctionne à son échelle actuelle ET prépare l'échelle suivante.

---

## Références

- Architecture : `docs/reference/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`
- Convergence φ : `docs/identity/CYNIC-PHI-CONVERGENCE.md`
- Findings tracker : `docs/audit/CYNIC-FINDINGS-TRACKER.md`
- KPIs : `docs/reference/CRYSTAL-LOOP-KPIS.md`
