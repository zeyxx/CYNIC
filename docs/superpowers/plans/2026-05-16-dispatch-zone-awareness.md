# Dispatch Zone Awareness — Data-Centric Implementation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Real-time zone activity visibility derived from existing observation stream. No new state, no locks, no TTL. Query-time freshness interpretation.

**Architecture:** The kernel loads `zones.json` at boot, enriches observations with zone info, and exposes `GET /dispatch/zone-activity` — a read-only query over existing observations. The hook becomes a thin GET client that interprets freshness locally. Zero write path, zero lifecycle management.

**Tech Stack:** Rust (axum), existing observations table in SurrealDB, zones.json (static config)

---

## Design Principles

1. **No new write path** — observe-tool.sh already writes every edit to `/observe`. That IS the heartbeat.
2. **No claim/release lifecycle** — activity is derived from observations, not declared.
3. **No expiry task** — freshness is computed at query-time, not managed by a background sweep.
4. **Advisory only** — kernel reports data, hook/agent interprets and decides.
5. **Zone resolution at query-time** — kernel resolves file→zone when queried, not when stored.

## What changes

| Component | Before | After |
|-----------|--------|-------|
| Zone resolution | Hook reads `zones.json` via jq | Kernel loads at boot, resolves on query |
| Zone state | `/tmp/cynic-zones/*.claimed` files | None — derived from observation stream |
| Liveness | Hook checks kernel then file mtime | Query-time: "last observation in zone by other agent" |
| Hook logic | 90 lines bash (parse, zone lookup, file lock, kernel claim) | 20 lines bash (GET, interpret freshness, warn or pass) |
| Conflict model | Hard BLOCK | Advisory WARN with activity data |

## What does NOT change

- `observe-tool.sh` — already posts Edit/Write/Bash observations with agent_id + file path
- SurrealDB `observations` table — already stores all tool activity
- `zones.json` — stays git-tracked, same format
- `/coord/*` endpoints — unchanged, parallel system (for file-level claims if needed later)

## File structure

| File | Action | Responsibility |
|------|--------|---------------|
| `cynic-kernel/src/domain/zones.rs` | CREATE | Zone config type + file-to-zone resolver |
| `cynic-kernel/src/domain/mod.rs` | MODIFY | Add `pub mod zones;` |
| `cynic-kernel/src/api/rest/dispatch.rs` | CREATE | `GET /dispatch/zone-activity` handler |
| `cynic-kernel/src/api/rest/mod.rs` | MODIFY | Wire new route + `pub mod dispatch;` |
| `cynic-kernel/src/main.rs` | MODIFY | Load zones.json at boot, inject into AppState |
| `cynic-kernel/src/api/rest/types.rs` | MODIFY | Add `zones` field to AppState |
| `cynic-kernel/tests/integration_zone_activity.rs` | CREATE | Integration tests |
| `.cortex/mcp/coord-claim.sh` | REWRITE | Thin GET client |

---

### Task 1: Zone config type + resolver

**Files:**
- Create: `cynic-kernel/src/domain/zones.rs`
- Modify: `cynic-kernel/src/domain/mod.rs` (add `pub mod zones;`)

- [ ] **Step 1: Write the failing test**

