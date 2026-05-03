# Cortex Protocol Wiring — How Agents Consume Artifacts

**Problem:** Protocol infrastructure exists (~/.cynic/organisms/). Consumers registered (ARTIFACTS_METADATA.json). But **no Cortex knows how to load it yet.**

**Solution:** Three integration points.

---

## Option A: Session Startup Hook (Simplest)

### Implementation

Create shell hook that runs at session init:

```bash
# ~/.config/claude/hooks/session-start-protocol.sh
#!/bin/bash
set -e

REGISTRY="$HOME/.cynic/organisms/consumers/consumer_registry.json"

if [[ ! -f "$REGISTRY" ]]; then
  exit 0  # Protocol not yet deployed
fi

echo "⏳ Loading artifact protocol..."

# Load consumer registry, report WAITING status
jq '.consumers[] | select(.status == "WAITING" or .status == null) | 
  {id, system, blocker: (.inputs[0].blocker // "none")}' "$REGISTRY" | \
  while IFS= read -r line; do
    if [[ -n "$line" ]]; then
      echo "  ⏸ $(jq -r '.id' <<< "$line"): $(jq -r '.blocker' <<< "$line")"
    fi
  done

# Load ORGANISM_PROTOCOL.md as context (optional)
PROTOCOL="$HOME/.cynic/organisms/artifacts/validated/protocol/ORGANISM_PROTOCOL.md"
if [[ -f "$PROTOCOL" ]]; then
  echo "✓ Protocol loaded. Consumers: $(jq '.consumers | length' "$REGISTRY")"
fi
```

**Integration:** Hook runs at session start, outputs blocker status to user.

**Consumer:** `session_init`  
**Access:** jq queries on consumer_registry.json

---

## Option B: Cortex Manifest (Medium)

### Implementation

Each Cortex declares what it consumes via a manifest file:

```json
// ~/.cynic/cortex-manifests/claude-code.json
{
  "cortex_id": "claude-code",
  "role": "engineering_cortex",
  "consumes": [
    {
      "consumer_id": "kernel_routing_v1",
      "artifacts": [
        "domain_discovery_complete",
        "token_gates_v1.3",
        "twitter_gates_v1.0"
      ],
      "action": "Load on startup if kernel_health=true"
    },
    {
      "consumer_id": "protocol_auditor",
      "artifacts": [
        "ARTIFACTS_METADATA.json",
        "ORGANISM_ARTIFACT_PROTOCOL.md"
      ],
      "action": "Verify compliance on monthly audit"
    }
  ]
}
```

```json
// ~/.cynic/cortex-manifests/gemini-cli.json
{
  "cortex_id": "gemini-cli",
  "role": "autonomous_cortex",
  "consumes": [
    {
      "consumer_id": "hermes_framing",
      "artifacts": [
        "organ_x_token_mentions_summary",
        "cortex_domain_generation"
      ],
      "action": "Inject into search reasoning if available"
    }
  ]
}
```

At Cortex startup:

```python
# In Claude Code initialization
import json
from pathlib import Path

def load_cortex_artifacts():
    manifest_path = Path.home() / ".cynic" / "cortex-manifests" / "claude-code.json"
    if not manifest_path.exists():
        return {}
    
    with open(manifest_path) as f:
        manifest = json.load(f)
    
    artifacts = {}
    for consumer in manifest.get("consumes", []):
        consumer_id = consumer["consumer_id"]
        for artifact_name in consumer.get("artifacts", []):
            # Load artifact from ~/.cynic/organisms/artifacts/
            artifact_path = find_artifact(artifact_name)
            if artifact_path:
                with open(artifact_path) as af:
                    artifacts[artifact_name] = json.load(af)
    
    return artifacts
```

**Consumer:** cortex-specific (claude-code, gemini-cli, hermes-9b)  
**Access:** Manifest declares what each cortex needs; Cortex loads at startup  
**Benefit:** Multi-cortex awareness (each knows what others consume, prevents conflicts)

---

## Option C: MCP Server (Most Scalable)

### Implementation

Create an MCP server that exposes ~/.cynic/organisms/ as resources:

```rust
// cynic-kernel/mcp-servers/artifact-server.rs
// Exposes artifact registry as MCP resources

#[derive(Debug, Serialize)]
struct ArtifactResource {
    uri: String,                    // artifact://domain-discovery/discovery
    name: String,
    maturity: String,               // validated, deferred, dead
    consumer: String,               // kernel_routing_v1
    description: String,
}

impl ResourceProvider for ArtifactServer {
    fn list_resources() -> Vec<ArtifactResource> {
        // Read ~/.cynic/organisms/artifacts/ and consumer_registry.json
        // Return all validated artifacts with metadata
        vec![
            ArtifactResource {
                uri: "artifact://domain-discovery/discovery".into(),
                name: "Domain Discovery Complete".into(),
                maturity: "validated".into(),
                consumer: "kernel_routing_v1".into(),
                description: "7 semantic clusters from 6K tweets".into(),
            },
            // ... more artifacts
        ]
    }
    
    fn read_resource(uri: &str) -> String {
        // Load artifact from filesystem
        // Return JSON or markdown
    }
}
```

Cortex usage (via MCP):

```python
# In any Cortex that supports MCP
artifacts = mcp_client.list_resources("artifact://")

# Find artifacts for my consumer
my_artifacts = [a for a in artifacts if a.consumer == "hermes_framing"]

for artifact in my_artifacts:
    content = mcp_client.read_resource(artifact.uri)
    # Use artifact
```

