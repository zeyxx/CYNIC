# Session: Workflow — Make the Tooling Work

## Problème

Le workflow Claude Code est du hope engineering. Les rules CLAUDE.md sont violées sans détection. 33 docs à plat dans docs/ sans structure. .claude/rules/ vide. .claude/agents/ n'existe pas. Les hooks couvrent 4/21 events disponibles. Diagnostiqué le 2026-03-24, jamais implémenté.

## Lire avant de commencer

- `.claude/settings.local.json` — hooks et permissions actuels
- `.claude/hooks/*.sh` — les 4 hooks existants (session-init, protect-files, observe-tool, session-stop)
- Memory: `project_workflow_infrastructure_research.md` — le design du 2026-03-24 (référence projets showcase)

## Livrables

1. **docs/ réorganisé** — sous-dossiers (audit/, research/, architecture/, design/, sessions/, archive/). Chaque doc dans le bon dossier. git mv, pas cp.
2. **CLAUDE.md < 200 lignes** — le reste migré dans .claude/rules/ (rules par scope: kernel, workflow, build)
3. **.claude/rules/** créé — rules extraites, scoped par path pattern
4. **.claude/agents/** créé — minimum: code-reviewer (Rust-specific, clippy, Rule violations)
5. **Hook PostToolUse(Edit|Write .rs)** → `cargo check` (feedback rapide après edit Rust)
6. **Hook PreToolUse(git commit)** → `cargo test + clippy` gate (bloque le commit si fail)
7. **Hook Stop** → vérifie `git status --short` = 0 modifiés (Rule #30 mécanique)
8. **cargo audit en hard gate** dans make check (pas conditionnel)
9. **Permissions allow list** nettoyée — supprimer les one-shots accumulés
10. `git status --short` = 0 en fin de session

## Ne PAS faire

- Toucher au kernel Rust (cynic-kernel/src/)
- Rechercher des patterns architecturaux (c'est fait)
- Designer des fondations épistémiques (session suivante)
- Installer des outils externes (claude-mem, Lore — session self-learning)
