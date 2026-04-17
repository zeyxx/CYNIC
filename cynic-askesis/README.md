# Askesis — 3rd CYNIC Pillar

> *The human augmentation layer. Training through exercise. Discipline through reflection.*

Askesis is the third pillar of CYNIC's sovereign infrastructure: grounded audit of human practice through structured logging, reflection, and feedback.

## What Is Askesis?

Askesis (from Greek: *ἄσκησις* — training, exercise, discipline) is a philosophical practice of rigorous self-examination and improvement through repeated, intentional effort. In CYNIC, it serves as the bridge between *kernel* (infrastructure) and *human* (reasoning):

- **Kernel** = what the machine observes and judges
- **Human** = why the human acts as they do
- **Askesis** = the discipline of making that acting visible, measurable, and improvable

## Phase 1: CLI Skeleton

Current release: **v0.1.0** (Phase 1 CLI operational, integration points ready)

### CLI Usage

```bash
# Audit a session log and produce reflection
cynic-askesis audit <logfile>
```

**Input:** JSONL log file (see `LogStore` trait for schema)
**Output:** Structured reflection with verdict (HOWL, WAG, GROWL, BARK, DEGRADED)

### Example

```bash
$ cynic-askesis audit ~/.logs/2026-04-17.jsonl

Auditing log: /home/user/.logs/2026-04-17.jsonl

=== REFLECTION ===
{
  "verdict": "WAG",
  "prose": "CLI skeleton operational — Phase 1 ready for integration",
  "patterns_detected": ["cli-ready"],
  "kenosis_candidate": null,
  "confidence": 0.618
}

=== MARKDOWN ===
# Weekly Reflection — WAG

**Confidence:** 0.618 (φ⁻¹ bounded)

## Prose

CLI skeleton operational — Phase 1 ready for integration
```

## Architecture

### Core Types

| Type | Purpose |
|------|---------|
| `LogEntry` | Single timestamped event in a session log |
| `LogStore` trait | Read/write interface for log backends (JSONL, SurrealDB, etc.) |
| `AuditEngine` | Produces `Reflection` from a log |
| `Reflection` | Structured verdict: prose, patterns, confidence (φ⁻¹ bounded) |
| `AnchorProvider` trait | Time anchors for session boundaries (OAuth timestamps, calendar events) |
| `DomainTracker` trait | Domain-specific prompt and question sets (Phase 2) |

### Module Structure

```
src/
├── error.rs          # AskesisError type
├── log.rs            # LogEntry + LogStore trait
├── log/jsonl.rs      # JSONL implementation
├── audit.rs          # AuditEngine
├── reflection.rs     # Reflection + Verdict types
├── anchor.rs         # AnchorProvider trait + implementations
├── domains/mod.rs    # DomainTracker trait + Phase 1 registry (empty)
└── main.rs           # CLI entry point
```

## Development Status

### Phase 1 (Current) — CLI Skeleton

- ✅ Error type (`AskesisError`)
- ✅ LogEntry + LogStore trait + JSONL backend
- ✅ AuditEngine (mock, returns static reflection)
- ✅ Reflection + Verdict types with markdown rendering
- ✅ AnchorProvider trait (OAuth2, Calendar)
- ✅ DomainTracker trait + empty Phase 1 registry
- ✅ CLI with `audit` subcommand
- 🔄 **Next:** Phase 2 integration (wire LogStore, real DomainTracker)

### Phase 2 — Real Audit Pipeline

- Load logs from multiple backends
- Run domain-specific audits
- Aggregate feedback into weekly reflection
- Output to markdown (weekly-reflection.md)

### Phase 3 — Feedback Loop

- Crystal injection into kernel (known patterns → prompt enhancements)
- Human authorization (review before system action)
- Metrics: compliance trend, pattern convergence

## Building

**Requires Rust 1.94.0+ with RUST_MIN_STACK workaround:**

```bash
export RUST_MIN_STACK=67108864
cargo build --release
```

## Testing

```bash
export RUST_MIN_STACK=67108864
cargo test -p cynic-askesis
```

## Integration Points (Phase 2)

- **Kernel → Askesis:** `/observe` endpoint to log decisions
- **Askesis → Kernel:** Crystal injection via `/crystal` (user-reviewed patterns)
- **Gemini → Askesis:** Subprocess audit engine (complex judgment)
- **Human ↔ Askesis:** Weekly reflection review & editing (markdown file)

## Philosophy

See [`CLAUDE.md`](../CLAUDE.md) — Askesis embodies the **SOVEREIGNTY** axiom:
> Preserves agency and freedom? Does the system augment human judgment without replacing it?

Askesis does not automate decision-making. It structures observation, surfaces patterns, and invites reflection—leaving *choice* with the human.

---

**Architecture Ref:** [`cynic-askesis/src/lib.rs`](./src/lib.rs)  
**Philosophy Ref:** [`docs/identity/CYNIC-PERENNIAL-EPISTEMOLOGY.md`](../docs/identity/CYNIC-PERENNIAL-EPISTEMOLOGY.md)
