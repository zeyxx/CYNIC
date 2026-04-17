# CYNIC Inference Gap Map — Session Dédiée Scope

> *"Vous testez vos GATES plus rigoureusement que vos MODÈLES. Les gates protègent le code. Les modèles produisent la value."*
> — Hostile expert critique, 2026-04-17.

**Updated:** 2026-04-17 | **Source:** 5-agent audit + metathinking + live probes
**Falsifiable:** every gap is verified by absence of code/config. If the gap is filled, grep confirms.

---

## Foundations qui EXISTENT (observed — ne pas reconstruire)

| Composant | Lieu | Ce qu'il fait |
|---|---|---|
| InferenceProfile (4 profils) | `domain/chat.rs:18-39` | Scoring/Agent/Summary/Infer — intent per-call |
| effective_max_tokens() | `backends/openai.rs:270-278` | Thinking-aware: quand thinking=ON, utilise backend max_tokens au lieu du profil |
| should_disable() | `backends/openai.rs:262-264` | Thinking control local-only (cloud pas touché) |
| BackendConfig (14 champs) | `infra/config.rs:31-63` | max_tokens, temperature, disable_thinking, json_mode, timeout, context, cost |
| json_mode response format | `backends/openai.rs:152-157` | `{"type":"json_object"}` quand activé |
| Lenient JSON parser | `dogs/inference.rs:200+` | Fallback pour modèles avec duplicate keys |
| ChatPort trait | `domain/chat.rs` | `chat(system, user, profile, request_id)` — interface abstraite |
| Fleet probe (model identity) | `infra/probes/fleet.rs` | Détecte model mismatch config vs running (observé 2026-04-17) |

---

## Niveau 1 — Config gaps (0 code change, backends.toml seul)

| # | Gap | Backend(s) affecté(s) | Fix | Falsification |
|---|---|---|---|---|
| C1 | `disable_thinking` mal configuré | gemma-4-e4b-core (false → thinking active → timeout) | `disable_thinking = true` OU timeout adapté | Dog participe aux verdicts (voter_count inclut gemma) |
| C2 | `json_mode` absent | qwen-7b-hf, gemini-cli | `json_mode = true` | json_valid_rate augmente (mesurer avant/après) |
| C3 | `context_size` absent | qwen-7b-hf | `context_size = 32768` | Fleet probe rapporte la bonne valeur |
| C4 | `max_tokens` sous-dimensionné pour gemini-cli | gemini-cli (1024) | Vérifier axiom response length réelle, ajuster | Gemini ne tronque plus |
| C5 | Capabilities non-déclarées | Tous | Ajouter champs : `supports_thinking`, `supports_json_mode`, `supports_tool_use`, `tokenizer_family` | grep config shows per-backend capabilities |
| C6 | Latency class absente | Tous | `latency_class = "realtime" / "background"` | Routing possible par latency tier |

**Effort estimé : 1-2h pour C1-C4 (mesurable). C5-C6 = design decision.**

---

## Niveau 2 — Code changes modérés (kernel, 1 session)

| # | Gap | Impact | Design direction |
|---|---|---|---|
| P1 | **InferenceProfile hardcodé en Rust** | Ajouter un profil (labeling, classification) = recompile | Profiles dans backends.toml ou domain config, pas enum. OU : garder l'enum mais la rendre extensible via `Custom { max_tokens, temperature }` |
| P2 | **Prompt identique tous modèles** | Gemma/Qwen/Gemini ont des formats optimaux différents | Per-backend ou per-family prompt template dans `domains/`. Clé: `prompt_template = "gemma4"` dans backends.toml |
| P3 | **Pas de batch inference** | Nightshift/benchmark = séquentiel, machines idle | Batch endpoint dans ChatPort : `chat_batch(stimuli[]) -> responses[]` |
| P4 | **Embedding model hors-cadre** | Port 8081, pas un Dog, pas fleet-probed | Backend type `embedding` dans backends.toml, health-probed comme les autres |
| P5 | **Thinking budget** | Certains modèles (Qwen3, Gemma 4) supportent `thinking_budget` param | Champ `thinking_budget: Option<u32>` dans BackendConfig |
| P6 | **Tokenizer awareness** | Même max_tokens = counts différents par modèle | `tokens_per_char_estimate: f32` → budget adapté au modèle |

**Effort estimé : P1+P2 = 1 session. P3-P6 = design + impl, 2 sessions.**

---

## Niveau 3 — Outillage (scripts + automation, 1 session)

| # | Gap | Impact | Deliverable |
|---|---|---|---|
| Q1 | **Model qualification gate** | Download → swap → prier. 0 validation. | `scripts/qualify-dog.sh` : 3 stimuli → JSON valid? latency < timeout? pass/fail |
| Q2 | **Benchmark automation** | benchmark.py existe, manual, N=30, rare | `cynic-benchmark.timer` nightly 04:00 + Trackio (5 lignes) |
| Q3 | **Corpus expansion** | 30 stimuli, statistiquement faible | Metamorphic ×3 (30→120) + RAG-grounded (120→300) + adversarial mining (300→500) |
| Q4 | **Drift detection** | Aucun | Tier-accuracy Δ rolling 7j + Dog concordance alert si > φ⁻² |
| Q5 | **Calibration plots** | Aucun | Reliability diagram + Brier score per axiom per Dog |

