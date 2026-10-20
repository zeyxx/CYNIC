# cynic-evidence

Append-only evidence log with hash-chained entries and verification.

Domain pure: no I/O, no chrono, no FS. Types, hashes, verification.
Adapters live in other crates (kernel, ...).

Invariants:
- Every entry has a content-addressed hash (SHA-256 of canonical JSON).
- Hash chain: entry N includes hash(N-1) — tamper-evident.
- No deletion, no mutation, no silent overwrite.

## Types

| Type | Purpose |
|------|---------|
| `EvidenceEntry` | One immutable, content-addressed, hash-chained entry |
| `EvidenceLog` | Append-only chain of entries |
| `EvidenceKind` | Entry category: court audit, forensics, release proof, agent evidence, custom |
| `EvidenceWriter` | Port for append-only writers (implemented by adapters) |

## Schemas

- Log: `cynic.evidence.v1`
- Entry: `cynic.evidence_entry.v1`

```bash
cargo test -p cynic-evidence
```
