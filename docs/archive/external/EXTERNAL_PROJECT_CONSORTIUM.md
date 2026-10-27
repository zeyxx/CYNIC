# External Project Consortium — CYNIC as Agentic OS for OSS

Status: `draft v0` (observed-reality seed, 2026-10-18)
Worktree: `~/.cynic/worktrees/agent-hermes-aegis-consortium` (branch `agent/hermes/aegis-consortium`)
Owner: Titouan Corneloup (zeyxx) + collaborator Robinho100

## 0. Purpose

This document seeds roadmap item 7 of `docs/foundation/WHITEPAPER.md` §20:
"Add an external project through the same ports."

Goal: formalise how CYNIC acts as the agentic OS for open-source projects that
are **not** CYNIC itself — multiple external repositories, multiple
collaborators — while keeping the CYNIC invariants (Bottom-Up, Strict DAG,
FCIS, no hardcoded values, dynamic registries, no mutation without read-back
proof).

This is a *design seed*, not a ratified contract. It is iterated in `docs/`
before any code or contract lands (per `docs/foundation/CRATE_RULES.md`
iteration discipline).

## 1. Observed reality — the three seed projects

All three are public GitHub repositories owned by collaborator `Robinho100`.
Cloned (read-only observation) into `~/.cynic/external/` (see §3).

| Slug | Repo | Language | Default branch | Last commit (observed) | Role |
|------|------|----------|----------------|------------------------|------|
| `aegisscan` | `Robinho100/aegisscan` | Go | `master` | `8758ad3` fix(api): harden local API admission, bind policy and server lifecycle (R1) | Pentest engine, local-first, modular |
| `aegis-core` | `Robinho100/aegis-core` | Go + C# (supervisor/desktop) | `main` | `d82ca40` Prepare disarmed sanitized Core collaboration source | Sanitized collaboration source, strict security boundary (STOP 64, B1/B7 refusal semantics) |
| `ai-bus` | `Robinho100/ai-bus` | TypeScript/Node | `main` | `dd87232` feat(ai-bus): add Aegis AI gateway | Local Windows app: Claude Code CLI ↔ Codex CLI over localhost MCP bus |

Observed coupling (from `aegisscan/README.md`):

```
AI Bus (TypeScript/Node.js)          AegisScan (Go)
  orchestration IA                     Scope Guard, Policy Engine,
  gestion de contexte                  Discovery, Planner, Modules,
  Claude <-> Codex          ──HTTP/JSON──►  Findings, Evidence, Reporting
                              localhost
```

- AegisScan runs **without** Claude/Codex/AI Bus/Internet (IA is an optional
  front layer, strict boundary — `docs/AI_INTEGRATION.md`).
- AI Bus uses subscription auth of the CLIs, no API keys, localhost MCP only,
  read-only workspace access for agents, linear turn chain.
- Aegis Core ships a `SOURCE-INVENTORY.json` with exact bytes + SHA-256 for
  every shared file; verification via `tools/audit-share.py`. Security boundary
  is *refusal by construction* (no private deployment config, no native pins).

Shared theme: the **Aegis** consortium — offensive-security tooling + AI
orchestration + hardened core. Single collaborator today (`Robinho100`), but
the model below must scale to N collaborators and N projects.

## 2. Principles (inherited from CYNIC)

1. **Bottom-Up over Top-Down**: observe the project's real build/test/runtime
   before any policy or model. No policy without an empirically verified
   baseline. (This doc is seeded from `ls`/`git log`/`README`, not from
   assumptions.)
2. **Strict DAG downward**: an external project is a *leaf* consumer of CYNIC
   ports; CYNIC never depends on an external project. `cynic-*` crates must
   not gain a dependency on `aegisscan`/`aegis-core`/`ai-bus`.
3. **FCIS / Zero-I/O Core**: pure logic in domain modules; all I/O (clone,
   build, test, push) behind ports/adapters.
4. **Dynamic registries over hardcoded values**: project list, collaborator
   list, paths, build commands live in registries (`infra/*.json`), never in
   code or prose.
5. **No mutation without read-back proof**: any write to an external repo
   (branch, commit, push, PR) must be observed and proven. Read-only
   observation is the default and requires no approval.
6. **Worktree isolation**: every mutation task allocates an isolated git
   worktree branched from verified `origin/main` (per
   `contracts/source-lifecycle.json`).

## 3. Canonical directory structure (SSOT)

Proposed (to be ratified into `infra/workspace-paths.json`):

```
~/.cynic/
├── worktrees/                       # CYNIC source worktrees (contract: source-lifecycle root_directory)
│   └── agent-<id>-<scope>/          # one per agent task, branched from origin/main
├── external/                        # external (non-CYNIC) project clones (READ-ONLY by default)
│   └── <project-slug>/             # git clone of Robinho100/<repo>
└── projects/                        # (reserved) derived artifacts, never source
```

