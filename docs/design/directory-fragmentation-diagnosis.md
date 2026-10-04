# Directory Structure & Config Fragmentation — Diagnosis

Status: `diagnosis` — evidence only, no proposal yet.
Schema: `cynic.directory_fragmentation_diagnosis.v1`
Method: every claim below is a count or a hash produced by a command in §5.

---

## 0. The claim under test

> "the directory structures are catastrophic and not DDD; there is
> fragmentation between policies/configs (which are moreover static for now)"

Verdict: **confirmed, and worse than stated.** Two independent findings:
the *config* layer has a live document and a dead parallel copy; the
*directory* layer has a normative authority that the tree does not obey.

---

## 1. Finding A — 19 vertical `intent.json` are dead duplicates

`infra/verticals/<name>/intent.json` — 19 files, one per vertical.

| Fact | Value |
|---|---|
| Files present | 19 |
| Rust readers (`intent.json` in `crates/**/*.rs`) | **0** |
| `registry.json` Rust readers | **247** |
| Verticals referenced from code (`infra/verticals`) | **0** |

**Byte-identical duplicates** (sha256 over canonical JSON):

| `registry.json` section | `verticals/…/intent.json` `desired_state` | Verdict |
|---|---|---|
| `provider_control` | `provider_enrollment` | **IDENTICAL** |
| `release_publication` | `release` | **IDENTICAL** |

**Diverged duplicates** (same responsibility, different content):

| `registry.json` section | vertical | Divergence |
|---|---|---|
| `network_policy` | `network` | registry has **only** `{canonical_service_plane, planes}`; the vertical also has `reservations` + **`wireguard.nodes[]`** |
| `storage_registry` | `storage` | registry has `{classes, volumes, claims, bindings}`; vertical has `{storage_role, bindings}` — **different shapes** |
| `node_enrollment` | `machine_enrollment` | same keys, different section name (`node_` vs `machine_`) |
| `nodes` | `node` | registry holds a **bare array**; vertical holds `{schema, nodes}` |
| `fleet_desired` | `fleet` | registry `{targets}`; vertical `{schema, targets}` |
| `lifecycle` | `lifecycle` | registry `{kernel_release}`; vertical `{schema, kernel_release}` |

**This is the root cause of D4** (the WireGuard readers pointing at a
nonexistent path). The WireGuard desired state *does* exist — in
`verticals/network/intent.json`, the one document with **zero readers**. The
readers were written against `/network_policy/wireguard/nodes`, which is what
you get if you assume the registry is the vertical with `schema` stripped.
Nobody ever wired the two together.

**Note the inversion:** the live document (`registry.json`, 247 readers) is
missing `wireguard`; the dead document has it. So "configs are static" is
literally true — the part that would make them dynamic has never been read.

---

## 2. Finding B — 24 top-level directories, 6 with zero code readers

| Directory | Files | Rust refs | Verdict |
|---|---|---|---|
| `crates/` | 1885 | 190 | canonical |
| `infra/` | 249 | 156 | canonical |
| `contracts/` | 16 | 42 | canonical (but see §3) |
| `docs/` | 203 | 7 | prose |
| `archive/` | 18 | 4 | legacy |
| `scripts/` | 20 | 4 | shell |
| `tests/` | 3 | 6 | canonical |
| `evidence/` | 2 | 1 | canonical |
| **`apps/`** | 31 | **0** | **dead** |
| **`programs/`** | 5 | **0** | **dead** |
| **`protocols/`** | 18 | **0** | **dead** |
| **`references/`** | 2 | **0** | **dead** |
| **`spaces/`** | 61 | **0** | **dead** |
| **`specs/`** | 6 | **0** | **dead** |

`spaces/` (61 files), `apps/` (31), `protocols/` (18) are the three largest
dead trees — 110 files with no executable consumer. `protocols/` contains
`headscale-mesh-coordination.md`, `network-topology-design.md`,
`tls-fleet-design.md` — network design prose that is *also* unread, which is a
second copy of the responsibility §1 shows fragmented.

Inside `infra/`, 9 of 19 subdirectories have **zero** code readers:

| infra subdir | Refs | infra subdir | Refs |
|---|---|---|---|
| `registries/` | 17 | `firewall/` | **0** |
| `surface-registry/` | 11 | `network/` | **0** |
| `privilege/` | 6 | `wireguard/` | **0** |
| `systemd/` | 4 | `verticals/` | **0** |
| `host-bootstrap/` | 2 | `catalogs/` | **0** |
| `agent-projections/` | 2 | `compose/` | **0** |
| `runtime-checkpoints/` | 1 | `data_system/` | **0** |
| | | `observability/` | **0** |
| | | `surrealdb/` | **0** |
| | | `sudoers.d/` | **0** |
| | | `benchmark-results/` | **0** |

`infra/network/`, `infra/wireguard/`, `infra/firewall/` — the three directories
a network architecture would own — are all unread. Same story as §1.

---

## 3. Finding C — the normative directory authority is not obeyed

