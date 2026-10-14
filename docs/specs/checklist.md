# CYNIC Operational Checklist

**Schema:** `cynic.checklist.v1`  
**Purpose:** Mandatory procedures for all CYNIC fleet operations on unstable infrastructure.

---

## 1. Git Integrity Protocol (every commit)

| Step | Action | Proof |
|------|--------|-------|
| **1.1 Write** | Hash each modified file | `sha256sum <file>` |
| **1.2 Diff** | Review staged changes | `git diff --stat` before commit |
| **1.3 Tree** | Verify no unauthorized modifications | `git status --short` = 0 unexpected entries |
| **1.4 Commit** | Include proof in message | `git commit -m "<type>(<scope>): <desc>\n\nProof: <X files, +Y/-Z LOC>"` |
| **1.5 Push** | Push immediately after commit | `git push origin <branch>` |
| **1.6 Read-back** | Verify remote SHA matches local | `git ls-remote origin <branch>` |
| **1.7 Diff remote** | Confirm no drift | `git diff --stat origin/main origin/<branch>` (empty = clean) |

### Forbidden

- `git repack -a -d -f` before pushing (crashes on unstable host, writes new corruption)
- `git bundle create --all` from a corrupt repo (produces corrupt bundle)
- `pkill -f memtester` (kills own shell — pattern matches launcher)
- `rm -rf` on any path with a case-variant sibling (check inode first: `stat -c '%d %i'`)
- Running `cargo`/`rustc` locally on cortex (RAM fault — use builder 10.10.10.119)

---

## 2. Binary Build & Deploy Protocol

| Step | Action | Target |
|------|--------|--------|
| **2.1 Sync** | `git pull origin main` | All machines |
| **2.2 Build** | `make check` on builder | 10.10.10.119 |
| **2.3 Build binary** | `cargo build --release` on builder | 10.10.10.119 |
| **2.4 Verify** | `cynic version` matches `main` SHA | All machines |
| **2.5 Install** | `cp target/release/cynic /usr/local/bin/cynic` | Builder, cortex |
| **2.6 Deploy** | Copy to all fleet nodes | PVE CTs, physical nodes |

### SSOT

- **Binary location:** `/usr/local/bin/cynic` (Linux), `/usr/local/bin/cynic` (macOS via brew)
- **Repository path:** `/home/user/Bureau/SolANA/asdf-forge/zeyxx/CYNIC` (canonical, case-sensitive)
- **Builder:** `10.10.10.119` (build/test SSOT)
- **Git remote:** `ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git`

---

## 3. Privilege System Setup

| Step | Action | Proof |
|------|--------|-------|
| **3.1 Catalog** | Verify `infra/capabilities/catalog.json` is current | `cynic privilege catalog --json` |
| **3.2 Install** | Generate capabilities installed file | `cynic privilege install` |
| **3.3 Verify** | `cynic privilege catalog` shows `status: ok` | No blockers |
| **3.4 Admit** | Resolve capability to argv (dry-run) | `cynic privilege admit <capability> --params` |
| **3.5 Execute** | Run with journal | `cynic privilege exec <capability> --params` |

### Required Capabilities (checklist)

| Capability | Domain | Status |
|------------|--------|--------|
| `node-control` | runtime | mandatory |
| `pct-clone` | hypervisor | optional |
| `pct-destroy` | hypervisor | optional |
| `pct-list` | hypervisor | optional |
| `pct-probe-hostname` | hypervisor | optional |
| `pct-resize` | hypervisor | optional |
| `pct-status` | hypervisor | optional |
| `forgejo-inspect-app-ini` | hypervisor | optional |
| `forgejo-inspect-user` | hypervisor | optional |
| `forgejo-machine-token` | identity | optional |
| `systemctl-restart-pve-cluster` | system | optional |
| `systemctl-restart-pvedaemon` | system | optional |
| `apt-install` | system | **TODO: add to catalog** |
| `tmux-session-create` | runtime | **TODO: add to catalog** |

---

## 4. Capability Catalog Additions

### 4.1 `apt-install` (system package installation)

```json
{
  "id": "apt-install",
  "domain": "system",
  "argv_template": [
    "/usr/bin/apt-get",
    "install",
    "-y",
    "{package}"
  ],
  "params": [
    {
      "name": "package",
      "kind": "package_name"
    }
  ],
  "requires_root": true,
  "mandatory": false,
  "purpose": "install a system package via apt (e.g., tmux, htop)"
}
```

### 4.2 `tmux-session-create` (persistent terminal session)

```json
{
  "id": "tmux-session-create",
  "domain": "runtime",
  "argv_template": [
    "/usr/bin/tmux",
    "new-session",
    "-d",
    "-s",
    "{session_name}",
    "{command}"
  ],
  "params": [
    {
      "name": "session_name",
      "kind": "identifier"
    },
    {
      "name": "command",
      "kind": "command"
    }
  ],
  "requires_root": false,
  "mandatory": false,
  "purpose": "create a detached tmux session for long-running agents"
}
```

