# CYNIC Naming Convention

Crystallized 2026-03-24. Researched from: Rust API Guidelines, Vector (Datadog), OpenFang, Lighthouse (Ethereum consensus), ai-hedge-fund, Mozilla Application Services.

This document defines HOW things are named in CYNIC. Every new name must follow these conventions. Existing violations are documented as legacy debt with migration paths.

---

## Principles

1. **One name, one concept.** A name must encode exactly one thing. If a word means two things, rename one of them.
2. **Identity ≠ infrastructure.** A Dog's identity is not its hostname, provider, or model. Identity is permanent; infra changes.
3. **Domain vocabulary in domain contexts.** Config files, APIs, and code use CYNIC terms (Dog, Verdict, Crystal). External-facing docs explain the vocabulary.
4. **Stable for 7 years.** A naming decision should hold without friction for 7 years. If it won't, it's the wrong decision.
5. **Mechanically enforced.** Conventions that depend on human compliance will drift. Use compiler lints, hooks, or CI checks.

---

## Dog Identity

Researched from: Lighthouse (`validator_definitions.yml` — pubkey as identity), Vector (user-chosen IDs), ai-hedge-fund (strategy names).

**Pattern: Generated ID + display_name.**

Each Dog has:
- `id`: A short hex string (6-8 chars), generated once at creation, **never changes**. This is the formal identity used in verdicts, logs, API responses, and cross-node references.
- `display_name`: A human-readable label. Can change freely. Used in dashboards, logs, documentation.
- `kind`: The Dog type discriminant. Determines which config fields are required.

**Why generated IDs?** CYNIC will support clusters and federation. A Dog ID must be globally unique and stable across node migrations, model changes, and infrastructure swaps. Named IDs (`gemini`, `sovereign-ubuntu`) collide across nodes and break when infra changes.

**Example (`dogs.toml`):**
```toml
[dog.a1b2c3]
display_name = "Gemini Flash (Google Cloud)"
kind = "inference"
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key_env = "GEMINI_API_KEY"
model = "gemini-2.5-flash"
auth_style = "bearer"

[dog.d4e5f6]
display_name = "Heuristic scorer"
kind = "heuristic"

[dog.f7a8b9]
display_name = "Sovereign Ubuntu (Gemma 3 4B)"
kind = "inference"
base_url = "http://<TAILSCALE_UBUNTU>:8080/v1"
api_key_env = "CYNIC_SOVEREIGN_KEY"
model = "gemma-3-4b"
auth_style = "bearer"
```

**Dog kinds (extensible enum):**
- `heuristic` — Pure algorithmic scoring, no network calls. In-kernel.
- `inference` — LLM-based evaluation via OpenAI-compatible endpoint.
- Future: `ensemble`, `statistical`, `rag-augmented`, etc.

**Migration path:** Current `backends.toml` → `dogs.toml`. Current dog names (`gemini`, `sovereign-ubuntu`, `deterministic-dog`) become `display_name` values. New hex IDs are generated. API responses include both `dog_id` (hex) and `display_name`. Transition period: accept both old names and new IDs.

---

## Environment Variables

Researched from: Vector (`VECTOR_*`), OpenFang (`OPENFANG_*`), Datadog Agent (`DD_*`).

**Pattern: `CYNIC_` prefix for all project-internal variables. Third-party namespaces preserved.**

| Category | Prefix | Examples |
|----------|--------|----------|
| Kernel config | `CYNIC_` | `CYNIC_REST_ADDR`, `CYNIC_API_KEY`, `CYNIC_CORS_ORIGINS` |
| Sovereign infra | `CYNIC_SOVEREIGN_` | `CYNIC_SOVEREIGN_KEY`, `CYNIC_SOVEREIGN_URL` |
| Kernel internals | `CYNIC_` | `CYNIC_AGGREGATE_INTERVAL`, `CYNIC_MODELS_DIR`, `CYNIC_EMBED_URL` |
| SurrealDB | `SURREALDB_` | `SURREALDB_URL`, `SURREALDB_PASS`, `SURREALDB_NS` |
| Gemini | `GEMINI_` | `GEMINI_API_KEY`, `GEMINI_MODEL` |
| HuggingFace | `HF_` | `HF_TOKEN` |
| Ollama | `OLLAMA_` | `OLLAMA_MODELS` |
| KAIROS | `KAIROS_` | `KAIROS_DB_URL` |

**Rules:**
- Every `CYNIC_*` variable has exactly ONE consumer in the codebase. If two systems need the same value, one reads the env var and passes it to the other.
- No variable exists in `~/.cynic-env` without at least one consumer. Ghost exports are debt.
- The `api_key_env` pattern in TOML (OpenFang pattern): config points to env var name, not the secret value.
- `setup-ubuntu.sh` and `~/.cynic-env` must agree on variable names and protocols (no `ws://` vs `http://` drift).

**Current violations (legacy debt):**
- `SOVEREIGN_API_KEY` exists alongside `CYNIC_SOVEREIGN_KEY` — same secret, two names. Migrate to `CYNIC_SOVEREIGN_KEY` only.
- `CYNIC_SOVEREIGN_URL` in `~/.cynic-env` is not consumed by the kernel — either wire it or document as scripts-only.
- `SURREALDB_URL` protocol differs between `setup-ubuntu.sh` (`ws://`) and deployed env (`http://`). Standardize.

---

## API JSON Fields

Researched from: Rust API Guidelines, REST best practices.

**Pattern: `snake_case` throughout. One concept = one field name across all endpoints.**

