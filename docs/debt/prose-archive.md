# Prose Archive Registry — CYNIC

**Critère** : Fichiers .md narratifs, redondants avec contracts/, ou prose non-actionnable
**Référence** : AGENTS.md "Deleted authorities: axioms/, organs/, docs/ as runtime law"
**Action** : Déplacer vers `docs/archive/` — garder trace git, supprimer du runtime

---

## À ARCHIVER (15 fichiers)

### PROSE-001: AGENTS.md (10,500 chars, root)
- **Contenu** : Runtime law, agent coordination, structural invariants, execution standard
- **Redondance** : `contracts/foundation.json`, `contracts/framework.json`, `infra/runtime-authority.json`, `infra/agent-roles.json`
- **Action** : `mv AGENTS.md docs/archive/AGENTS.md`
- **Status** : `observed`

### PROSE-002: DOMAIN_GLOSSARY.md (6,040 chars, root)
- **Contenu** : Vocabulaire FR/EN, définitions termes
- **Redondance** : `contracts/framework.json` contient tous les termes typés
- **Action** : `mv DOMAIN_GLOSSARY.md docs/archive/DOMAIN_GLOSSARY.md`
- **Status** : `observed`

### PROSE-003: CYNIC_ORGANISM_PRIMITIVES.md (4,100 chars, root)
- **Contenu** : Primitives organisme, mix FR/EN, duplicate concepts
- **Redondance** : `contracts/framework.json` + `cynic-protocol/src/framework.rs`
- **Action** : `mv CYNIC_ORGANISM_PRIMITIVES.md docs/archive/CYNIC_ORGANISM_PRIMITIVES.md`
- **Status** : `observed`

### PROSE-004: AUDIT_CAP_FEDERATION.md (19,399 chars, root)
- **Contenu** : Audit technique détaillé, findings, recommandations
- **Problème** : Findings techniques → appartiennent en CODE/TESTS, pas en prose
- **Action** : Extraire findings → `docs/debt/` (déjà fait), archiver le reste
- **Status** : `observed` (partiellement migré vers debt/)

### PROSE-005: MANIFESTO.md (root)
- **Contenu** : Narrative projection publique
- **Référence** : AGENTS.md "Public narrative projections live in MANIFESTO.md... subordinate to contracts"
- **Action** : Garder (projection publique déclarée) MAIS marquer non-authoritative
- **Status** : `keep_marked`

### PROSE-006: WHITEPAPER.md (root)
- **Contenu** : Projection narrative technique
- **Référence** : AGENTS.md "subordinate to contracts and runtime proof"
- **Action** : Garder (projection publique déclarée) MAIS marquer non-authoritative
- **Status** : `keep_marked`

### PROSE-007: docs/ (directory entier)
- **Contenu** : Documentation historique, guides, specs anciennes
- **Redondance** : Superseded par contracts/ + infra/ + code
- **Action** : `mv docs/ docs/archive/docs_legacy/`
- **Status** : `observed`

### PROSE-008: veilo_corpus_docs_inventory.md (root)
- **Contenu** : Inventaire docs Veilo (projet précédent ?)
- **Pertinence** : Hors scope CYNIC actuel
- **Action** : `mv veilo_corpus_docs_inventory.md docs/archive/veilo_corpus_docs_inventory.md`
- **Status** : `observed`

### PROSE-009: veilo_framework_architecture.md (root)
- **Contenu** : Architecture Veilo
- **Pertinence** : Hors scope CYNIC actuel
- **Action** : `mv veilo_framework_architecture.md docs/archive/veilo_framework_architecture.md`
- **Status** : `observed`

### PROSE-010: veilo_governance_model.md (root)
- **Contenu** : Modèle gouvernance Veilo
- **Pertinence** : Hors scope CYNIC actuel
- **Action** : `mv veilo_governance_model.md docs/archive/veilo_governance_model.md`
- **Status** : `observed`