```rust
// In cynic-kernel/src/domain/zones.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_file_to_zone() {
        let config = ZoneConfig::from_json(r#"{
            "zones": {
                "api": { "paths": ["cynic-kernel/src/api/"], "description": "API" },
                "scripts": { "paths": ["scripts/", "infra/"], "description": "Scripts" }
            }
        }"#).unwrap();

        assert_eq!(config.resolve("cynic-kernel/src/api/rest/coord.rs"), Some("api".to_string()));
        assert_eq!(config.resolve("scripts/config-sync.sh"), Some("scripts".to_string()));
        assert_eq!(config.resolve("infra/systemd/foo.service"), Some("scripts".to_string()));
        assert_eq!(config.resolve("README.md"), None);
    }

    #[test]
    fn longest_prefix_wins() {
        let config = ZoneConfig::from_json(r#"{
            "zones": {
                "domain-core": { "paths": ["cynic-kernel/src/domain/"] },
                "domain-ccm": { "paths": ["cynic-kernel/src/domain/ccm/"] }
            }
        }"#).unwrap();

        assert_eq!(config.resolve("cynic-kernel/src/domain/ccm/crystal.rs"), Some("domain-ccm".to_string()));
        assert_eq!(config.resolve("cynic-kernel/src/domain/dog.rs"), Some("domain-core".to_string()));
    }

    #[test]
    fn empty_zones_resolves_none() {
        let config = ZoneConfig::default();
        assert_eq!(config.resolve("anything.rs"), None);
    }
}
```

- [ ] **Step 2: Implement ZoneConfig**

```rust
//! Zone configuration — maps file paths to ownership zones.
//! Loaded from .cortex/zones.json at boot. Immutable after load.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct ZoneConfig {
    zones: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Deserialize)]
struct ZoneFile {
    zones: BTreeMap<String, ZoneEntry>,
}

#[derive(Debug, Deserialize)]
struct ZoneEntry {
    paths: Vec<String>,
    #[serde(default)]
    description: Option<String>,
}

impl ZoneConfig {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: ZoneFile = serde_json::from_str(json)
            .map_err(|e| format!("invalid zones.json: {e}"))?;

        let zones: Vec<(String, Vec<String>)> = file.zones
            .into_iter()
            .map(|(name, entry)| (name, entry.paths))
            .collect();

        Ok(ZoneConfig { zones })
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::from_json(&content)
    }

    /// Resolve a file path to its zone. Longest prefix match wins.
    pub fn resolve(&self, file_path: &str) -> Option<String> {
        let mut best: Option<(&str, usize)> = None;

        for (zone_name, paths) in &self.zones {
            for prefix in paths {
                if file_path.starts_with(prefix) && prefix.len() > best.map_or(0, |b| b.1) {
                    best = Some((zone_name.as_str(), prefix.len()));
                }
            }
        }

        best.map(|(name, _)| name.to_string())
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}
```

- [ ] **Step 3: Add `pub mod zones;` to domain/mod.rs**

- [ ] **Step 4: Run tests**

Run: `cargo test -p cynic-kernel zones::tests -- --nocapture`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```
git commit -m "feat(domain): ZoneConfig type with longest-prefix file-to-zone resolver"
```

---

### Task 2: Load zones at boot + inject into AppState

**Files:**
- Modify: `cynic-kernel/src/api/rest/types.rs` (AppState)
- Modify: `cynic-kernel/src/main.rs` (boot loading)
- Modify: `cynic-kernel/tests/rest_routes.rs` (two AppState construction sites)

- [ ] **Step 1: Add zones field to AppState**

In `types.rs`, add:
```rust
pub zones: Arc<crate::domain::zones::ZoneConfig>,
```

- [ ] **Step 2: Load zones.json in main.rs**

Before AppState construction:
```rust
let zones = {
    let zones_path = project_root.join(".claude").join("zones.json");
    let zones_str = zones_path.display().to_string();
    match crate::domain::zones::ZoneConfig::from_file(&zones_str) {
        Ok(z) => {
            klog!("[Ring 2] Zones: loaded {} zone(s) from {zones_str}", z.zone_count());
            Arc::new(z)
        }
        Err(e) => {
            klog!("[Ring 2] Zones: failed to load ({e}) — zone activity disabled");
            Arc::new(crate::domain::zones::ZoneConfig::default())
        }
    }
};
```

Add `zones` to AppState construction.

- [ ] **Step 3: Fix test AppState constructors**

In `tests/rest_routes.rs` — add `zones: Arc::new(crate::domain::zones::ZoneConfig::default())` to both `test_state()` and the SSE semaphore test AppState.