### 4.3 `tmux-session-observe` (read session state)

```json
{
  "id": "tmux-session-observe",
  "domain": "runtime",
  "argv_template": [
    "/usr/bin/tmux",
    "list-sessions",
    "-F",
    "#{session_name}:#{session_windows}:#{session_attached}"
  ],
  "params": [],
  "requires_root": false,
  "mandatory": false,
  "purpose": "observe tmux sessions (read-only)"
}
```

---

## 5. Wrapper Registry

| Capability | Wraps | Native? | Notes |
|------------|-------|---------|-------|
| `pct-clone` | `/usr/sbin/pct` | Yes (Proxmox CLI) | Wrapper around `pct` binary |
| `systemctl-restart-pve-cluster` | `/usr/bin/systemctl` | Yes | systemd wrapper |
| `apt-install` | `/usr/bin/apt-get` | Yes | APT wrapper |
| `tmux-session-create` | `/usr/bin/tmux` | Yes | tmux wrapper |
| `forgejo-inspect-*` | `/usr/sbin/pct exec` | Yes | Proxmox exec wrapper |
| `cynic build` | `cargo build` | Yes | Rust toolchain wrapper |
| `cynic privilege exec` | various | Yes | CYNIC native dispatcher |

---

## 6. Binary Location SSOT

Create `infra/binary-locations.json`:

```json
{
  "schema": "cynic.binary_locations.v1",
  "binaries": [
    {
      "name": "cynic",
      "canonical_path": "/usr/local/bin/cynic",
      "fallback_paths": [
        "/home/user/.local/bin/cynic"
      ],
      "source": "cargo build --release",
      "target_profile": "release"
    }
  ],
  "machines": [
    {
      "hostname": "cortex",
      "os": "Ubuntu 24.04",
      "binary_path": "/home/user/.local/bin/cynic",
      "status": "outdated"
    },
    {
      "hostname": "builder",
      "os": "Ubuntu 24.04",
      "binary_path": "/usr/local/bin/cynic",
      "status": "current"
    }
  ]
}
```

---

## 7. Session Recovery

After crash or reboot:

```bash
# 1. Restore from origin
git clone ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git \
  /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC

# 2. Verify
git -C /home/user/Bureau/SolANA/asdf-forge/zeyxx/CYNIC log --oneline -1

# 3. Resume tmux sessions
tmux attach -t build
tmux attach -t inference
```

---

## 8. RAM Instability Mitigation

| Symptom | Countermeasure |
|---------|----------------|
| Shell loses git context | Use `git -C <absolute-path>` always |
| Pack corruption | Push before any repack; verify backups by cloning |
| Segfault variance | Count per-boot: `journalctl -k -b N \| grep -c segfault` |
| `memtester` errors | Descend frequency: 3200 → 2933 → 2667 → 2133 → stop |
| Inconsistent file reads | Run `stat` to confirm inode; retry read |

---

## 9. Fleet Machine Inventory

| Machine | Hostname | Role | OS | Binary | Status |
|---------|----------|------|-----|--------|--------|
| Workstation | `cortex` | Dev, inference | Ubuntu 24.04 | `/home/user/.local/bin/cynic` | RAM unstable |
| Builder | `10.10.10.119` | Build, test | Ubuntu 24.04 | `/usr/local/bin/cynic` | Stable |
| PVE host | `pve` | Hypervisor | Proxmox VE 8 | N/A | Stable |
| Forgejo CT | `101` | Git, web | Ubuntu 22.04 (CT) | N/A | Stable |
| Builder CT | `105` | Build container | Ubuntu 24.04 (CT) | N/A | Stable |
| Embedding CT | `106` | Dense embeddings (EmbeddingGemma-2, DIM 768) | Debian 12 (CT) | `llama-server-embed` + fallback | Stable, systemd + enabled |

---

## 10. Post-Commit Verification

After every `git push`:

```bash
# 1. Remote SHA matches local
git ls-remote origin <branch> | awk '{print $1}'
git rev-parse HEAD

# 2. No drift between branches
git diff --stat origin/main origin/<branch>

# 3. Build still green (on builder)
ssh root@10.10.10.119 "cd /var/lib/cynic/source/repositories/CYNIC && make check"
```

---

## 11. Organic Sovereign RAG Execution Checklist

Tracked under `docs/specs/ORGANIC_RAG_BOTTOM_UP_SPEC.md` (`cynic.spec.organic_rag.v1`):