| Concept | Canonical field name | Used in |
|---------|---------------------|---------|
| Dog identifier | `dog_id` | `/judge`, `/health`, `/usage`, `/dogs` — everywhere |
| Dog display name | `display_name` | `/health`, `/dogs` |
| Dog type | `kind` | `/health`, `/dogs` |
| System health status | `status` | `/health` — values: `healthy`, `degraded`, `critical` |
| Axiom scores | `fidelity`, `phi`, `verify`, `culture`, `burn`, `sovereignty` | `/judge` q_score and dog_scores |
| Axiom list (display) | `axioms` | `/health` — `["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN", "SOVEREIGNTY"]` |

**Breaking changes from current API:**
- `status: "sovereign"` → `status: "healthy"` (T1 — "sovereign" reserved for infra concept)
- `/health` dogs: `id` → `dog_id` (consistency)
- `/judge`: `dogs_used` (String, joined) → `dog_ids` (Array of String) — fixes type mismatch
- Add `display_name` and `kind` to dog objects in `/health`

**Note:** Frontend (S.) does not consume these fields yet. Migration is free.

---

## Rust Types

Follows Rust API Guidelines (RFC 430) + Mozilla Application Services conventions.

| Category | Pattern | Examples |
|----------|---------|----------|
| Modules | `snake_case` | `health_gate`, `verdict_cache`, `circuit_breaker` |
| Types, Traits | `UpperCamelCase` | `StoragePort`, `HealthGate`, `VerdictKind` |
| Port traits | `{Domain}Port` | `StoragePort`, `CoordPort`, `EmbeddingPort` |
| Gate traits | `{Domain}Gate` | `HealthGate` |
| Null impls | `Null{Concept}` | `NullStorage`, `NullCoord`, `NullInfer` |
| Error types | `{Domain}Error` (enum) | `StorageError`, `JudgeError`, `DogError` |
| Constants | `SCREAMING_SNAKE_CASE` | `PHI_INV`, `HEURISTIC_DOG_KIND` |
| Dog IDs | `const` not string literals | `pub const HEURISTIC_DOG_ID: &str = "..."` → eliminate scattered `"deterministic-dog"` |

**Current violations:**
- `BackendInitError` is a struct, not an enum (all other errors are enums). Migrate to enum variant.
- `"deterministic-dog"` appears as string literal in 12 places. Extract to const.
- `domain/temporal.rs` exists but module is commented out (Rule #21 — burn or wire).

---

## Config Files

**Pattern: Domain vocabulary. `kind` field as discriminant. `snake_case` keys.**

| File | Purpose | Key pattern |
|------|---------|-------------|
| `dogs.toml` | Dog registration + infra params | `[dog.ID]` with `kind`, `display_name`, type-specific fields |
| `~/.cynic-env` | Secrets + env vars | `SCREAMING_SNAKE_CASE`, sourced by shell |
| `~/.config/cynic/env` | Systemd env (generated from `~/.cynic-env`) | Same names, generated by deploy scripts |

**Migration:** `backends.toml` → `dogs.toml`. `[backend.NAME]` → `[dog.ID]`. Add `kind` and `display_name` fields.

---

## Infrastructure Placeholders

For documentation and example configs (non-executable):

| Placeholder | Machine | Role |
|-------------|---------|------|
| `<TAILSCALE_UBUNTU>` | T.'s Ubuntu server | Backend (kernel, DB, sovereign LLM) |
| `<TAILSCALE_GPU>` | S.'s Windows/GPU machine | Frontend + GPU inference |
| `<TAILSCALE_FORGE>` | CI/build server | Builds (currently offline) |

**Rules:**
- Angle brackets = non-executable placeholder (docs, example configs)
- Executable code uses env vars (`$CYNIC_SOVEREIGN_URL`), never placeholders
- No abbreviations (`<TAILSCALE_S>` → use full `<TAILSCALE_GPU>`)

---

## Mechanical Enforcement

Conventions that are not enforced mechanically will drift. Target gates:

| Convention | Enforcement | Status |
|-----------|-------------|--------|
| No real IPs in tracked files | `gitleaks` hook (active) + grep in `make check` | Partially active |
| `CYNIC_*` prefix for internal vars | Grep hook on new env var additions | Not yet |
| `dog_id` field name consistency | Integration test checking API response shapes | Not yet |
| No string-literal dog IDs | `clippy` can't catch this. Grep-based hook: `grep '"deterministic-dog"'` must return only the const definition | Not yet |
| Health status values | Unit tests for `system_health_status()` | Active (7 tests) |

---

## Appendix: Research Sources

- [Rust API Guidelines — Naming](https://rust-lang.github.io/api-guidelines/naming.html)
- [Vector STYLE.md](https://github.com/vectordotdev/vector/blob/master/STYLE.md)
- [Vector RUST_STYLE.md](https://github.com/vectordotdev/vector/blob/master/docs/RUST_STYLE.md)
- [Mozilla Application Services — Naming Conventions](https://mozilla.github.io/application-services/book/naming-conventions.html)
- [Lighthouse validator_definitions.rs](https://github.com/sigp/lighthouse/blob/stable/common/account_utils/src/validator_definitions.rs) — serde tagged enum for heterogeneous validator config
- [OpenFang .env.example + config.toml](https://github.com/RightNow-AI/openfang) — `{PROJECT}_` env prefix, `api_key_env` indirection
- [Datadog Agent env vars](https://docs.datadoghq.com/agent/guide/environment-variables/) — `DD_*` prefix convention
- [Vector secret management RFC](https://github.com/vectordotdev/vector/blob/master/rfcs/2022-02-24-11552-dd-agent-style-secret-management.md)
- [Martin Fowler — Ubiquitous Language](https://martinfowler.com/bliki/UbiquitousLanguage.html)
- [Enterprise Craftsmanship — Ubiquitous Language and Naming](https://enterprisecraftsmanship.com/posts/ubiquitous-language-naming/)