Ensure `domain::zones` is `pub` accessible from test (through crate re-exports).

- [ ] **Step 4: Compile check**

Run: `cargo check --workspace --all-targets`
Expected: clean

- [ ] **Step 5: Commit**

```
git commit -m "feat(boot): Load zones.json at kernel boot, inject into AppState"
```

---

### Task 3: `GET /dispatch/zone-activity` endpoint

**Files:**
- Create: `cynic-kernel/src/api/rest/dispatch.rs`
- Modify: `cynic-kernel/src/api/rest/mod.rs` (wire route)

- [ ] **Step 1: Define request/response types**

```rust
//! Dispatch endpoints — data-centric zone activity visibility.
//! Read-only queries over existing observation stream.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::rest::{AppState, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct ZoneActivityQuery {
    /// File path to resolve to zone (relative to project root)
    pub file_path: Option<String>,
    /// Or query a zone directly by name
    pub zone: Option<String>,
    /// Requesting agent (excluded from results)
    pub agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ZoneActivityResponse {
    /// Resolved zone (None if file doesn't map to any zone)
    pub zone: Option<String>,
    /// Other agents active in this zone, ordered by most recent
    pub active_agents: Vec<AgentActivity>,
    /// Total observations in this zone in the last hour
    pub observation_count: u64,
}

#[derive(Debug, Serialize)]
pub struct AgentActivity {
    pub agent_id: String,
    pub last_active: String,    // ISO timestamp
    pub last_file: String,      // most recent file edited
    pub activity_count: u64,    // observations in last hour
}
```

- [ ] **Step 2: Write the handler**

```rust
pub async fn zone_activity_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ZoneActivityQuery>,
) -> Result<Json<ZoneActivityResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Resolve zone
    let zone = match (&params.zone, &params.file_path) {
        (Some(z), _) => Some(z.clone()),
        (None, Some(fp)) => state.zones.resolve(fp),
        (None, None) => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "provide file_path or zone".into() }),
        )),
    };

    let zone = match zone {
        Some(z) => z,
        None => return Ok(Json(ZoneActivityResponse {
            zone: None,
            active_agents: vec![],
            observation_count: 0,
        })),
    };

    // 2. Get zone paths for SQL IN clause
    let zone_paths = state.zones.paths_for_zone(&zone).unwrap_or_default();
    if zone_paths.is_empty() {
        return Ok(Json(ZoneActivityResponse {
            zone: Some(zone),
            active_agents: vec![],
            observation_count: 0,
        }));
    }

    // 3. Query observations: agents active in this zone in last hour
    let exclude_agent = params.agent_id.as_deref().unwrap_or("");
    let agents = state.storage.zone_activity(&zone_paths, exclude_agent).await
        .unwrap_or_default();

    let observation_count = agents.iter().map(|a| a.activity_count).sum();

    Ok(Json(ZoneActivityResponse {
        zone: Some(zone),
        active_agents: agents,
        observation_count,
    }))
}
```

- [ ] **Step 3: Add `paths_for_zone` to ZoneConfig**

```rust
pub fn paths_for_zone(&self, zone_name: &str) -> Option<Vec<String>> {
    self.zones.iter()
        .find(|(name, _)| name == zone_name)
        .map(|(_, paths)| paths.clone())
}
```

- [ ] **Step 4: Add `zone_activity` to StoragePort**

In `domain/storage/mod.rs`, add:
```rust
/// Query agent activity in a set of path prefixes over the last hour.
/// Returns agents (excluding `exclude_agent`) with their latest observation.
async fn zone_activity(
    &self,
    _path_prefixes: &[String],
    _exclude_agent: &str,
) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
    Ok(vec![])
}
```

- [ ] **Step 5: Implement `zone_activity` in SurrealDB storage**