`infra/directory-tree.json` (`status: normative`, 13 code readers) declares
itself *"Single Source of Truth for all filesystem paths. Every path in the
codebase MUST reference this document."*

It defines **5 layers**: `git_sources`, `installed_system`, `runtime_state`,
`logs`, `cache`. Their `anti_rules` state:

> - No path hardcoding outside this document — use runtime_paths resolution.
> - No duplication of git sources into databases (K6 lives in git).

Measured against reality:

| Anti-rule | Violation |
|---|---|
| "no duplication of git sources" | `registry.json` ↔ 2 verticals byte-identical (§1) |
| "every path MUST reference this document" | 6 top-level dirs and 11 `infra/` subdirs absent from the tree |
| `git_sources.key_files` (9 entries) | lists `contracts/` and `spaces/operator/` — but **not** `infra/verticals/`, `specs/`, `protocols/`, `apps/`, `programs/` |

So the normative document is itself incomplete: it names 9 git sources while
the tree holds 24 top-level directories. A document that says "no path outside
me" and then omits 15 of 24 top-level paths is not an authority, it is a
snapshot.

---

## 4. Finding D — the config layer has no declared precedence

`contracts/` holds 16 policy documents. Their reader counts:

| Read (≥5) | Weak (1–4) | **Zero readers** |
|---|---|---|
| `foundation.json` (79) | `framework.json` (3) | `build-placement.json` |
| `claims.json` (21) | `source-lifecycle.json` (2) | `observability-loop.json` |
| `pruning-policy.json` (18) | `relay-policy.json` (1) | `pool-contribution.json` |
| `ownership.json` (15) | `release-versioning.json` (1) | `release-domain.json` |
| `measure-arbitration.json` (11) | | `release-gates.json` |
| `publication.json` (5) | | `staging-model.json` |

**6 of 16 contracts are unread.** `release-gates.json` and
`release-domain.json` are unread while `release-versioning.json` has 1 reader —
three release contracts, none of which is a working authority.

Combined with §1 (registry vs verticals) and §3 (directory-tree incomplete),
the repository has **four parallel claim-sets** for "what the system should be":
`contracts/`, `infra/registry.json`, `infra/verticals/*/intent.json`,
`infra/directory-tree.json` + `infra/runtime-authority.json`. `AGENTS.md`
declares an `authority_order` among five of these, but nothing enforces that the
*lower* documents are consistent with the *higher* ones — which is exactly how
`registry.network_policy` lost `wireguard` while the vertical kept it.

---

## 5. Reproduce

```bash
# §1 dead verticals
grep -rn 'intent\.json' crates/ --include='*.rs' | wc -l          # -> 0
grep -rn 'registry\.json' crates/ --include='*.rs' | wc -l        # -> 247
# §1 byte-identical
python3 -c "
import json,hashlib
c=lambda o: hashlib.sha256(json.dumps(o,sort_keys=True,separators=(',',':')).encode()).hexdigest()
r=json.load(open('infra/registry.json'))
print(c(r['provider_control'])==c(json.load(open('infra/verticals/provider_enrollment/intent.json'))['desired_state']))"
# §1 divergence
python3 -c "import json;r=json.load(open('infra/registry.json'));print('wireguard' in r['network_policy'])"   # -> False
python3 -c "import json;print('wireguard' in json.load(open('infra/verticals/network/intent.json'))['desired_state'])"  # -> True
# §2 dead trees
for d in apps programs protocols references spaces specs; do
  printf '%-12s %s\n' "$d" "$(grep -rn "$d/" crates/ --include='*.rs' | wc -l)"; done
# §4 unread contracts
for f in contracts/*.json; do printf '%-40s %s\n' "$f" "$(grep -rn "$(basename $f)" crates/ --include='*.rs' | wc -l)"; done
```

---

## 6. What this is NOT

- Not "the tree is disorganised" — the tree has *more* declared authority than
  most repos, and the authority is correct in intent.
- Not a naming problem — `crates/`, `infra/`, `contracts/` are well named.
- Not (yet) a proposal — §1–§5 are measurements. The fix requires an operator
  decision on **which document wins**, and that decision is not mine to make.

The defect is **absence of reconciliation between declared authority and
declared authority**: each document is internally consistent, and they disagree
with each other. `AGENTS.md` ranks them but nothing checks the rank.

---

## 7. Open questions

1. **Verticales** — are `infra/verticals/*/intent.json` intended to be
   generated *from* `registry.json`, or is the registry intended to be a
   projection of the verticals? (Currently neither: they were copy-pasted and
   then drifted.)
2. **Dead trees** — `apps/`, `programs/`, `protocols/`, `references/`,
   `spaces/`, `specs/` have no code reader. Delete, or give them a reader?
3. **`directory-tree.json`** — extend it to cover the 15 omitted top-level
   paths, or reduce the tree to what it covers?
4. **Unread contracts** — 6 of 16 (`release-gates`, `release-domain`,
   `staging-model`, `build-placement`, `observability-loop`,
   `pool-contribution`). Executor missing, or contract dead?