### PROSE-011: veilo_operational_playbook.md (root)
- **Contenu** : Playbook opérationnel Veilo
- **Pertinence** : Hors scope CYNIC actuel
- **Action** : `mv veilo_operational_playbook.md docs/archive/veilo_operational_playbook.md`
- **Status** : `observed`

### PROSE-012: veilo_tokenomics.md (root)
- **Contenu** : Tokenomics Veilo
- **Pertinence** : Hors scope CYNIC actuel
- **Action** : `mv veilo_tokenomics.md docs/archive/veilo_tokenomics.md`
- **Status** : `observed`

### PROSE-013: CODEX.md (root)
- **Contenu** : Config/notes pour Codex agent
- **Pertinence** : Agent projection, non-authoritative
- **Action** : `mv CODEX.md docs/archive/CODEX.md`
- **Status** : `observed`

### PROSE-014: .hermes/plans/ (directory)
- **Contenu** : Plans Hermes sessions précédentes
- **Pertinence** : Historique sessions, pas runtime law
- **Action** : Garder dans `.hermes/` (Hermes managed), ne pas archiver
- **Status** : `keep_hermes`

### PROSE-015: plans/ (directory root)
- **Contenu** : Plans divers
- **Pertinence** : Vérifier contenu
- **Action** : Audit puis archive si legacy
- **Status** : `to_audit`

---

## À GARDER (authoritative projections)

| Fichier | Raison | Marqueur |
|---------|--------|----------|
| `MANIFESTO.md` | Public narrative projection déclarée | Add header: `> ⚠️ NON-AUTHORITATIVE — see contracts/foundation.json` |
| `WHITEPAPER.md` | Public narrative projection déclarée | Add header: `> ⚠️ NON-AUTHORITATIVE — see contracts/framework.json` |

---

## Script d'archivage

```bash
#!/bin/bash
# archive_prose.sh — exécuter depuis repo root

mkdir -p docs/archive

# Archive prose noise
mv AGENTS.md docs/archive/
mv DOMAIN_GLOSSARY.md docs/archive/
mv CYNIC_ORGANISM_PRIMITIVES.md docs/archive/
mv AUDIT_CAP_FEDERATION.md docs/archive/
mv docs docs/archive/docs_legacy
mv veilo_corpus_docs_inventory.md docs/archive/
mv veilo_framework_architecture.md docs/archive/
mv veilo_governance_model.md docs/archive/
mv veilo_operational_playbook.md docs/archive/
mv veilo_tokenomics.md docs/archive/
mv CODEX.md docs/archive/

# Mark non-authoritative (non-destructif)
echo -e "> ⚠️ NON-AUTHORITATIVE — see contracts/foundation.json\n$(cat MANIFESTO.md)" > MANIFESTO.md.tmp && mv MANIFESTO.md.tmp MANIFESTO.md
echo -e "> ⚠️ NON-AUTHORITATIVE — see contracts/framework.json\n$(cat WHITEPAPER.md)" > WHITEPAPER.md.tmp && mv WHITEPAPER.md.tmp WHITEPAPER.md

echo "Archive complete. Run: git add docs/archive/ && git commit -m 'chore: archive prose noise'"
```

---

## Vérification post-archive

```bash
# Vérifier qu'aucun fichier .md non-archivé ne contient "runtime law" ou "authority"
grep -r "runtime law\|authority\|canonical" --include="*.md" . | grep -v "docs/archive" | grep -v "MANIFESTO.md" | grep -v "WHITEPAPER.md"

# Doit retourner vide
```

---

## Priorisation

| Entrée | Criticité | Effort | ROI | Ordre |
|--------|-----------|--------|-----|-------|
| PROSE-001 à PROSE-013 | LOW | 0.5j | 3/10 | P3-1 (batch unique) |
| PROSE-015 (plans/) | LOW | 0.5j | 2/10 | P3-2 |