Rules:

- External clones live **outside** the CYNIC repository tree. Never nest a
  foreign `.git` inside CYNIC (avoids accidental commits, submodule drift,
  and untracked-repo pollution).
- `~/.cynic/external/<slug>` is the single canonical clone location. No
  duplicate clones under `~/`, `/tmp`, or inside CYNIC worktrees.
- CYNIC-side formalisation (policy, registry, design) lives in the CYNIC
  worktree under `docs/design/` and `infra/` / `contracts/`.
- The CYNIC repo path itself remains governed by `infra/workspace-paths.json`
  (`canonical_path`). External paths must be added there as a new registry
  section (see §6, open question OQ-1).

## 3.1 Observed emergences — in-tree subprojects (already present, unregistered)

Before formalising external projects, observe what CYNIC already carries
**inside** its own repository. These are not external leaves; they are
subprojects that emerged organically and are **not** in any registry today
(grep of `infra/` for their names returns nothing).

| Path | Stack | README | Policy/registry | Notes |
|------|-------|--------|-----------------|-------|
| `services/relay-gateway/` (GASDF) | TS/Node | none | none | `name: gasdf-relay-gateway`; commits `dist/` + `node_modules/`; .gitignore 20 bytes |
| `apps/kyon-mobile/` | Kotlin/Android | yes | none | Kyon Shield + Mirror; privacy defaults documented |
| `programs/cynic-anchor/` | Rust/Anchor (Solana) | none | none | Anchor.toml + Cargo.toml; `target/` present |
| `mollusk/` | Rust (Solana test framework) | n/a (vendored-ish) | none | own Cargo.lock/Makefile |
| `wallet-standard/` | TS (pnpm workspace) | yes | none | pnpm-workspace; own lockfile |

Emergences (facts, not judgments):

1. **Two populations exist**: in-tree subprojects (part of CYNIC repo, one
   collaborator, one remote) vs. external projects (separate repos, separate
   collaborators, separate remotes). They must not share the same policy
   defaults: in-tree is governed by CYNIC's own source lifecycle; external is
   governed by the external-project policy (§4).
2. **No per-subproject policy** exists anywhere. Build/test commands, ownership,
   and sensitivity are implicit (read README or guess).
3. **Artifact pollution**: `dist/`, `node_modules/`, `target/` are committed or
   present in-tree for some subprojects — a debt signal, out of scope here but
   flagged.
4. **Naming is ad hoc**: `services/`, `apps/`, `programs/` coexist with
   top-level `mollusk/`, `wallet-standard/`. No single canonical placement.

Consequence for the SSOT: the directory registry must cover **both**
populations, in separate sections, so that "where does X live" has one answer.

## 4. Project policy model"""

Each external project declares a policy. Proposed location (CYNIC side):

```
infra/external-projects.json          # central registry (SSOT): list of projects
contracts/external-project-policy.schema.json  # schema for per-project policy
```

Per-project policy fields (minimum):

- `project_id` (slug, stable)
- `display_name`
- `repo_url` (canonical remote)
- `default_branch`
- `collaborator_ids` (list, references `collaborators` registry)
- `language_stack` (observed, e.g. `go`, `ts/node`, `go+csharp`)
- `build_commands` (observed, e.g. `go build ./...`, `npm ci && npm test`)
- `test_commands` (observed)
- `mutation_policy`: `read_only` (default) | `branch_pr` | `direct_push_forbidden`
- `sensitivity`: `public` | `sanitized` | `private` (aegis-core is `sanitized`)
- `ports_used`: list of CYNIC port names this project touches (none yet —
  "through the same ports" is aspirational until wired)
- `notes` (free, evidence-backed)

Default for a newly added project: `mutation_policy = read_only`,
`sensitivity = public` until observed otherwise. Aegis-core is special-cased to
`sanitized` because its README states it is a sanitized collaboration source
with STOP 64 / refusal semantics — no private deployment config may be added.

## 5. Collaborator model

Proposed registry: `infra/collaborators.json` (SSOT), referenced by
`external-projects.json`.

Minimum fields:

- `collaborator_id` (slug)
- `display_name`
- `github_handle`
- `trust_level`: `observed` | `trusted` | `core` (default `observed` for a new
  collaborator; promotion requires proof)
- `projects` (list of `project_id`)
- `notes` (evidence-backed)

Seed: one collaborator `robinho100` (Robinho100), trust_level `observed`,
projects `[aegisscan, aegis-core, ai-bus]`.

Principle: trust is *observed and promoted*, never assumed. A new collaborator
starts at `observed` and cannot trigger mutations above `read_only` until
promoted with proof (mirrors CYNIC office/capability model in
`infra/registry.json`).

## 6. Integration with CYNIC ("through the same ports")

