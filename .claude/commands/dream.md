---
name: dream
description: Memory consolidation — 4-phase cleanup of accumulated session memories. Run when suggested by session-init or manually when memory feels stale.
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Agent
---

# /dream — Memory Consolidation

You are performing a dream consolidation on the memory system. This is a mechanical process with 4 phases. Follow them exactly.

Memory directory: `~/.claude/projects/-home-user-Bureau-CYNIC/memory/`

## Phase 1: Orient

1. Read `MEMORY.md` — understand current index structure
2. List all `*.md` files in the memory directory
3. Identify: orphans (on disk but not in index), broken links (in index but missing file)

## Phase 2: Gather Signal

For each memory file:
1. Read the file
2. Classify: **keep** (still relevant), **merge** (overlaps with another), **delete** (stale, superseded, already in .claude/rules/)
3. Check for: relative dates (convert to absolute), contradictions between files, references to deleted code/files

**Rules for classification:**
- Session logs where all work is committed → delete (the code IS the record)
- Feedback that restates a rule in `.claude/rules/` without adding WHY → delete
- Project state that's superseded by a newer file → delete
- Anything with only ephemeral value (handoff notes, task lists) → delete

## Phase 3: Consolidate

1. Merge files that cover the same topic (keep the densest version)
2. Convert all relative dates to absolute dates (relative to today)
3. Remove contradicted facts at source
4. Each surviving file must earn its place: does it contain knowledge NOT derivable from code, git, or rules?

## Phase 4: Prune & Index

1. Rewrite `MEMORY.md` as a lean index (target: under 80 lines, hard limit: 200)
2. Format: `- [name](file.md) — one-line description`
3. Organize by type (User, Feedback, Project, Reference), not chronologically
4. Zero orphans, zero broken links

## After Completion

Update the dream state file:
```bash
STATE_FILE="$HOME/.claude/projects/-home-user-Bureau-CYNIC/memory/.dream-state"
echo "last_dream=$(date -Iseconds)" > "$STATE_FILE"
echo "sessions_since=0" >> "$STATE_FILE"
```

Report: files before → after, MEMORY.md lines before → after, what was deleted/merged.