In `storage/surreal/` (new file or existing ops.rs):
```rust
async fn zone_activity(
    &self,
    path_prefixes: &[String],
    exclude_agent: &str,
) -> Result<Vec<AgentActivity>, StorageError> {
    // Build prefix match condition
    let prefix_conditions: Vec<String> = path_prefixes.iter()
        .map(|p| format!("target CONTAINS '{}'", escape_surreal(p)))
        .collect();
    let prefix_filter = prefix_conditions.join(" OR ");

    let query = format!(
        "SELECT agent_id, \
                math::max(timestamp) AS last_active, \
                array::first(target) AS last_file, \
                count() AS activity_count \
         FROM observations \
         WHERE ({prefix_filter}) \
           AND agent_id != '{}' \
           AND timestamp > time::now() - 1h \
         GROUP BY agent_id \
         ORDER BY last_active DESC \
         LIMIT 10;",
        escape_surreal(exclude_agent)
    );

    // Execute and parse results
    let results = self.query_raw(&query).await?;
    // ... parse into Vec<AgentActivity>
}
```

Note: Exact implementation depends on observation table schema. The observation `tool_input` contains file paths — the query needs to match against the `target` or `context` field that holds the file path. Check actual schema before implementing.

- [ ] **Step 6: Wire the route**

In `api/rest/mod.rs`:
```rust
pub mod dispatch;

// In router:
.route("/dispatch/zone-activity", get(dispatch::zone_activity_handler))
```

- [ ] **Step 7: Forward in ReconnectableStorage**

```rust
async fn zone_activity(&self, path_prefixes: &[String], exclude_agent: &str) -> Result<Vec<AgentActivity>, StorageError> {
    self.current().zone_activity(path_prefixes, exclude_agent).await
}
```

- [ ] **Step 8: Compile check**

Run: `cargo check --workspace --all-targets`

- [ ] **Step 9: Commit**

```
git commit -m "feat(dispatch): GET /dispatch/zone-activity — query observation stream for zone conflicts"
```

---

### Task 4: Rewrite hook as thin GET client

**Files:**
- Rewrite: `.cortex/mcp/coord-claim.sh`

- [ ] **Step 1: Write the new hook**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Zone activity check — thin client to kernel dispatch endpoint.
# Advisory only: warns on conflict, never blocks.
# The kernel derives activity from the observation stream (no state to manage).

INPUT=$(cat)

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
[[ -z "$FILE_PATH" ]] && exit 0

SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-}"
[[ -z "$API_KEY" ]] && exit 0