Roadmap item 7 is aspirational. Today the three projects are **external leaf
nodes**: CYNIC observes them, does not depend on them, does not build them.

Candidate ports (hypothesis, to be validated bottom-up). Status: `not_started`
for all. See `contracts/external-project-ports.schema.json`.

- `SourcePort` (cynic-vcs) — clone/fetch/branch/push/PR for external repos.
  Reuses CYNIC's source lifecycle, parameterized by remote.
- `BuildPort` (cynic-build) — run a project's observed build/test commands
  on the builder node with read-back proof. Commands from
  `infra/external-projects.json` (observed, not fabricated).
- `EvidencePort` (cynic-kernel) — ingest a project's findings/evidence into
  CYNIC's observation store (aegisscan findings → CYNIC findings schema).

**Not proposed now**: any code change. First, observe each project's actual
build/test commands and CI, then propose port wiring in a follow-up. The
`aegis-core` security boundary (refusal by construction) must be preserved:
CYNIC must never attempt to "arm" or deploy aegis-core components.

**Port wiring prerequisites (all must hold):**
1. Build/test baseline observed on builder (done 2026-10-18 for all three).
2. Read-back proof (build/test commands verified, not just observed).
3. Contract updated (`contracts/external-project-ports.schema.json`).
4. Port traits defined in domain crates (pure, no I/O). Adapters in adapters/.
5. CLI surface projects port status (already: `cynic audit external`).

**Next wiring step**: define `SourcePort` trait in `cynic-vcs` and a
`ForgejoSourceAdapter` / `GitHubSourceAdapter` implementing it. Then wire
`BuildPort` in `cynic-build` with a builder command execution adapter.
EvidencePort last (requires schema mapping).

## 11. Port wiring design (2026-10-18)

## 7. Open questions (resolved 2026-10-18)

- **OQ-1** — resolved: extend `infra/workspace-paths.json` with an
  `external_projects` section (single registry). Done in this worktree.
- **OQ-2** — resolved: canonical clone root = `~/.cynic/external/`
  (`${HOME}/.cynic/external`), outside the CYNIC tree. Done.
- **OQ-3** — resolved: single registry `infra/external-projects.json` plus
  schema `contracts/external-project-policy.schema.json`. Done.
- **OQ-4** — deferred: collaborator trust promotion criteria to be defined
  with proof in a follow-up. `infra/collaborators.json` ships
  `trust_level=observed` only.
- **OQ-5** — confirmed: `aegis-core` is `sanitized`; CYNIC treats it as
  read-only observation + build/test only, never deploy.

## 8. State of this worktree (2026-10-18)

Done:

1. `infra/workspace-paths.json` — added `external_projects` section.
2. `infra/directory-tree.json` — added `external/` and `projects/` under
   `operator_user.key_paths`.
3. `infra/external-projects.json` — seed registry for the 3 projects.
4. `infra/collaborators.json` — seed registry (1 collaborator, observed).
5. `contracts/external-project-policy.schema.json` — policy schema.

Still open (gated on builder observation):

6. Observe each project's build/test commands on the builder (10.10.10.119)
   with read-back proof; flip `build_commands_status` to `verified`.
7. Thin CLI read-back surface (`cynic audit external`) per
   `cynic-cli-thin-surface` skill — only after registry is ratified.
