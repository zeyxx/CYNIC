# Service Discovery Architecture Problem

## Status: OBSERVED — needs structural fix

## Problem

CYNIC uses hardcoded LAN IPs (`192.168.0.25`) to reach services (embedding, reranking, inference, storage). These IPs are not discoverable dynamically. The registry (`infra/registry.json`) declares what should run but not what runs actually.

## Observed evidence

| Service | Port | From cortex | From compute-01 |
|---|---|---|---|
| embedding (embeddinggemma-2-Q8_0) | 8080 | 200 OK | 404 Not Found |
| reranking (Qwen3-Reranker-0.6B) | 8082 | 200 OK | 404 Not Found |
| inference (LFM3-1.7B) | 8088 | 200 OK | 200 OK |

The same service responds differently depending on the source host. This indicates network/firewall/routing issues, not application issues.

## Root cause

1. **Hardcoded IPs**: Code uses `192.168.0.25` instead of service names
2. **Registry as declaration**: `infra/registry.json` declares intent, not observed state
3. **No health probe**: Services are not probed at startup
4. **No fallback**: If a service is unreachable, the code panics instead of degrading gracefully

## Proposed structural fix

### 1. Service names instead of IPs

Replace hardcoded IPs with service names:
- `embedding.service.cynic` instead of `192.168.0.25`
- `reranker.service.cynic` instead of `192.168.0.25`
- `inference.service.cynic` instead of `192.168.0.25`

### 2. DNS/mDNS resolution

Hosts should resolve service names via DNS or mDNS. IPs are observed, not declared.

### 3. Runtime-observed state

Create `infra/runtime-observed.json` that contains:
- Service name
- Host ID (reference to registry)
- Port
- Health status (ok/degraded/down)
- Last probe timestamp
- Observed IP (from health probe)

### 4. Health probe at startup

A mechanism that:
- Reads the registry for expected services
- Probes each health endpoint
- Updates `runtime-observed.json`
- Does not block startup

### 5. Code reads observed state, not declaration

Code should read `runtime-observed.json` for endpoints, not `registry.json` and not hardcoded IPs.

### 6. Graceful degradation

If a service is not observed, the code should:
- Log a warning
- Disable the dependent feature (e.g., no dense search without embedding)
- Continue running

## What exists already

- `infra/registry.json` — declaration of intent (needs IPs replaced with service names)
- `infra/runtime-authority.json` — runtime authority SSOT
- `cynic-discovery` crate — node capability discovery (not service discovery)
- `cynic-inference/backend_verification.rs` — backend verification (could be extended)
- `cynic-host` — host management (could host the probe)

## What needs to be created

1. `infra/runtime-observed.json` — observed state format
2. Service name resolution (DNS/mDNS or hosts file)
3. Health probe in `cynic-host` or new module in `cynic-discovery`
4. Update `infra/registry.json` to use service names instead of IPs
5. Update all code to read `runtime-observed.json` instead of hardcoded IPs

## Files affected

- `crates/cynic-knowledge/src/bin/bench_dense_rerank.rs` — hardcoded URLs
- `crates/cynic-inference/` — backend verification
- `crates/cynic-surreal/` — SurrealDB connection
- `crates/cynic-cli/` — storage surface
- `infra/registry.json` — IPs need to become service names
- `AGENTS.md` — builder SSOT references

## Priority

This is a **structural** problem that affects all services. It should be addressed before adding new features that depend on service discovery (dense RAG, model routing, etc.).