# Derive agent_id
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    STATE_FILE=$(ls /tmp/claude-*/*/state.json 2>/dev/null | head -1)
    if [[ -n "${STATE_FILE:-}" ]]; then
        AGENT_ID="claude-$(jq -r '.sessionId // empty' "$STATE_FILE" 2>/dev/null | head -c 12)"
    fi
fi
[[ -z "$AGENT_ID" ]] && exit 0

# Strip project root for relative path
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
REL_PATH="${FILE_PATH#$PROJECT_DIR/}"

# Query kernel: who else is active in this zone?
RESP=$(curl -s --connect-timeout 2 --max-time 3 \
    "http://${KERNEL_ADDR}/dispatch/zone-activity?file_path=$(printf '%s' "$REL_PATH" | jq -sRr @uri)&agent_id=${AGENT_ID}" \
    -H "Authorization: Bearer $API_KEY" \
    2>/dev/null || echo "")

# Parse response — advisory only
if [[ -n "$RESP" ]]; then
    ZONE=$(echo "$RESP" | jq -r '.zone // empty' 2>/dev/null)
    AGENTS=$(echo "$RESP" | jq -r '.active_agents[]?.agent_id' 2>/dev/null | head -3)
    if [[ -n "$AGENTS" && -n "$ZONE" ]]; then
        LAST=$(echo "$RESP" | jq -r '.active_agents[0].last_active // ""' 2>/dev/null)
        echo "⚠ ZONE '${ZONE}': also active — ${AGENTS//$'\n'/, } (last: ${LAST})" >&2
    fi
fi

exit 0
```

- [ ] **Step 2: Remove `/tmp/cynic-zones/` directory**

```bash
rm -rf /tmp/cynic-zones 2>/dev/null || true
```

- [ ] **Step 3: Test hook manually**

```bash
echo '{"tool_input":{"file_path":"/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/foo.sh"},"session_id":"test-1234-5678"}' | bash .cortex/mcp/coord-claim.sh
# Should exit 0, possibly with zone activity warning
```

- [ ] **Step 4: Commit**

```
git commit -m "refactor(hooks): Rewrite coord-claim as thin GET client — data-centric zone activity"
```

---

### Task 5: Integration test

**Files:**
- Create: `cynic-kernel/tests/integration_zone_activity.rs`

- [ ] **Step 1: Write integration test**

```rust
//! Zone activity integration — requires running kernel + SurrealDB
#![cfg(test)]

#[tokio::test]
#[ignore] // requires live kernel
async fn zone_activity_returns_data() {
    let client = reqwest::Client::new();
    let base = std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "127.0.0.1:3030".to_string());
    let key = std::env::var("CYNIC_API_KEY").unwrap_or_default();

    // Query zone activity for a file in scripts/
    let resp: serde_json::Value = client.get(format!("http://{base}/dispatch/zone-activity"))
        .header("Authorization", format!("Bearer {key}"))
        .query(&[("file_path", "scripts/config-sync.sh"), ("agent_id", "test-integration")])
        .send().await.unwrap()
        .json().await.unwrap();

    assert_eq!(resp["zone"], "scripts");
    assert!(resp["active_agents"].is_array());
    assert!(resp["observation_count"].is_number());

    // File outside any zone
    let resp: serde_json::Value = client.get(format!("http://{base}/dispatch/zone-activity"))
        .header("Authorization", format!("Bearer {key}"))
        .query(&[("file_path", "README.md"), ("agent_id", "test-integration")])
        .send().await.unwrap()
        .json().await.unwrap();

    assert!(resp["zone"].is_null());
    assert_eq!(resp["active_agents"].as_array().unwrap().len(), 0);

    // Query by zone name directly
    let resp: serde_json::Value = client.get(format!("http://{base}/dispatch/zone-activity"))
        .header("Authorization", format!("Bearer {key}"))
        .query(&[("zone", "api"), ("agent_id", "test-integration")])
        .send().await.unwrap()
        .json().await.unwrap();

    assert_eq!(resp["zone"], "api");
}
```

- [ ] **Step 2: Run against live kernel**

Run: `cargo test -p cynic-kernel integration_zone_activity -- --ignored --nocapture`

- [ ] **Step 3: Commit**

```
git commit -m "test: Integration test for zone-activity endpoint"
```

---

### Task 6: Build, deploy, verify

- [ ] **Step 1: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 2: Build release**

Run: `cargo build --release`

- [ ] **Step 3: Deploy**

```bash
cp target/release/cynic-kernel /tmp/cynic-kernel-new
mv ~/bin/cynic-kernel ~/bin/cynic-kernel.bak
cp /tmp/cynic-kernel-new ~/bin/cynic-kernel
systemctl --user restart cynic-kernel
```

- [ ] **Step 4: Verify zones loaded**

```bash
journalctl --user -eu cynic-kernel | grep "Zones:"
# Expected: "[Ring 2] Zones: loaded 11 zone(s)"
```

- [ ] **Step 5: Test endpoint live**

```bash
source ~/.cynic-env
curl -s "http://${CYNIC_REST_ADDR}/dispatch/zone-activity?file_path=scripts/foo.sh&agent_id=test" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | python3 -m json.tool
```

- [ ] **Step 6: Test hook end-to-end**

Edit any file — should see kernel query in journal, no more `/tmp/` locks.

- [ ] **Step 7: Run integration test**

- [ ] **Step 8: PR**