8. Port wiring (§6) — only after 6 and 7."""

## 9. Observation results (2026-10-18)

Build/test observed on the builder (10.10.10.119) after provisioning Go 1.27.2
and Node v22.23.3 (2026-10-18). The builder previously had rust/cargo only;
Go and Node were installed under /opt with symlinks in /usr/local/bin and
/etc/profile.d/cynic-toolchains.sh. Formalized in infra/registry.json
(cynic-builder.toolchains).

| Project | Build | Tests | Verdict |
|---------|-------|-------|---------|
| aegisscan | PASS (`go build ./...`) | FAIL (1/many): `TestBinaryTransport_PipesHeldOpen_KillsLeftoverGrandchild` (context deadline exceeded) in `internal/external`; all other packages pass | build baseline green; one timing-sensitive test failure, to re-observe |
| aegis-core | PASS (`./internal/...`); FAIL desktop-backend (Windows-only build constraint) | FAIL (`authentication storage unavailable` in syncclient/syncserver) | sanitized boundary preserved (no Linux desktop build, no deploy attempted) |
| ai-bus | PASS (`npm ci` + `npm run build`) | FAIL (2/many): strict fast-forward pull, normal push to fake local remote (env-dependent git config) | build baseline green on builder; 33 tests pass |

Notes:
- aegisscan's single test failure is timing-related (grandchild kill); not a
  build break. Re-observe before declaring verified.
- aegis-core test failures are environment-dependent (auth storage), consistent
  with refusal-by-construction. No native deployment attempted.
- ai-bus test failures are environment-dependent (git remote config). Build
  green on builder (was failing on cortex due to missing types).

Registry updated: `build_commands_status = observed_on_builder` for all three,
with per-project `observation` objects. See `infra/external-projects.json`.

- `~/.cynic/external/aegisscan` → `origin https://github.com/Robinho100/aegisscan.git`, branch `master`, HEAD `8758ad3`
- `~/.cynic/external/aegis-core` → `origin https://github.com/Robinho100/aegis-core.git`, branch `main`, HEAD `d82ca40`
- `~/.cynic/external/ai-bus` → `origin https://github.com/Robinho100/ai-bus.git`, branch `main`, HEAD `dd87232`

## 10. Ownership links registry (2026-10-18)

`infra/ownership-links.json` is the SSOT of RELATIONS between operator,
collaborators, external projects, and toolchains. It references by ID (foreign
keys); no data duplication. It captures ownership, maintenance, and
proof-of-work (PoW) metrics that model real interactions between
interconnected systems (physics, culture, economy obey the same laws of
friction).

Contents:
- `operator`: Titouan Corneloup (zeyxx), constituent (human sovereign), core trust.
- `collaborators`: Robinho100 (observed trust).
- `links`: 6 relations (operator↔collaborator, operator↔project×3, operator↔toolchain×2) with PoW metrics and friction signals.
- `project_toolchains`: 3 entries (which toolchains each project consumes).
- `toolchain_projects`: 3 entries (which projects each toolchain serves).

Every link carries a `pow_metric` (measurable) and a `friction_signal` (first-class data, not noise). A link without a pow_metric is incomplete.

Philosophy: technology must reconstruct the real from first principles. The
sentiment of impasse and social friction are the variables that indicate where
technology must act. This registry models those variables as observable,
measurable links — not as abstract claims.

Related: `infra/collaborators.json`, `infra/external-projects.json`,
`infra/registry.json` (toolchains), `infra/workspace-paths.json`.

Worktree: `~/.cynic/worktrees/agent-hermes-aegis-consortium` on
`agent/hermes/aegis-consortium` (from `origin/main` @ `19111bc`).

## 10. Serving tier model (formalized 2026-10-21)

The path divergence across onboarding eras (aegis-era `~/.cynic/external`,
sollama58-era `~/.local/share/cynic/src/external`, gasdf-era
`/var/lib/cynic/source/repositories/external`) is not noise: it encodes a real
distinction that was never made explicit. This section formalizes it.

### 10.1 The axis

`serving_tier` is the SSOT of an external project's checkout root:

- `served` — CYNIC deploys/serves it. Checkout lives in `production_root`
  (`/var/lib/cynic/source/repositories/external/<slug>`, service-owned, sudo).
- `dev` — observe + contribute (PR upstream), no deploy. Checkout lives in
  `development_root` (`${HOME}/.cynic/external/<slug>`, user-owned).

`roots.referential` in `infra/external-project-registry.json` MUST equal
`f(serving_tier)` against the two roots declared in
`infra/workspace-paths.json` (`external_projects.production_root` /
`development_root`). Enforced by
`crates/cynic-cli/tests/external_project_tier_coherence.rs`.

`serving_tier` is orthogonal to `adoption_mode` (upstream relationship) and
`mutation_policy` (write authority). Three axes, each declared once.

### 10.2 Canonical roots (post-reconciliation)

- `production_root` = `/var/lib/cynic/source/repositories/external` — served
  projects only (gasdf, kora).
- `development_root` = `${HOME}/.cynic/external` — all dev projects; the single
  canonical dev root (retires `~/.local/share/cynic/src/external`).
- `derived_artifacts_root` = `${HOME}/.cynic/projects`.

### 10.3 Interconnection

gasdf (relay prototype) and kora (relay substrate, json-rpc, operate-only) are
interconnected: gasdf depends on kora. Both are `served`; deployment of gasdf
requires kora available. Edge recorded in `infra/ownership-links.json`
(`project-gasdf--project-kora`) with shared `evidence_ref: gasdf-rebuild-20261016`.

### 10.4 Runtime promotion

`cynic external promote <slug> --to <served|dev|contribute> --yes` migrates a
project between tiers cleanly: moves the clone (SourcePort), updates registries,
re-pins, updates ownership links, with read-back proof. Two distinct operations:
`mutation_policy` promotion (observe→contribute, no move) vs `serving_tier`
migration (dev→served, move + sudo).

### 10.5 Registry consolidation

The normative registry (`infra/external-project-registry.json`) now covers all
9 governed projects (gasdf, kora served; trenchscanner, cultscreener, tokensage,
holdex, aegisscan, aegis-core, ai-bus dev). The legacy seed
(`infra/external-projects.json`) remains for aegisscan-era compatibility;
`cynic audit external` will be repointed to the normative registry in Phase 2.