**Effort estimé : Q1 = 30min. Q2 = 1h. Q3 = 2 sessions. Q4-Q5 = 1 session.**

---

## Niveau 4 — Architecture (post-hackathon)

| # | Gap | Design direction |
|---|---|---|
| A1 | **Model registry** | DB ou markdown tracking : modèle × quant × hardware × benchmark results × deploy dates |
| A2 | **A/B deployment** | Canary Dog : nouveau modèle reçoit 10% trafic, compare metrics vs incumbent |
| A3 | **Cost routing** | `cost_per_mtok` existe dans config mais 0 code l'utilise. Cheap Dog first → expensive si disagreement |
| A4 | **Domain routing** | Router stimuli vers Dogs calibrés par domaine (chess→qwen35, token→gemini) au lieu de fan-out total |
| A5 | **Inference organ v2** | Redesign de ChatPort + BackendPort comme un organe unifié : Dogs + embedding + labeling + fine-tuning dans le même framework |
| A6 | **Dog ID = role-based** | `sovereign-cpu`, `gpu-judge`, `hf-api` au lieu de `gemma-4-e4b-core`. Model = paramètre mutable |

---

## Anti-patterns observés (session 2026-04-17, patterns récurrents)

| # | Anti-pattern | Observation | Principe violé |
|---|---|---|---|
| AP1 | "Download → swap → pray" | Gemma 4 Q8_0 → thinking → timeout → 0 content | Qualify before deploy |
| AP2 | "One-size-fits-all inference" | Même max_tokens/prompt pour Gemma et Qwen | Model-literate, not model-agnostic |
| AP3 | "Test with random token count" | curl avec 80, 150, 400 → résultats non-comparables | Standardized benchmark protocol |
| AP4 | "Config exists, not populated" | BackendConfig a 14 champs, backends.toml en remplit 8 | Strong > No > Weak foundation |
| AP5 | "Grep audit < runtime observation" | Agent 4 dit dry_run, journalctl dit 401 | Probe live (#6), not static |

---

## Session 2026-04-17 — Results

**Titre :** "Inference Organ — model-literate foundations"

### Completed
- [x] Lire la doc llama.cpp pour Gemma 4 thinking mode control → `chat_template_kwargs: {"enable_thinking": false}` + `--reasoning off` CLI flag
- [x] Lire la doc Qwen3.5 thinking mode → `enable_thinking` via chat_template_kwargs, `/no_think` uncertain for 3.5
- [x] Vérifier capabilities gemini-cli → CliBackend ignores max_tokens/json_mode/disable_thinking entirely
- [x] C1-C4 : backends.toml completed for ALL backends (thinking, json_mode, context_size, comments)
- [x] Prompt budget audit : measured all 5 domains × 4 backends → Gemma overflows on 3/5 domains
- [x] External research : 6 papers + 6 GitHub repos deep-dived and mapped to truths
- [x] Crystallize-truth : 8 truths T1-T8 with confidence, falsification, design impact

### Key Finding (observed)
**The gap is deeper than config.** C1-C4 config fixes are necessary but insufficient:
- `disable_thinking=true` works (empirically verified: 666→75 completion tokens)
- BUT context overflow is the structural cause: chess prompt (1844 tok) > Gemma ctx (2048)
- token-analysis prompt (2439 tok) overflows by 741 tokens — no config can fix this
- Gemma returns `None` (zero tokens) on overflow — silent death, no error

### Falsification Result
> "si après la session, les 5 Dogs participent à chaque verdict avec json_valid_rate > 80%, le Niveau 1 est résolu"

**FALSIFIED.** 3/5 Dogs participate (deterministic + qwen-hf + qwen35-gpu). Gemma: context overflow. Gemini: quota exhausted. The gap IS deeper than config — it requires capability-aware dispatch (see INFERENCE-FOUNDATIONS.md).

### Next Session Roadmap (priority pyramid)

```
1. MEASURE (Q5+Q6): Gemma actual tokenization + ctx=4096/8192 feasibility test (5 min)
2. IMPLEMENT T1+T6: Pre-call context check + DogCapabilities (kernel code, 1-2h)
3. IMPLEMENT T2: Tiered prompts — condensed for small-ctx backends (1h)
4. MEASURE T5: Dog discrimination σ + inter-rater Spearman ρ (qualify-dog.sh, 30min)
5. DESIGN T3: Strategy diversity — at least 2 distinct eval strategies across Dogs
```

Full analysis: `docs/inference/INFERENCE-FOUNDATIONS.md`

---

## Observation méta (epistemic: deduced, 0.55)

*"Le problème d'inférence n'est pas un problème de config. C'est un problème d'architecture : le pipeline envoie le même prompt à tous les backends sans vérifier s'il tient dans le contexte. C'est comme envoyer un email de 10MB à une boîte qui accepte 5MB — le fix n'est pas de compresser l'email, c'est de vérifier la taille AVANT d'envoyer."*

Le pattern existe (LiteLLM `enable_pre_call_checks`). Le code existe dans des projets starred (llmfit, openfang). L'implémentation est 50-100 lignes dans `dogs/inference.rs` ou `pipeline/`. La connaissance est maintenant acquise — la prochaine session peut CODER.
