# External Tool Capability Registry

## Purpose

CYNIC integrates external tools (llama-server, systemd, docker, CUDA toolkit,
kubectl, …) whose **actual capabilities** are not reliably self-declared via
their CLI. This document establishes the canonical pattern for observing,
caching, and consuming those capabilities — instead of hardcoding assumptions
in Rust source.

## Problem

| Tool | Exposes version | Exposes capabilities | Discovery method |
|------|---------------|----------------------|------------------|
| llama-server | `--version` | ❌ arches, backends | Try + observe exit |
| systemd | `--version` | ❌ unit types, features | Try + observe |
| docker | `version` | ❌ storage, buildkit | Dry-run + parse |
| rustc | `--version` | ✅ `--print cfg` | Honest outlier |
| kernel | `uname -a` | ❌ compile config | `/proc/config.gz` (often absent) |

Low-level tools follow EAFP (Easier to Ask Forgiveness than Permission). For an
orchestrator, EAFP is expensive — loading 14 GiB to test if `qwen35moe` loads
is not acceptable.

## Law

**Every external tool integrated into CYNIC must have an observed capability
entry with invalidation.** No hardcoded allowlist may gate a launch without a
fallback observation path.

## Canonical Pattern

```
desired: use tool capability X
  → observe: what does the deployed tool actually support?
  → reconcile: is X ∈ observed capabilities?
  → bounded action: proceed with proof
  → read-back: did it work? update observation
```

### Data Model

```jsonc
{
  "schema": "cynic.external_tool_capability.v1",
  "tool": "llama-server",
  "bin_path": "/usr/local/bin/llama-server",
  "bin_fingerprint": "size:12345678:mtime:1724000000",
  "last_probed_at_unix_s": 1724000000,
  "version": "b9600",
  "capabilities": {
    "arches": {
      "llama": "observed_ok",
      "qwen35moe": "observed_ok",
      "dspark": "observed_fail"
    },
    "backends": ["cpu", "cuda"]
  }
}
```

### Invalidation

Re-probe when `bin_fingerprint` changes (size + mtime, same heuristic as
`make`). A new binary = unknown capabilities until first observation.

### Decision Rules

| Source | Confidence | Action |
|--------|-----------|--------|
| `observed_ok` | High | Allow silently |
| `observed_fail` | High | Reject with proof |
| In fallback static list only | Low | Allow with warning "unverified" |
| Unknown | None | Reject — "never observed, launch required" |

### Storage

- SSOT: JSON under `principal_paths().cache_dir/engine_capabilities.json`
- Schema-validated on load
- Updated atomically (write temp + rename)

## Replication Checklist

When adding a new external tool to CYNIC:

1. Add a `*_capability_store.rs` module under `adapters/local/`
2. Define a struct with `bin_fingerprint`, `version`, `capabilities` map
3. Implement `load()`, `save()`, `invalidate_if_stale()`
4. Implement `observe_from_outcome(capability, result)` for a posteriori learning
5. Gate decisions on `is_observed()` not on hardcoded lists
6. Register the tool in the table below

## Registered Tools

| Tool | Module | Status |
|------|--------|--------|
| llama-server | `adapters/local/engine_capability_store.rs` | active |
| systemd | — | pending |
| docker | — | pending |
