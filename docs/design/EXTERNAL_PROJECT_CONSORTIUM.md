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
contracts/external-project-policy.v1.json  # schema for per-project policy
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

For a project to be "added through the same ports", it must expose/consume a
CYNIC port (defined in `crates/cynic-kernel` ports / `adapters/`). Candidate
ports for the Aegis consortium (hypothesis, to be validated bottom-up):

- `SourcePort` — clone/fetch/branch/push/PR for external repos (reuse
  CYNIC's own source lifecycle, parameterised by remote).
- `BuildPort` — run a project's observed build/test commands on the builder
  node (10.10.10.119) with read-back proof.
- `EvidencePort` — ingest a project's findings/evidence into CYNIC's
  observation store (aegisscan findings → CYNIC findings V2 schema?).

**Not proposed now**: any code change. First, observe each project's actual
build/test commands and CI, then propose port wiring in a follow-up. The
`aegis-core` security boundary (refusal by construction) must be preserved:
CYNIC must never attempt to "arm" or deploy aegis-core components.

## 7. Open questions (resolved 2026-10-18)

- **OQ-1** — resolved: extend `infra/workspace-paths.json` with an
  `external_projects` section (single registry). Done in this worktree.
- **OQ-2** — resolved: canonical clone root = `~/.cynic/external/`
  (`${HOME}/.cynic/external`), outside the CYNIC tree. Done.
- **OQ-3** — resolved: single registry `infra/external-projects.json` plus
  schema `contracts/external-project-policy.v1.json`. Done.
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
5. `contracts/external-project-policy.v1.json` — policy schema.

Still open (gated on builder observation):

6. Observe each project's build/test commands on the builder (10.10.10.119)
   with read-back proof; flip `build_commands_status` to `verified`.
7. Thin CLI read-back surface (`cynic audit external`) per
   `cynic-cli-thin-surface` skill — only after registry is ratified.
8. Port wiring (§6) — only after 6 and 7."""

## 9. Observation results (2026-10-18)

Build/test observed on cortex (go 1.27.2, node v22.23.3, npm 10.9.9). The
builder (10.10.10.119) was probed via SSH and has **rust/cargo only** — no
go, no node/npm. Consequence: external Go/Node project observation runs on
cortex until Go/Node are provisioned on the builder; the builder remains SSOT
for Rust/cargo gates only. This is a gap to track, not a failure of this
work.

| Project | Build | Tests | Verdict |
|---------|-------|-------|---------|
| aegisscan | PASS (`go build ./...`) | FAIL (1/many): `TestBinaryTransport_PipesHeldOpen_KillsLeftoverGrandchild` (context deadline exceeded) in `internal/external`; all other packages pass | build baseline green; one timing-sensitive test failure, to re-observe |
| aegis-core | PASS (`./internal/...`); FAIL desktop-backend (Windows-only build constraint) | FAIL (`authentication storage unavailable` in syncclient/syncserver) | sanitized boundary preserved (no Linux desktop build, no deploy attempted) |
| ai-bus | FAIL (`npm ci` OK, `npm run build` exit 2: missing @types/node, node:sqlite/fs/promises type decls) | FAIL (ERR_MODULE_NOT_FOUND ×33, no dist/) | targets Windows 10/11; Linux baseline not green; observed, not corrected (read-only policy) |

Notes:
- aegisscan's single test failure is timing-related (grandchild kill); not a
  build break. Re-observe before declaring verified.
- aegis-core test failures are environment-dependent (auth storage), consistent
  with refusal-by-construction. No native deployment attempted.
- ai-bus requires Windows or dependency fixes; left as observed.

Registry updated: `build_commands_status = observed_on_cortex` for all three,
with per-project `observation` objects. See `infra/external-projects.json`.

- `~/.cynic/external/aegisscan` → `origin https://github.com/Robinho100/aegisscan.git`, branch `master`, HEAD `8758ad3`
- `~/.cynic/external/aegis-core` → `origin https://github.com/Robinho100/aegis-core.git`, branch `main`, HEAD `d82ca40`
- `~/.cynic/external/ai-bus` → `origin https://github.com/Robinho100/ai-bus.git`, branch `main`, HEAD `dd87232`

Worktree: `~/.cynic/worktrees/agent-hermes-aegis-consortium` on
`agent/hermes/aegis-consortium` (from `origin/main` @ `19111bc`).