- [x] **11.1 Substrate Ingestion & Cache**: Persistent cache (`corpus_cache.json`) + Git-diff incremental ingestion.
- [x] **11.2 Rust Code AST Ingestion**: Extract structs, enums, functions from `crates/*/src` into knowledge matrix.
- [x] **11.3 Graph-RAG Traversal**: Link chunks to parent contracts via `KnowledgeGraph` dependency edges.
- [x] **11.4 Matrix Re-Ranking**: Score chunks by BM25 × Authority Boost × Layer Boost × Git Freshness.
- [x] **11.5 Hybrid Retrieval (RRF)**: Reciprocal Rank Fusion combining BM25 and dense local embeddings (dense = stub vectors in unit tests until 2026-10-13; live 768d endpoint since, see 11.8).
- [ ] **11.6 MCP Native Exposure**: Wire `cynic_know_ask` into `cynic-mcp` with zero shell-fork overhead.
- [x] **11.7 Epistemic CI Evaluation Harness**: Automated 10-question evaluation matrix with 0% hallucination gate.
- [x] **11.8 Live Dense Endpoint (2026-10-13)**: `embeddinggemma-2-bf16` served by llama-server on `cynic-lxc-embedding` (lxc/106, `10.10.10.106:8080`, DIM 768, systemd unit `llama-server-embedding.service`); declared in `infra/registry.json`; live smoke test `cynic-inference/tests/embedding_live.rs` (ignored by default, `dims=768`, 71 ms builder→106).
- [x] **11.9 Chunk Vectorizer + Cosine Index**: `vectorize_chunks` +
  `top_k_dense` + `fuse_reciprocal_ranks` live in `cynic-knowledge`
  (unit-covered); neural consumption via `SyncEmbeddingBridge` (merged).
  Backend fans WIDTH concurrent sub-lots of 8 (`CYNIC_EMBED_WIDTH`, default
  4) across server slots. No cached-vector pipeline yet — vectors are
  computed per run.
- [ ] **11.10 Retrieval Gain Measurement**: graded labels exist
  (`retrieval_gain_labels.json`: 10 pairs / 1 trap / 9 scored). Measured
  2026-10-14: BM25 MRR 0.222, hashed-stub hybrid 0.133 (delta -0.089).
  Neural run in flight against cortex overflow server (ubatch 2048).
- [ ] **11.11 Embedding Server Calibration**: the request cap is the
  **ubatch**, not the batch (observed live: ubatch 512 rejects 541-token
  inputs). LXC 106 (operator):
  ```bash
  systemctl edit --full llama-server-embedding.service
  # set: --batch-size 2048 --ubatch-size 2048 (watch container RAM)
  systemctl restart llama-server-embedding.service
  curl -s http://10.10.10.106:8080/health
  ```
  Cortex overflow (this host): master build + Vulkan, `-c 4096 --batch-size
  2048 --ubatch-size 2048 -t 12 -fa on -ngl 99 --parallel 4`, unit
  `cynic-embedding-cortex.service` (MemoryMax=3G).
  Client levers (no recompile): `CYNIC_EMBED_LOT_SIZE` (default 32, clamp
  1..128), `CYNIC_EMBED_WIDTH` (default 4, clamp 1..8). Sweep matrix:
  server ubatch {512, 2048, 4096} x client lot {8, 32, 64}; keep the
  fastest `EMBED done` seconds from the live test.

---

## 12. Builder Retention GC (opened 2026-10-14)

Census: `checkouts` 40G → 29G (9 dirs, ~11G reclaimed 2026-10-14).
`/tmp` 15G = same-day sibling clones (hands-off). `shared-target` 19G,
`source` 15G, `build-targets` 6.7G, `target` 4.3G = warm build caches
(load-bearing: deleting them only slows the next builds, saves nothing durable).

Policy: `contracts/builder-retention.json` (TTL 7d per-SHA/session checkouts,
2d `/tmp`; fence: HEAD < 48h, live claims, `/var/lib/cynic/source`, sccache).
Test `cynic-cli/tests/builder_retention.rs` green on builder.

Verdict: do NOT clean the rest — remainder is live or load-bearing. Next pass
when TTLs mature. Self-cleaning conditions = the contract above, nothing else.

### Pending privileged actions (traced, not done)

- **PENDING ROOT** (cynic has no sudo on builder): `rm -rf
  /var/lib/cynic/checkouts/cynic-65381e765497
  /var/lib/cynic/checkouts/cynic-92da50e51cdd` — 3.5G, 7-9 days old, no claim,
  reproducible-or-provenanceless. Root cause: builds ran as root on the builder
  (16 011 root-owned files); every future purge blocks the same way until builds
  run as `cynic`.
- **PENDING CREDENTIAL**: github relay keeps 38 dead agent refs (read-only
  access from this host). `main` SHAs in sync origin/github/local — content-safe.

---
**End of checklist.**