**Consumer:** Any cortex with MCP support  
**Access:** MCP client queries artifact server  
**Benefit:** Real-time artifact serving, no file parsing needed, cross-session artifact sharing

---

## Comparison

| Option | Setup | Runtime | Multi-Cortex | Scalability |
|--------|-------|---------|--------------|-------------|
| **A: Hook** | 30 min (shell script) | 5s (jq query) | No (local status only) | ⭐ Simple, one-time |
| **B: Manifest** | 2h (Python loader) | 10s (load + parse) | Yes (manifests) | ⭐⭐⭐ Medium, extensible |
| **C: MCP** | 4h (Rust server) | Real-time (query) | Yes (resource protocol) | ⭐⭐⭐⭐⭐ Scalable, live |

---

## Immediate Wiring (Next 2h)

### Step 1: Session Hook (10 min)
```bash
mkdir -p ~/.config/claude/hooks
# Write session-start-protocol.sh (shown above)
# Make executable, test with: bash ~/.config/claude/hooks/session-start-protocol.sh
```

### Step 2: Protocol Awareness in Claude Code (30 min)
Add to `.claude/settings.json`:
```json
{
  "protocols": {
    "artifact_protocol": {
      "enabled": true,
      "registry_path": "~/.cynic/organisms/consumers/consumer_registry.json",
      "artifact_base": "~/.cynic/organisms/artifacts"
    }
  }
}
```

At session init in Claude Code, read this config:
```python
import json
from pathlib import Path

settings = json.loads(Path("~/.claude/settings.json").read_text())
if settings.get("protocols", {}).get("artifact_protocol", {}).get("enabled"):
    registry = json.loads(Path(settings["protocols"]["artifact_protocol"]["registry_path"]).read_text())
    print(f"✓ Artifact protocol loaded: {len(registry['consumers'])} consumers")
```

### Step 3: Manifest for Claude Code (1h)
Create `~/.cynic/cortex-manifests/claude-code.json` (shown above).

Integrate into Claude Code startup:
```python
def initialize_artifacts():
    manifest_path = Path.home() / ".cynic" / "cortex-manifests" / "claude-code.json"
    if manifest_path.exists():
        with open(manifest_path) as f:
            manifest = json.load(f)
        
        for consumer in manifest["consumes"]:
            for artifact_name in consumer["artifacts"]:
                # Load each artifact
                artifact = load_artifact(artifact_name)
                # Store in session context
                session_context[artifact_name] = artifact
                print(f"  ✓ Loaded: {artifact_name}")
```

### Step 4: Test Wiring (20 min)
Start a new Claude Code session. Verify:
- [ ] Session hook runs (shows blocker status)
- [ ] Settings.json artifact_protocol is enabled
- [ ] Claude Code loads manifest
- [ ] Artifacts are available in session context

---

## Fallback: No Integration Yet (Acceptable)

If full integration is not done by May 5:

1. **Manual protocol check at session start:**
   ```bash
   jq '.consumers[] | select(.status == "WAITING")' ~/.cynic/organisms/consumers/consumer_registry.json
   ```
   User reads output, acts accordingly.

2. **Artifacts still consumable by K15 manually:**
   When kernel recovers, `/api/kernel/load_artifact?name=domain_discovery` can serve artifacts.

3. **Phase 2 gate (May 5-6) still fires** because falsification test is hardcoded in measurement code, not dependent on protocol wiring.

---

## Recommended Path

**Start with Option A + B (hook + manifest)** by May 4:
- Hook gives operator visibility (costs 30 min)
- Manifest gives Claude Code awareness (costs 1h)
- Together, they're 50% of the value with 10% of the MCP server effort

**Plan Option C (MCP server) for post-Phase-3** when protocol is proven stable.

---

## What Each Cortex Sees (After Wiring)

### Claude Code (this session)
```
⏳ Loading artifact protocol...
  ⏸ kernel_routing_v1: kernel_health = false
  ⏸ hermes_framing: cortex Δ test pending (May 5-6)
  ⏸ organism_learning: kernel unreachable via reflections
✓ Protocol loaded: 6 consumers
✓ Loaded artifacts: domain_discovery, token_gates_v1.3, twitter_gates_v1.0, KENOSIS_FINDINGS
```

### Gemini CLI (autonomous)
```
✓ Artifact manifest: gemini-cli.json
✓ Consuming: hermes_framing (organ_x_token_mentions_summary)
⏸ Deferred: cortex_domain_generation (waiting on Phase 2 Δ gate)
```

### Hermes 9B (execution engine)
```
✓ Protocol available via MCP (when wired)
✓ Ready to consume: behavioral-grounding artifacts
  - KENOSIS_FINDINGS (Wu-Wei patterns)
  - organ_x_token_mentions_summary (domain context)
```

---

## Falsification of Wiring

**Hypothesis:** If protocol wiring is correct, cortexes load artifacts without manual intervention.

**Test:** May 5-6 (Phase 2), when kernel recovers:
- kernel_routing_v1 automatically loads domain_discovery
- hermes_framing automatically injects cortex_domain_generation (if Δ > 5%)
- skill_evolution automatically generates SKILL.md (if Phase 2 succeeds)

**Success:** All 3 consumers activate automatically → signal improves.  
**Failure:** Any consumer fails to load → manual intervention needed → redesign wiring.

**Confidence:** φ⁻¹ (0.618) — wiring is new, untested on real cortexes yet.
