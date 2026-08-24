# CYNIC Theme Topology Policy

Status: normative foundation.

Machine contract: `axioms/organism-theme-topology-v1.json`.

This policy defines thematic ownership for CYNIC/TALARIA/Odysseum material. It
exists because the repo contains public docs, private drafts, historical
submissions, generated topology reports, packages and curated datasets. Those
surfaces are useful, but they must not become competing sources of truth.

## 1. Core Rule

Every durable organism claim belongs to one theme:

| Theme | Meaning | Canonical layer |
|-------|---------|-----------------|
| `talaria_legal_genesis` | TALARIA SAS legal identity, ownership and statutory route | `docs/foundation/` + legal primary sources |
| `cynic_protocol_framework` | CYNIC framework, protocol, token firewall, E-Score and revenue semantics | `docs/foundation/` |
| `odysseum_futardio_legacy` | Historical Odysseum/Futard.io evidence and incubation narrative | `docs/internal/` until promoted |
| `organism_runtime_topology` | Repo, organs, registry, MCP, coordination and runtime placement | `axioms/`, `infra/`, manifests |
| `data_and_evidence` | Curated datasets, fixtures, reports and observations | `data/`, fixtures, Kernel observations |

A document outside the canonical layer may summarize or package a theme. It may
not redefine the theme.

## 2. Document Zones

| Zone | Role | Canonicality |
|------|------|--------------|
| `docs/foundation/` | Human-readable doctrine and policy | canonical when indexed |
| `axioms/` | Machine-readable contracts | canonical by `schema` and `status` |
| `docs/architecture/` | Current/target implementation maps | projection unless referenced by contract |
| `docs/internal/` | Private drafts, historical context, communication material | non-canonical evidence |
| `docs/hackathon/` | External submission packaging | projection only |
| `data/` | Curated data and reports | canonical only for declared curated datasets |
| `packages/` | Product/UI projection | projection only |

## 3. Promotion Rule

Material from `docs/internal/`, `docs/hackathon/`, `docs/research/`,
`docs/reference/` or `data/reports/` becomes canonical only when:

1. the target theme is declared;
2. the canonical foundation, contract, registry/exposure or organ manifest file
   is updated;
3. the new rule preserves the theme drift rule;
4. machine-readable concerns gain validation.

Historical Odysseum/Futard.io material is evidence. TALARIA and CYNIC canon is
not changed by editing a pitch, submission, package, generated report or private
draft alone.

## 4. Enforcement

`make lint-repo-topology-contract` validates the machine contract shape and
checks that declared canonical sources exist. The gate is deliberately narrow:
it prevents missing authorities and theme/source drift without forcing a
dangerous mass move of historical documents.
