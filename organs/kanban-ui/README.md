# Organ Obsidian (organ-obsidian)

**Role**: `dashboard_projection_manager`

This organ serves as a one-way projector. It reads the internal machine state of Hermes Kanban (via `hermes kanban ls --json`) and projects it into a human-readable Obsidian Kanban board (`data/obsidian/kanban.md`).

This adheres to the `JSON is King` principle: the actual state machine is in SQLite/JSON, while Markdown is only used as a visualization dashboard for humans.

## Entrypoint
`services/cynic-python/organs/obsidian/projector.py`
