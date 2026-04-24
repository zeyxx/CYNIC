# CYNIC — System Status (Source of Truth)

> Last probed: 2026-04-24T19:47 CEST | Commit: `fcd613f` | Binary: `v0.7.7-215-g9a38673`

This file is the **observed** state of the organism — not goals, not design, not aspirations. Every line was verified by live probe. Update after every deploy.

---

## Kernel

| Field | Value |
|-------|-------|
| Version | `v0.7.7-215-g9a38673` |
| Status | `degraded` (soma: wild binds detected) |
| Uptime | 299s at last probe |
| Storage | `connected` (SurrealDB, localhost:8000) |
| Embedding | `sovereign` (Qwen3-Embed 0.6B, localhost:8081) |
| Total requests | 2314 (lifetime) |
| Crystals | 31 forming, 0 crystallized, loop_active=false |

## Dogs

| Dog | Circuit | Sovereign? | Notes |
|-----|---------|-----------|-------|
| deterministic-dog | closed | Yes | Heuristic, always works |
| qwen35-9b-gpu | closed | Partial | RTX 4060 Ti via Tailscale, 55 tok/s |
| qwen-7b-hf | closed | No | HuggingFace API |
| gemma-4-e4b-core | closed | Yes | CPU+Vulkan iGPU, thinking overflow issue |
| gemini-cli | **critical** | No | Google subscription, currently down |

Healthy: 4/5 | Sovereign: 2/5 | Quorum: functional (3 needed)

## Probes

| Probe | Status | Interval |
|-------|--------|----------|
| resource | ok | 30s |
| process | ok | 30s |
| pressure | ok | 30s |
| network | ok | 30s |
| soma | **degraded** | 30s |
| fleet | ok | 30s |

Soma degraded because 9 wild binds detected (0.0.0.0). Known legitimate: WireGuard (41641), mDNS (5353), DHCP (67), RustDesk (21119). Needs whitelist to distinguish.

## Network

| Surface | State |
|---------|-------|
| Tailscale Funnel | **ON** → `https://<HOST>.tail7ec70e.ts.net` → :3030 |
| Firewall (nftables) | **Active** — 15 rules, default DROP inbound |
| `/health` (no auth) | Returns `status` + `phi_max` only. No version. |
| `/metrics` (no auth) | **401** — auth required (KC3 fix) |
| `/events` (no auth) | **401** — auth required (KC3 fix) |
| RustDesk | Running, 0.0.0.0, public relay rs-ny — **TEMPORARY** |
| mitmdump | **Killed** — restart via `hermes-proxy.service` when needed |

## Services (systemd --user)

| Service | State |
|---------|-------|
| cynic-kernel.service | **active** |
| surrealdb.service | active (inferred — storage=connected) |
| llama-server.service (Gemma) | active (inferred — gemma circuit=closed) |
| llama-embed.service (Qwen-embed) | active (inferred — embedding=sovereign) |
| cynic-firewall.service | **active** (nftables applied on boot) |
| hermes-proxy.service | **disabled** (installed, not started) |

## Git

| Field | Value |
|-------|-------|
| Branch | `main` |
| HEAD | `fcd613f` |
| Origin | synced (PRs #23 + #24 merged) |
| Open PRs | 0 |
| Branches | 1 (main) |
| Stashes | 0 |
| Worktrees | 1 (principal) |

## Known Gaps (observed, not aspirational)

| ID | Gap | Severity |
|----|-----|----------|
| KC1 | No disk encryption (LUKS) | Critical |
| KC2 | Tailscale control plane = Tailscale Inc (no Headscale) | High |
| KC4 | RustDesk on 0.0.0.0 + public relay | Medium |
| KC5 | DNS = ISP router (no self-hosted resolver) | Medium |
| KC6 | GPU node = Windows + different Tailscale user | Medium |
| SOMA-1 | Soma wild_bind whitelist missing → false degraded | Low |
| SOMA-2 | Firewall rule count = 1 (parsing bug, actual 15) | Low |
| PROBE-1 | ProbeScheduler replaces snapshot per tick (no merge) | Low |
| CRYSTAL | Crystal loop inactive (loop_active=false, 31 forming, 0 crystallized) | Medium |
| GEMINI | gemini-cli circuit=critical (quota or connectivity) | Medium |
