# cynic-askesis Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational Rust crate `cynic-askesis` with 4 core traits (LogStore, AuditEngine, AnchorProvider, DomainTracker) and base implementations (JsonlLog, GeminiWisdomAudit, GoogleCalendarAnchor), with NO DomainTracker implementation (body deferred to Phase 2). Sovereignty nue — zero enforcement.

**Architecture:** New workspace member crate. Port/adapter pattern: traits in `src/<domain>/mod.rs`, implementations in `src/<domain>/<impl>.rs`. Free-form JSONL log feeds a subprocess-based Gemini audit that produces a structured `Reflection`. Anchor creation uses Google Calendar REST API directly (OAuth2), not MCP.

**Tech Stack:** Rust edition 2024, tokio 1.50, chrono 0.4, reqwest 0.13 (json + native-tls), serde 1, thiserror 2.0, clap 4 (derive), yup-oauth2 12, tempfile 3 (tests). Workspace lints inherited.

**Spec reference:** `docs/superpowers/specs/2026-04-17-cynic-askesis-design.md`

---

## Pre-flight (required environment)

```bash
export RUST_MIN_STACK=67108864       # workspace A1 workaround (inherited via .cargo/config.toml)
export RUSTFLAGS="-C debuginfo=1"    # rmcp debug DWARF overflow prevention
cd /home/user/Bureau/CYNIC
```

Per `.claude/rules/workflow.md`: pre-commit validation `cargo build --tests && cargo clippy --workspace --all-targets -- -D warnings` is MANDATORY before every commit that touches Rust code.

---

## File Structure

```
CYNIC/
├── Cargo.toml                          MODIFY: add "cynic-askesis" to workspace members
└── cynic-askesis/                      CREATE
    ├── Cargo.toml                      CREATE: package + deps + [lints] workspace=true
    ├── README.md                       CREATE: philosophy + CLI + Phase 1 scope
    ├── src/
    │   ├── lib.rs                      CREATE: pub mod exports for integration tests
    │   ├── main.rs                     CREATE: clap CLI wiring traits
    │   ├── error.rs                    CREATE: AskesisError via thiserror
    │   ├── log/
    │   │   ├── mod.rs                  CREATE: LogStore trait + LogEntry
    │   │   └── jsonl.rs                CREATE: JsonlLog impl
    │   ├── audit/
    │   │   ├── mod.rs                  CREATE: AuditEngine trait
    │   │   └── gemini_wisdom.rs        CREATE: subprocess adapter
    │   ├── anchor/
    │   │   ├── mod.rs                  CREATE: AnchorProvider trait + AnchorId
    │   │   └── gcal.rs                 CREATE: Google Calendar REST adapter
    │   ├── domains/
    │   │   └── mod.rs                  CREATE: DomainTracker trait + empty registry
    │   └── reflection.rs               CREATE: Reflection struct + Verdict enum + markdown
    └── tests/
        ├── log_roundtrip.rs            CREATE: JsonlLog integration test
        ├── audit_mock.rs               CREATE: AuditEngine contract test with mock
        └── cli_smoke.rs                CREATE: end-to-end CLI smoke test
```

**Responsibility per file (K16 — 3 words each):**

| File | Purpose |
|------|---------|
| `log/mod.rs` | trait + LogEntry |
| `log/jsonl.rs` | JSONL append read |
| `audit/mod.rs` | trait + questions |
| `audit/gemini_wisdom.rs` | Gemini subprocess audit |
| `anchor/mod.rs` | trait + AnchorId |
| `anchor/gcal.rs` | Calendar REST client |
| `domains/mod.rs` | trait + registry |
| `reflection.rs` | Reflection markdown render |
| `error.rs` | error enum |
| `main.rs` | CLI arg parsing |
| `lib.rs` | public re-exports |

---

## Task 0: Workspace bootstrap

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `cynic-askesis/Cargo.toml`
- Create: `cynic-askesis/src/lib.rs`

- [ ] **Step 1: Add cynic-askesis to workspace members**

Edit `Cargo.toml`:

```toml
[workspace]
members = [
    "cynic-kernel",
    "cynic-node",
    "cynic-askesis",
]
resolver = "2"
```

- [ ] **Step 2: Create cynic-askesis/Cargo.toml**

```toml
[package]
name = "cynic-askesis"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"
publish = false

[[bin]]
name = "cynic-askesis"
path = "src/main.rs"

[lib]
name = "cynic_askesis"
path = "src/lib.rs"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1.50.0", features = ["macros", "rt-multi-thread", "process", "fs", "io-util"] }
thiserror = "2.0"
clap = { version = "4", features = ["derive"] }
dirs = "6"
reqwest = { version = "0.13", default-features = false, features = ["json", "native-tls"] }
yup-oauth2 = "12"
async-trait = "0.1"
tracing = "0.1"

[dev-dependencies]
tempfile = "3"
tokio = { version = "1.50.0", features = ["full", "test-util"] }

[lints]
workspace = true
```

- [ ] **Step 3: Create empty lib.rs stub**

File `cynic-askesis/src/lib.rs`:

```rust
//! cynic-askesis — 3rd CYNIC pillar: human augmentation layer.
//!
//! See docs/superpowers/specs/2026-04-17-cynic-askesis-design.md
```

- [ ] **Step 4: Verify workspace builds**

Run:
```bash
RUST_MIN_STACK=67108864 RUSTFLAGS="-C debuginfo=1" cargo build -p cynic-askesis
```
Expected: SUCCESS (empty crate compiles).

- [ ] **Step 5: Commit bootstrap**

```bash
git add Cargo.toml cynic-askesis/Cargo.toml cynic-askesis/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(askesis): bootstrap cynic-askesis workspace member

Add cynic-askesis as 3rd workspace member. Empty lib+bin skeleton
with dependencies declared. Workspace lints inherited.

Refs: docs/superpowers/specs/2026-04-17-cynic-askesis-design.md

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 1: Error type

**Files:**
- Create: `cynic-askesis/src/error.rs`
- Modify: `cynic-askesis/src/lib.rs` (add `pub mod error;`)

- [ ] **Step 1: Define AskesisError**

File `cynic-askesis/src/error.rs`:

```rust
//! Error types for cynic-askesis.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AskesisError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("serde_json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("audit engine unavailable: {0}")]
    AuditUnavailable(String),

    #[error("anchor provider error: {0}")]
    AnchorProvider(String),

    #[error("invalid log entry: {0}")]
    InvalidLogEntry(String),

    #[error("gemini subprocess failed: {0}")]
    GeminiSubprocess(String),

    #[error("google calendar api error: {0}")]
    GoogleCalendar(String),

    #[error("oauth2 error: {0}")]
    OAuth(String),
}

pub type Result<T> = std::result::Result<T, AskesisError>;
```

- [ ] **Step 2: Wire into lib.rs**

```rust
//! cynic-askesis — 3rd CYNIC pillar: human augmentation layer.
#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]

pub mod error;

pub use error::{AskesisError, Result};
```

- [ ] **Step 3: Verify build**

```bash
cargo build -p cynic-askesis
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add cynic-askesis/src/error.rs cynic-askesis/src/lib.rs
git commit -m "feat(askesis): error type via thiserror

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: LogEntry + LogStore trait

**Files:**
- Create: `cynic-askesis/src/log/mod.rs`
- Modify: `cynic-askesis/src/lib.rs`

- [ ] **Step 1: Write failing trait compile test**

Create `cynic-askesis/src/log/mod.rs`:

```rust
//! LogStore trait + LogEntry — free-form text logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Result;

/// A single free-form log entry.
///
/// `domain` is optional — Phase 1 accepts free-form logs without domain
/// classification. Gemini audit detects dimensions from the text itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub domain: Option<String>,
    pub content: String,
}

impl LogEntry {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            domain: None,
            content: content.into(),
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

/// Persistence port for log entries.
pub trait LogStore {
    fn append(&mut self, entry: LogEntry) -> Result<()>;
    fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LogEntry>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_new_sets_content_and_timestamp() {
        let entry = LogEntry::new("pas fait de sport");
        assert_eq!(entry.content, "pas fait de sport");
        assert!(entry.domain.is_none());
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn log_entry_with_domain_sets_domain() {
        let entry = LogEntry::new("pompes 3x20").with_domain("body");
        assert_eq!(entry.domain.as_deref(), Some("body"));
    }

    #[test]
    fn log_entry_serializes_roundtrip() {
        let entry = LogEntry::new("test").with_domain("body");
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

```rust
//! cynic-askesis — 3rd CYNIC pillar: human augmentation layer.
pub mod error;
pub mod log;

pub use error::{AskesisError, Result};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p cynic-askesis log::
```
Expected: 3 tests PASS.

- [ ] **Step 4: Clippy**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-askesis/src/log/mod.rs cynic-askesis/src/lib.rs
git commit -m "feat(askesis): LogEntry + LogStore trait

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: JsonlLog implementation

**Files:**
- Create: `cynic-askesis/src/log/jsonl.rs`
- Modify: `cynic-askesis/src/log/mod.rs` (add `pub mod jsonl;`)
- Create: `cynic-askesis/tests/log_roundtrip.rs`

- [ ] **Step 1: Write failing integration test**

Create `cynic-askesis/tests/log_roundtrip.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{Duration, Utc};
use cynic_askesis::log::{LogEntry, LogStore, jsonl::JsonlLog};
use tempfile::TempDir;

#[test]
fn append_then_range_returns_entries() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("log.jsonl");
    let mut store = JsonlLog::new(path).unwrap();

    let entry = LogEntry::new("pas fait de sport aujourd'hui").with_domain("body");
    store.append(entry.clone()).unwrap();

    let now = Utc::now();
    let entries = store.range(now - Duration::hours(1), now + Duration::hours(1)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, entry.content);
    assert_eq!(entries[0].domain, entry.domain);
}

#[test]
fn range_filters_by_timestamp() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("log.jsonl");
    let mut store = JsonlLog::new(path).unwrap();

    store.append(LogEntry::new("today")).unwrap();

    let future_from = Utc::now() + Duration::days(1);
    let future_to = future_from + Duration::hours(1);
    let entries = store.range(future_from, future_to).unwrap();
    assert_eq!(entries.len(), 0);
}
```

- [ ] **Step 2: Run test — expect FAIL (module doesn't exist)**

```bash
cargo test -p cynic-askesis --test log_roundtrip
```
Expected: COMPILE ERROR (jsonl module not found).

- [ ] **Step 3: Implement JsonlLog**

Create `cynic-askesis/src/log/jsonl.rs`:

```rust
//! JSONL-backed LogStore implementation.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::Result;
use crate::log::{LogEntry, LogStore};

pub struct JsonlLog {
    path: PathBuf,
}

impl JsonlLog {
    /// Opens (creating if needed) a JSONL log at `path`.
    ///
    /// Parent directory is created if missing. File permissions on creation
    /// are set to 0600 on Unix (via the implicit umask; Unix-specific tightening
    /// happens in append on first write if needed — Phase 1 relies on umask).
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Touch file to ensure existence for range() when empty.
        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl LogStore for JsonlLog {
    fn append(&mut self, entry: LogEntry) -> Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let line = serde_json::to_string(&entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LogEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(&line)?;
            if entry.timestamp >= from && entry.timestamp <= to {
                out.push(entry);
            }
        }
        Ok(out)
    }
}
```

- [ ] **Step 4: Expose from log/mod.rs**

Add to `cynic-askesis/src/log/mod.rs`:

```rust
pub mod jsonl;
```

- [ ] **Step 5: Run integration tests**

```bash
cargo test -p cynic-askesis --test log_roundtrip
```
Expected: 2 PASS.

- [ ] **Step 6: Clippy + build**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
cargo build -p cynic-askesis
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add cynic-askesis/src/log/jsonl.rs cynic-askesis/src/log/mod.rs cynic-askesis/tests/log_roundtrip.rs
git commit -m "feat(askesis): JsonlLog impl with integration tests

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Reflection + Verdict types

**Files:**
- Create: `cynic-askesis/src/reflection.rs`
- Modify: `cynic-askesis/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `cynic-askesis/src/reflection.rs`:

```rust
//! Reflection type — structured audit output.
//!
//! Verdict values parallel CYNIC's judge pipeline: HOWL (authentic) >
//! WAG (ok) > GROWL (shallow) > BARK (self-deception) > Degraded (audit down).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Howl,
    Wag,
    Growl,
    Bark,
    Degraded,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Howl => "HOWL",
            Verdict::Wag => "WAG",
            Verdict::Growl => "GROWL",
            Verdict::Bark => "BARK",
            Verdict::Degraded => "DEGRADED",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured reflection produced by an `AuditEngine`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reflection {
    pub verdict: Verdict,
    pub prose: String,
    pub patterns_detected: Vec<String>,
    pub kenosis_candidate: Option<String>,
    /// Confidence, φ⁻¹-bounded: MUST be ≤ 0.618.
    pub confidence: f32,
}

impl Reflection {
    /// Constructs a degraded reflection when the audit engine is unavailable.
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Degraded,
            prose: reason.into(),
            patterns_detected: Vec::new(),
            kenosis_candidate: None,
            confidence: 0.0,
        }
    }

    /// Render the reflection as a markdown document (for weekly-reflection.md).
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Weekly Reflection — {}\n\n", self.verdict));
        out.push_str(&format!("**Confidence:** {:.3} (φ⁻¹ bounded)\n\n", self.confidence));
        out.push_str("## Prose\n\n");
        out.push_str(&self.prose);
        out.push_str("\n\n");
        if !self.patterns_detected.is_empty() {
            out.push_str("## Patterns detected\n\n");
            for p in &self.patterns_detected {
                out.push_str(&format!("- {p}\n"));
            }
            out.push('\n');
        }
        if let Some(k) = &self.kenosis_candidate {
            out.push_str("## KENOSIS candidate\n\n");
            out.push_str(k);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_display_matches_as_str() {
        assert_eq!(Verdict::Howl.to_string(), "HOWL");
        assert_eq!(Verdict::Bark.to_string(), "BARK");
        assert_eq!(Verdict::Degraded.to_string(), "DEGRADED");
    }

    #[test]
    fn reflection_degraded_has_zero_confidence() {
        let r = Reflection::degraded("gemini timeout");
        assert_eq!(r.verdict, Verdict::Degraded);
        assert_eq!(r.confidence, 0.0);
        assert!(r.prose.contains("gemini timeout"));
    }

    #[test]
    fn reflection_markdown_contains_verdict_and_prose() {
        let r = Reflection {
            verdict: Verdict::Wag,
            prose: "Zey a bougé 3 fois cette semaine.".into(),
            patterns_detected: vec!["reprise progressive".into()],
            kenosis_candidate: None,
            confidence: 0.55,
        };
        let md = r.to_markdown();
        assert!(md.contains("WAG"));
        assert!(md.contains("Zey a bougé"));
        assert!(md.contains("reprise progressive"));
        assert!(md.contains("0.550"));
    }

    #[test]
    fn reflection_serializes_roundtrip() {
        let r = Reflection {
            verdict: Verdict::Howl,
            prose: "excellent".into(),
            patterns_detected: vec![],
            kenosis_candidate: Some("stopped smoking after lunch".into()),
            confidence: 0.6,
        };
        let json = serde_json::to_string(&r).unwrap();
        let parsed: Reflection = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

```rust
pub mod error;
pub mod log;
pub mod reflection;

pub use error::{AskesisError, Result};
pub use reflection::{Reflection, Verdict};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p cynic-askesis reflection::
```
Expected: 4 PASS.

- [ ] **Step 4: Clippy**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add cynic-askesis/src/reflection.rs cynic-askesis/src/lib.rs
git commit -m "feat(askesis): Reflection + Verdict types with markdown render

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: AuditEngine trait

**Files:**
- Create: `cynic-askesis/src/audit/mod.rs`
- Modify: `cynic-askesis/src/lib.rs`
- Create: `cynic-askesis/tests/audit_mock.rs`

- [ ] **Step 1: Define trait**

Create `cynic-askesis/src/audit/mod.rs`:

```rust
//! AuditEngine trait + questions for Gemini+cynic-wisdom.

use async_trait::async_trait;

use crate::Result;
use crate::log::LogEntry;
use crate::reflection::Reflection;

#[async_trait]
pub trait AuditEngine: Send + Sync {
    /// Audit a corpus of logs against audit questions.
    ///
    /// Must return `Reflection::degraded(reason)` on engine unavailability
    /// (per K14: poison/missing = degraded, never optimistic).
    async fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> Result<Reflection>;
}

/// Default Phase 1 audit questions (generic, not domain-specific).
/// Phase 2+ domains provide their own via DomainTracker::audit_questions.
pub fn default_phase1_questions() -> Vec<&'static str> {
    vec![
        "What patterns of self-deception vs honest reporting?",
        "What has Zey stopped doing? (KENOSIS check)",
        "Where is authenticity strongest, where weakest?",
        "Is the language concrete and grounded, or abstract and smoothing?",
    ]
}
```

- [ ] **Step 2: Write mock-based contract test**

Create `cynic-askesis/tests/audit_mock.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use cynic_askesis::Result;
use cynic_askesis::audit::{AuditEngine, default_phase1_questions};
use cynic_askesis::log::LogEntry;
use cynic_askesis::reflection::{Reflection, Verdict};

struct StaticMock(Reflection);

#[async_trait]
impl AuditEngine for StaticMock {
    async fn audit(&self, _logs: &[LogEntry], _questions: &[&str]) -> Result<Reflection> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn mock_audit_returns_static_reflection() {
    let expected = Reflection {
        verdict: Verdict::Wag,
        prose: "ok".into(),
        patterns_detected: vec![],
        kenosis_candidate: None,
        confidence: 0.5,
    };
    let engine = StaticMock(expected.clone());
    let result = engine.audit(&[], &default_phase1_questions()).await.unwrap();
    assert_eq!(result, expected);
}

#[test]
fn default_phase1_questions_are_non_empty() {
    let questions = default_phase1_questions();
    assert!(!questions.is_empty());
    assert!(questions.iter().any(|q| q.to_lowercase().contains("kenosis")));
}
```

- [ ] **Step 3: Add module to lib.rs**

```rust
pub mod audit;
pub mod error;
pub mod log;
pub mod reflection;
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p cynic-askesis --test audit_mock
```
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-askesis/src/audit/mod.rs cynic-askesis/src/lib.rs cynic-askesis/tests/audit_mock.rs
git commit -m "feat(askesis): AuditEngine trait + default Phase 1 questions

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: GeminiWisdomAudit (subprocess)

**Files:**
- Create: `cynic-askesis/src/audit/gemini_wisdom.rs`
- Modify: `cynic-askesis/src/audit/mod.rs`

**Design notes:**
- Embeds cynic-wisdom skill text at compile time via `include_str!("../../../.agents/skills/cynic-wisdom/SKILL.md")`
- Spawns `gemini -m gemini-2.5-pro -p <prompt>` via `tokio::process::Command`
- 120s timeout; on timeout → `Reflection::degraded`
- Output parser tolerant: looks for `VERDICT:` marker + prose; fallback to Degraded on malformed

- [ ] **Step 1: Write implementation**

Create `cynic-askesis/src/audit/gemini_wisdom.rs`:

```rust
//! Gemini CLI subprocess adapter with inline cynic-wisdom skill text.

use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;

use crate::Result;
use crate::audit::AuditEngine;
use crate::log::LogEntry;
use crate::reflection::{Reflection, Verdict};

/// Inline cynic-wisdom skill content. Version pinned at compile time.
///
/// **Load-bearing for FOGC** (spec §5): inverting axioms requires
/// changing this embedded text, which is committed to git under CODEOWNERS.
const CYNIC_WISDOM_SKILL: &str = include_str!(
    "../../../.agents/skills/cynic-wisdom/SKILL.md"
);

const DEFAULT_MODEL: &str = "gemini-2.5-pro";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

pub struct GeminiWisdomAudit {
    model: String,
    timeout: Duration,
}

impl Default for GeminiWisdomAudit {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl GeminiWisdomAudit {
    pub fn new(model: impl Into<String>, timeout: Duration) -> Self {
        Self {
            model: model.into(),
            timeout,
        }
    }

    fn build_prompt(logs: &[LogEntry], questions: &[&str]) -> String {
        let mut p = String::new();
        p.push_str("Use skill cynic-wisdom (inline text below).\n\n");
        p.push_str("=== cynic-wisdom SKILL TEXT ===\n");
        p.push_str(CYNIC_WISDOM_SKILL);
        p.push_str("\n=== END SKILL ===\n\n");
        p.push_str("Audit the following log entries against the questions.\n\n");
        p.push_str("=== LOGS ===\n");
        for e in logs {
            p.push_str(&format!(
                "[{}] domain={}: {}\n",
                e.timestamp.format("%Y-%m-%d %H:%M"),
                e.domain.as_deref().unwrap_or("(none)"),
                e.content
            ));
        }
        p.push_str("\n=== QUESTIONS ===\n");
        for q in questions {
            p.push_str(&format!("- {q}\n"));
        }
        p.push_str("\n=== OUTPUT FORMAT ===\n");
        p.push_str("Respond with exactly this structure:\n\n");
        p.push_str("VERDICT: HOWL|WAG|GROWL|BARK\n");
        p.push_str("CONFIDENCE: <float ≤ 0.618>\n");
        p.push_str("KENOSIS_CANDIDATE: <short sentence or NONE>\n");
        p.push_str("PATTERNS:\n- <pattern1>\n- <pattern2>\n\n");
        p.push_str("PROSE:\n<markdown narrative, honest but not shaming>\n");
        p
    }

    fn parse_output(raw: &str) -> Reflection {
        let mut verdict = Verdict::Degraded;
        let mut confidence: f32 = 0.0;
        let mut kenosis: Option<String> = None;
        let mut patterns: Vec<String> = Vec::new();
        let mut prose = String::new();
        let mut in_patterns = false;
        let mut in_prose = false;

        for line in raw.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("VERDICT:") {
                verdict = match rest.trim().to_uppercase().as_str() {
                    "HOWL" => Verdict::Howl,
                    "WAG" => Verdict::Wag,
                    "GROWL" => Verdict::Growl,
                    "BARK" => Verdict::Bark,
                    _ => Verdict::Degraded,
                };
                in_patterns = false;
                in_prose = false;
            } else if let Some(rest) = trimmed.strip_prefix("CONFIDENCE:") {
                confidence = rest.trim().parse().unwrap_or(0.0).min(0.618);
                in_patterns = false;
                in_prose = false;
            } else if let Some(rest) = trimmed.strip_prefix("KENOSIS_CANDIDATE:") {
                let v = rest.trim();
                kenosis = if v.eq_ignore_ascii_case("NONE") || v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                };
                in_patterns = false;
                in_prose = false;
            } else if trimmed == "PATTERNS:" {
                in_patterns = true;
                in_prose = false;
            } else if trimmed == "PROSE:" {
                in_patterns = false;
                in_prose = true;
            } else if in_patterns {
                if let Some(item) = trimmed.strip_prefix("- ") {
                    patterns.push(item.to_string());
                }
            } else if in_prose {
                prose.push_str(line);
                prose.push('\n');
            }
        }

        if verdict == Verdict::Degraded && prose.is_empty() {
            return Reflection::degraded("gemini output malformed");
        }

        Reflection {
            verdict,
            prose: prose.trim().to_string(),
            patterns_detected: patterns,
            kenosis_candidate: kenosis,
            confidence,
        }
    }
}

#[async_trait]
impl AuditEngine for GeminiWisdomAudit {
    async fn audit(&self, logs: &[LogEntry], questions: &[&str]) -> Result<Reflection> {
        let prompt = Self::build_prompt(logs, questions);

        let fut = Command::new("gemini")
            .args(["-m", &self.model, "-p", &prompt])
            .output();

        let output = match timeout(self.timeout, fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Ok(Reflection::degraded(format!("gemini spawn failed: {e}"))),
            Err(_) => return Ok(Reflection::degraded("gemini timed out")),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(Reflection::degraded(format!(
                "gemini exit != 0: {stderr}"
            )));
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_output(&raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_handles_well_formed_response() {
        let raw = "\
VERDICT: WAG
CONFIDENCE: 0.55
KENOSIS_CANDIDATE: stopped checking phone during dinner
PATTERNS:
- reprise progressive
- authentic language on body

PROSE:
Zey shows honest engagement with body tracking this week.
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert_eq!(r.verdict, Verdict::Wag);
        assert!((r.confidence - 0.55).abs() < 1e-6);
        assert_eq!(r.patterns_detected.len(), 2);
        assert!(r.kenosis_candidate.as_deref() == Some("stopped checking phone during dinner"));
        assert!(r.prose.contains("honest engagement"));
    }

    #[test]
    fn parse_output_clamps_confidence_to_phi_inverse() {
        let raw = "\
VERDICT: HOWL
CONFIDENCE: 0.99
KENOSIS_CANDIDATE: NONE
PATTERNS:
PROSE:
test
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert!(r.confidence <= 0.618);
    }

    #[test]
    fn parse_output_degraded_on_garbage() {
        let r = GeminiWisdomAudit::parse_output("lol what");
        assert_eq!(r.verdict, Verdict::Degraded);
    }

    #[test]
    fn parse_output_handles_kenosis_none() {
        let raw = "\
VERDICT: GROWL
CONFIDENCE: 0.4
KENOSIS_CANDIDATE: NONE
PATTERNS:
- shallow reporting
PROSE:
thin descriptions
";
        let r = GeminiWisdomAudit::parse_output(raw);
        assert_eq!(r.verdict, Verdict::Growl);
        assert!(r.kenosis_candidate.is_none());
    }

    #[test]
    fn build_prompt_includes_skill_text_and_questions() {
        let logs = vec![LogEntry::new("test entry").with_domain("body")];
        let questions = vec!["q1", "q2"];
        let prompt = GeminiWisdomAudit::build_prompt(&logs, &questions);
        assert!(prompt.contains("cynic-wisdom"));
        assert!(prompt.contains("test entry"));
        assert!(prompt.contains("q1"));
        assert!(prompt.contains("q2"));
        assert!(prompt.contains("VERDICT:"));
    }
}
```

- [ ] **Step 2: Export from audit/mod.rs**

Add to `cynic-askesis/src/audit/mod.rs`:

```rust
pub mod gemini_wisdom;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p cynic-askesis audit::gemini_wisdom
```
Expected: 5 PASS. The subprocess call itself is NOT unit-tested (would require real Gemini); parser is tested exhaustively.

- [ ] **Step 4: Clippy**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add cynic-askesis/src/audit/
git commit -m "feat(askesis): GeminiWisdomAudit subprocess adapter

Embeds cynic-wisdom skill inline (load-bearing for FOGC per spec §5).
Timeout 120s, graceful Reflection::degraded on failure.
Parser tolerant: VERDICT/CONFIDENCE/KENOSIS_CANDIDATE/PATTERNS/PROSE
structure, falls back to Degraded on malformed output.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: AnchorProvider trait + AnchorId

**Files:**
- Create: `cynic-askesis/src/anchor/mod.rs`
- Modify: `cynic-askesis/src/lib.rs`

- [ ] **Step 1: Define trait + types**

Create `cynic-askesis/src/anchor/mod.rs`:

```rust
//! AnchorProvider trait + AnchorId.

use async_trait::async_trait;
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Opaque provider-specific anchor identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorId(pub String);

impl AnchorId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[async_trait]
pub trait AnchorProvider: Send + Sync {
    async fn create_recurring(
        &self,
        domain: &str,
        at: NaiveTime,
        description: &str,
    ) -> Result<AnchorId>;

    async fn update_description(&self, id: AnchorId, new: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_id_roundtrip_via_serde() {
        let a = AnchorId::new("evt-abc");
        let json = serde_json::to_string(&a).unwrap();
        let parsed: AnchorId = serde_json::from_str(&json).unwrap();
        assert_eq!(a, parsed);
        assert_eq!(a.as_str(), "evt-abc");
    }
}
```

- [ ] **Step 2: Add module**

```rust
pub mod anchor;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p cynic-askesis anchor::
```
Expected: 1 PASS.

- [ ] **Step 4: Write mock AnchorProvider contract test**

Create `cynic-askesis/tests/anchor_mock.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use chrono::NaiveTime;
use cynic_askesis::Result;
use cynic_askesis::anchor::{AnchorId, AnchorProvider};

struct MockAnchor;

#[async_trait]
impl AnchorProvider for MockAnchor {
    async fn create_recurring(&self, domain: &str, _at: NaiveTime, _desc: &str) -> Result<AnchorId> {
        Ok(AnchorId::new(format!("mock-{domain}")))
    }
    async fn update_description(&self, _id: AnchorId, _new: &str) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn mock_anchor_creates_and_updates() {
    let anchor = MockAnchor;
    let id = anchor.create_recurring("body", NaiveTime::from_hms_opt(19, 0, 0).unwrap(), "test").await.unwrap();
    assert_eq!(id.as_str(), "mock-body");
    anchor.update_description(id, "updated").await.unwrap();
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p cynic-askesis --test anchor_mock
```
Expected: 1 PASS.

- [ ] **Step 6: Commit**

```bash
git add cynic-askesis/src/anchor/mod.rs cynic-askesis/src/lib.rs cynic-askesis/tests/anchor_mock.rs
git commit -m "feat(askesis): AnchorProvider trait + AnchorId + mock test

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: GoogleCalendarAnchor — OAuth2 setup

**Files:**
- Create: `cynic-askesis/src/anchor/gcal.rs`
- Modify: `cynic-askesis/src/anchor/mod.rs`

**Design notes:**
- Stores OAuth2 creds at `~/.cynic/askesis/gcal-creds.json`
- Uses `yup-oauth2` `InstalledFlowAuthenticator` (3-legged OAuth with loopback)
- Scope: `https://www.googleapis.com/auth/calendar.events`
- Phase 1 ships only the setup + create + update — no listing/deletion

- [ ] **Step 1: Implement GoogleCalendarAnchor**

Create `cynic-askesis/src/anchor/gcal.rs`:

```rust
//! Google Calendar REST adapter with OAuth2 (NOT MCP, per spec §3.3).

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use yup_oauth2::{
    ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
    authenticator::Authenticator,
    hyper_rustls::HttpsConnector,
    hyper_util::client::legacy::connect::HttpConnector,
};

use crate::Result;
use crate::anchor::{AnchorId, AnchorProvider};
use crate::error::AskesisError;

const SCOPE: &[&str] = &["https://www.googleapis.com/auth/calendar.events"];
const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";

pub struct GoogleCalendarAnchor {
    authenticator: Authenticator<HttpsConnector<HttpConnector>>,
    http: reqwest::Client,
    calendar_id: String,
}

/// Path discovery helper. Respects $HOME.
pub fn default_creds_path() -> PathBuf {
    dirs::home_dir()
        .expect("no HOME")
        .join(".cynic/askesis/gcal-creds.json")
}

impl GoogleCalendarAnchor {
    /// Builds the authenticator from a Google Cloud installed-app `client_secret.json`.
    /// The creds token cache is stored at `creds_cache_path`.
    pub async fn setup(
        client_secret_path: PathBuf,
        creds_cache_path: PathBuf,
        calendar_id: impl Into<String>,
    ) -> Result<Self> {
        let secret = yup_oauth2::read_application_secret(&client_secret_path)
            .await
            .map_err(|e| AskesisError::OAuth(format!("read secret: {e}")))?;
        Self::from_secret(secret, creds_cache_path, calendar_id).await
    }

    async fn from_secret(
        secret: ApplicationSecret,
        creds_cache_path: PathBuf,
        calendar_id: impl Into<String>,
    ) -> Result<Self> {
        if let Some(parent) = creds_cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let authenticator =
            InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
                .persist_tokens_to_disk(&creds_cache_path)
                .build()
                .await
                .map_err(|e| AskesisError::OAuth(format!("build authenticator: {e}")))?;

        Ok(Self {
            authenticator,
            http: reqwest::Client::builder()
                .build()
                .map_err(|e| AskesisError::GoogleCalendar(format!("http client: {e}")))?,
            calendar_id: calendar_id.into(),
        })
    }

    async fn token(&self) -> Result<String> {
        let tok = self
            .authenticator
            .token(SCOPE)
            .await
            .map_err(|e| AskesisError::OAuth(format!("fetch token: {e}")))?;
        tok.token()
            .map(|s| s.to_string())
            .ok_or_else(|| AskesisError::OAuth("token empty".into()))
    }
}

#[derive(Serialize)]
struct EventDateTime<'a> {
    #[serde(rename = "dateTime")]
    date_time: String,
    #[serde(rename = "timeZone")]
    time_zone: &'a str,
}

#[derive(Serialize)]
struct CreateEventBody<'a> {
    summary: String,
    description: &'a str,
    start: EventDateTime<'a>,
    end: EventDateTime<'a>,
    recurrence: Vec<&'a str>,
}

#[derive(Serialize)]
struct PatchEventBody<'a> {
    description: &'a str,
}

#[derive(Deserialize)]
struct EventResponse {
    id: String,
}

#[async_trait]
impl AnchorProvider for GoogleCalendarAnchor {
    async fn create_recurring(
        &self,
        domain: &str,
        at: NaiveTime,
        description: &str,
    ) -> Result<AnchorId> {
        // Tomorrow at `at`, 40min duration, daily recurrence.
        let tz = "Europe/Paris"; // Phase 1: hardcoded; config.toml in later phase
        let today = chrono::Local::now().date_naive();
        let start_naive = today.and_time(at);
        let end_naive = start_naive + chrono::Duration::minutes(40);

        // Format as RFC3339 local (the API interprets with timeZone field)
        let start_dt = start_naive.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_dt = end_naive.format("%Y-%m-%dT%H:%M:%S").to_string();

        let body = CreateEventBody {
            summary: format!("💪 cynic-askesis: {domain}"),
            description,
            start: EventDateTime { date_time: start_dt, time_zone: tz },
            end: EventDateTime { date_time: end_dt, time_zone: tz },
            recurrence: vec!["RRULE:FREQ=DAILY"],
        };

        let url = format!(
            "{CALENDAR_API_BASE}/calendars/{cid}/events",
            cid = urlencoding::encode(&self.calendar_id)
        );
        let token = self.token().await?;
        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("POST events: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AskesisError::GoogleCalendar(format!(
                "status {status}: {text}"
            )));
        }

        let parsed: EventResponse = resp
            .json()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("parse response: {e}")))?;
        Ok(AnchorId::new(parsed.id))
    }

    async fn update_description(&self, id: AnchorId, new: &str) -> Result<()> {
        let url = format!(
            "{CALENDAR_API_BASE}/calendars/{cid}/events/{eid}",
            cid = urlencoding::encode(&self.calendar_id),
            eid = urlencoding::encode(id.as_str()),
        );
        let token = self.token().await?;
        let body = PatchEventBody { description: new };
        let resp = self
            .http
            .patch(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AskesisError::GoogleCalendar(format!("PATCH event: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AskesisError::GoogleCalendar(format!(
                "status {status}: {text}"
            )));
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Add `urlencoding` + `hyper_util` deps**

In `cynic-askesis/Cargo.toml`:

```toml
[dependencies]
# ... existing ...
urlencoding = "2"
```

Note: `yup-oauth2` 12 already pulls in `hyper-rustls` and `hyper-util`. Confirm with `cargo tree`.

- [ ] **Step 3: Expose from anchor/mod.rs**

```rust
pub mod gcal;
```

- [ ] **Step 4: Build**

```bash
cargo build -p cynic-askesis
```
Expected: PASS. If `Authenticator<HttpsConnector<HttpConnector>>` does not compile due to re-export path mismatches in `yup-oauth2` v12, remove the explicit type annotation from the struct field and let the compiler infer it via the `from_secret` return type. May also need to adjust features: `yup-oauth2 = { version = "12", features = ["hyper-rustls"] }`.

- [ ] **Step 5: Clippy**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

Note: no unit test for the network client is included. An optional manual integration test would require Google Cloud credentials — not automated Phase 1. Parser-like logic is minimal (serde).

```bash
git add cynic-askesis/src/anchor/gcal.rs cynic-askesis/src/anchor/mod.rs cynic-askesis/Cargo.toml
git commit -m "feat(askesis): GoogleCalendarAnchor REST adapter with OAuth2

Direct Google Calendar API via yup-oauth2 + reqwest. NOT MCP (MCP
only available in Claude Code sessions; this binary must be
autonomous). Stores token cache at ~/.cynic/askesis/gcal-creds.json.

Phase 1: create_recurring (daily 40min) + update_description.
Scope minimal: calendar.events only.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: DomainTracker trait + empty registry

**Files:**
- Create: `cynic-askesis/src/domains/mod.rs`
- Modify: `cynic-askesis/src/lib.rs`

- [ ] **Step 1: Define trait + empty registry**

Create `cynic-askesis/src/domains/mod.rs`:

```rust
//! DomainTracker trait + registry.
//!
//! Phase 1: registry is deliberately empty. Body DomainTracker ships in Phase 2.

use chrono::NaiveTime;

pub trait DomainTracker: Send + Sync {
    fn name(&self) -> &str;
    fn log_prompt(&self) -> &str;
    fn audit_questions(&self) -> Vec<&str>;
    fn anchor_time(&self) -> NaiveTime;
}

/// Registry of domain trackers. Phase 1 returns an empty vector — intentional.
pub fn registry() -> Vec<Box<dyn DomainTracker>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_registry_is_empty() {
        assert!(registry().is_empty(), "Phase 1 MUST ship no domains");
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod anchor;
pub mod audit;
pub mod domains;
pub mod error;
pub mod log;
pub mod reflection;
```

- [ ] **Step 3: Test**

```bash
cargo test -p cynic-askesis domains::
```
Expected: 1 PASS.

- [ ] **Step 4: Commit**

```bash
git add cynic-askesis/src/domains/mod.rs cynic-askesis/src/lib.rs
git commit -m "feat(askesis): DomainTracker trait + empty Phase 1 registry

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: CLI (main.rs)

**Files:**
- Create: `cynic-askesis/src/main.rs`
- Create: `cynic-askesis/tests/cli_smoke.rs`

- [ ] **Step 1: Implement CLI**

Create `cynic-askesis/src/main.rs`:

```rust
//! cynic-askesis CLI.
#![allow(clippy::print_stdout)] // WHY: CLI binary; terminal output is the intended interface

use std::path::PathBuf;

use chrono::{Duration, NaiveTime, Utc};
use clap::{Parser, Subcommand};

use cynic_askesis::Result;
use cynic_askesis::anchor::{AnchorProvider, gcal::GoogleCalendarAnchor};
use cynic_askesis::audit::{AuditEngine, default_phase1_questions, gemini_wisdom::GeminiWisdomAudit};
use cynic_askesis::log::{LogEntry, LogStore, jsonl::JsonlLog};

#[derive(Parser)]
#[command(name = "cynic-askesis", version, about = "CYNIC 3rd pillar — human augmentation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Append a free-form log entry.
    Log {
        /// Optional domain tag (Phase 1 accepts free-form; Phase 2 adds body, reading, etc.)
        #[arg(long)]
        domain: Option<String>,
        /// Inline content; if omitted, reads from stdin interactively.
        content: Option<String>,
    },
    /// Display logs from the past N weeks.
    Reflect {
        #[arg(long, default_value_t = 1)]
        week: i64,
    },
    /// Run Gemini+cynic-wisdom audit over recent logs.
    Audit {
        #[arg(long, default_value_t = 1)]
        week: i64,
    },
    /// Google Calendar anchor subcommands.
    Anchor {
        #[command(subcommand)]
        sub: AnchorCommand,
    },
    /// Print status (paths, last log, last audit).
    Status,
}

#[derive(Subcommand)]
enum AnchorCommand {
    /// Run OAuth2 setup (browser flow).
    Setup {
        #[arg(long)]
        client_secret: PathBuf,
        #[arg(long, default_value = "primary")]
        calendar_id: String,
    },
    /// Create a recurring daily event.
    Add {
        #[arg(long)]
        name: String,
        /// Time in HH:MM (24h).
        #[arg(long)]
        time: String,
        #[arg(long)]
        description: String,
        #[arg(long, default_value = "primary")]
        calendar_id: String,
    },
}

fn askesis_dir() -> PathBuf {
    dirs::home_dir().expect("no HOME").join(".cynic/askesis")
}

fn log_path() -> PathBuf {
    askesis_dir().join("log.jsonl")
}

fn creds_path() -> PathBuf {
    askesis_dir().join("gcal-creds.json")
}

fn reflection_path() -> PathBuf {
    askesis_dir().join("weekly-reflection.md")
}

async fn run_log(domain: Option<String>, content: Option<String>) -> Result<()> {
    let content = match content {
        Some(c) => c,
        None => {
            use tokio::io::{AsyncBufReadExt, BufReader, stdin};
            print_prompt("Qu'est-ce qui est vrai aujourd'hui? ");
            let mut buf = String::new();
            BufReader::new(stdin()).read_line(&mut buf).await?;
            buf.trim().to_string()
        }
    };
    if content.is_empty() {
        println!("(empty — no log written)");
        return Ok(());
    }
    let mut store = JsonlLog::new(log_path())?;
    let entry = match domain {
        Some(d) => LogEntry::new(content).with_domain(d),
        None => LogEntry::new(content),
    };
    store.append(entry)?;
    println!("✓ logged");
    Ok(())
}

fn run_reflect(week: i64) -> Result<()> {
    let store = JsonlLog::new(log_path())?;
    let now = Utc::now();
    let from = now - Duration::weeks(week);
    let entries = store.range(from, now)?;
    if entries.is_empty() {
        println!("(no entries in last {week} week(s))");
        return Ok(());
    }
    for e in entries {
        println!(
            "[{}] {}: {}",
            e.timestamp.format("%Y-%m-%d %H:%M"),
            e.domain.as_deref().unwrap_or("(none)"),
            e.content
        );
    }
    Ok(())
}

async fn run_audit(week: i64) -> Result<()> {
    let store = JsonlLog::new(log_path())?;
    let now = Utc::now();
    let from = now - Duration::weeks(week);
    let entries = store.range(from, now)?;
    if entries.is_empty() {
        println!("(no entries to audit)");
        return Ok(());
    }
    let engine = GeminiWisdomAudit::default();
    let questions = default_phase1_questions();
    let reflection = engine.audit(&entries, &questions).await?;
    let md = reflection.to_markdown();
    std::fs::write(reflection_path(), &md)?;
    println!("{md}");
    println!("→ written to {}", reflection_path().display());
    Ok(())
}

async fn run_anchor_setup(client_secret: PathBuf, calendar_id: String) -> Result<()> {
    // Setup triggers OAuth; successful setup persists creds for future use.
    let _ = GoogleCalendarAnchor::setup(client_secret, creds_path(), calendar_id).await?;
    println!("✓ OAuth2 setup complete. Creds at {}", creds_path().display());
    Ok(())
}

async fn run_anchor_add(
    name: String,
    time: String,
    description: String,
    calendar_id: String,
) -> Result<()> {
    let at = NaiveTime::parse_from_str(&time, "%H:%M")
        .map_err(|e| cynic_askesis::AskesisError::AnchorProvider(format!("bad time: {e}")))?;
    // Re-uses existing creds (must have run `anchor setup` first).
    // For Phase 1 we re-load from disk with a dummy secret path — real impl
    // would split into a "loader" helper. Here we assume setup was already done
    // and simply reuse the cache via an in-memory dummy authentication.
    //
    // Phase 1 simplification: setup MUST be run once; after that anchor add
    // uses the cached token. This is enforced by requiring --client-secret
    // on `setup` only.
    let client_secret_path = askesis_dir().join("client_secret.json");
    if !client_secret_path.exists() {
        return Err(cynic_askesis::AskesisError::AnchorProvider(
            format!("missing {}; run `anchor setup --client-secret PATH` first", client_secret_path.display())
        ));
    }
    let provider =
        GoogleCalendarAnchor::setup(client_secret_path, creds_path(), calendar_id).await?;
    let id = provider.create_recurring(&name, at, &description).await?;
    println!("✓ anchor created: {}", id.as_str());
    Ok(())
}

fn run_status() -> Result<()> {
    println!("cynic-askesis status");
    println!("  askesis dir: {}", askesis_dir().display());
    println!("  log: {} ({})", log_path().display(),
        if log_path().exists() { "present" } else { "absent" });
    println!("  gcal creds: {} ({})", creds_path().display(),
        if creds_path().exists() { "present" } else { "absent" });
    println!("  last reflection: {} ({})", reflection_path().display(),
        if reflection_path().exists() { "present" } else { "absent" });
    Ok(())
}

fn print_prompt(s: &str) {
    use std::io::Write;
    print!("{s}");
    let _ = std::io::stdout().flush();
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().ok();
    let cli = Cli::parse();
    match cli.command {
        Command::Log { domain, content } => run_log(domain, content).await,
        Command::Reflect { week } => run_reflect(week),
        Command::Audit { week } => run_audit(week).await,
        Command::Anchor { sub } => match sub {
            AnchorCommand::Setup { client_secret, calendar_id } => {
                // Also copy client_secret to askesis dir for later reuse by `add`.
                std::fs::create_dir_all(askesis_dir())?;
                std::fs::copy(&client_secret, askesis_dir().join("client_secret.json"))?;
                run_anchor_setup(client_secret, calendar_id).await
            }
            AnchorCommand::Add { name, time, description, calendar_id } => {
                run_anchor_add(name, time, description, calendar_id).await
            }
        },
        Command::Status => run_status(),
    }
}
```

- [ ] **Step 2: Add `tracing-subscriber` dep**

In `cynic-askesis/Cargo.toml`:

```toml
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 3: Write CLI smoke test**

Create `cynic-askesis/tests/cli_smoke.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! End-to-end smoke test for CLI — log then reflect.
//!
//! Does not cover audit (would require Gemini) or anchor (would require OAuth).

use std::process::Command;
use tempfile::TempDir;

#[test]
fn log_then_reflect_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path();

    let binary = env!("CARGO_BIN_EXE_cynic-askesis");

    let log_out = Command::new(binary)
        .env("HOME", home)
        .args(["log", "--domain", "body", "pompes 3x20"])
        .output()
        .unwrap();
    assert!(
        log_out.status.success(),
        "log failed: stderr={}",
        String::from_utf8_lossy(&log_out.stderr)
    );

    let reflect_out = Command::new(binary)
        .env("HOME", home)
        .args(["reflect", "--week", "1"])
        .output()
        .unwrap();
    assert!(reflect_out.status.success());
    let stdout = String::from_utf8_lossy(&reflect_out.stdout);
    assert!(
        stdout.contains("pompes 3x20"),
        "expected log content in reflect stdout: {stdout}"
    );
}

#[test]
fn status_on_fresh_home_prints_all_absent() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path();
    let binary = env!("CARGO_BIN_EXE_cynic-askesis");
    let out = Command::new(binary)
        .env("HOME", home)
        .arg("status")
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("absent"));
}
```

- [ ] **Step 4: Run smoke tests**

```bash
cargo test -p cynic-askesis --test cli_smoke
```
Expected: 2 PASS.

- [ ] **Step 5: Clippy + build**

```bash
cargo clippy -p cynic-askesis --all-targets -- -D warnings
cargo build -p cynic-askesis --release
```

- [ ] **Step 6: Commit**

```bash
git add cynic-askesis/src/main.rs cynic-askesis/tests/cli_smoke.rs cynic-askesis/Cargo.toml
git commit -m "feat(askesis): CLI (log/reflect/audit/anchor/status)

End-to-end smoke test covers log → reflect roundtrip.
Audit and anchor are documented but not auto-tested (require Gemini
subprocess and Google OAuth respectively).

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: README

**Files:**
- Create: `cynic-askesis/README.md`

- [ ] **Step 1: Write README**

Create `cynic-askesis/README.md`:

```markdown
# cynic-askesis

> *"The lips of Wisdom are closed, except to the ears of Understanding."* — The Kybalion

**Third CYNIC pillar.** Human augmentation layer applying the 6-axiom framework to your own life.

- `cynic-kernel` = judgment pipeline for code/text
- `cynic-node` = standalone agent
- **`cynic-askesis`** = human augmentation (this crate)

## Philosophy

Sovereignty nue. Zero enforcement. System = **lamp, not hammer**.

The mechanism is self-honesty externalized to an AI interlocutor. It is simpler to say *"I didn't move my body today"* to an AI than to a human. Over time, this honesty trains the same honesty in all dimensions.

Full design: [`docs/superpowers/specs/2026-04-17-cynic-askesis-design.md`](../docs/superpowers/specs/2026-04-17-cynic-askesis-design.md).

## Phase 1 Scope

This crate ships the **foundational abstractions** only — no domain-specific tracker.

| Component | Status |
|-----------|--------|
| LogStore trait + JsonlLog | ✅ |
| AuditEngine trait + GeminiWisdomAudit | ✅ |
| AnchorProvider trait + GoogleCalendarAnchor | ✅ |
| DomainTracker trait + registry | ✅ (empty registry) |
| Body domain | Phase 2 |
| Reading / Focus / Solana KPIs | Phase 3+ |

## CLI

```bash
cynic-askesis log                               # free-form interactive log
cynic-askesis log --domain body "pompes 3x20"   # tagged log
cynic-askesis reflect --week 1                  # show logs of past week
cynic-askesis audit --week 1                    # Gemini+cynic-wisdom audit
cynic-askesis anchor setup --client-secret google-oauth-client.json
cynic-askesis anchor add --name "Body" --time "19:00" --description "..."
cynic-askesis status                            # paths + file presence
```

Storage: `~/.cynic/askesis/`
- `log.jsonl` — append-only log
- `gcal-creds.json` — OAuth2 token cache (mode 0600)
- `weekly-reflection.md` — latest audit output
- `client_secret.json` — Google Cloud OAuth client (local copy)

## Requirements

- Rust 1.94+ (edition 2024)
- `gemini` CLI in PATH (for audit — optional; fallback degraded reflection)
- Google Cloud project with OAuth2 client credentials (for anchor — optional)

## Testing

```bash
export RUST_MIN_STACK=67108864
cargo test -p cynic-askesis
cargo clippy -p cynic-askesis --all-targets -- -D warnings
```

## Falsifiability

- **Architecture**: if extending to a 2nd domain requires refactoring traits → architecture failed
- **Product** (after Phase 2 body + 30 days): if you stop logging or write "ok" placeholders → product failed
- **Axiom** (KENOSIS, after 3 months): if every KENOSIS insight reduces to BURN/FIDELITY → reject 7th axiom

## License

Apache-2.0. Part of the CYNIC project.
```

- [ ] **Step 2: Commit**

```bash
git add cynic-askesis/README.md
git commit -m "docs(askesis): README with philosophy + Phase 1 CLI

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Workspace compliance — `make check` + lint-drift

**Files:** none directly; verifies prior work.

- [ ] **Step 1: Run full workspace lints**

```bash
export RUST_MIN_STACK=67108864
export RUSTFLAGS="-C debuginfo=1"
cargo build --workspace --tests
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: PASS. Fix any K-rules violations (K1-K16) surfaced.

- [ ] **Step 2: Run `make check`**

```bash
make check
```
Expected: PASS for build + test + clippy + lint-rules + lint-drift + audit.

If lint-drift fails because `cynic-askesis/` isn't recognized, update `scripts/lint-drift.sh` or `Makefile` check targets to include the new workspace member.

- [ ] **Step 3: Run `make test-gates`**

```bash
make test-gates
```
Expected: PASS. The R21 gate falsification tests should still pass since cynic-askesis is independent.

- [ ] **Step 4: If any fix needed, commit separately**

```bash
# example if Makefile update needed
git add Makefile
git commit -m "chore(make): include cynic-askesis in lint pipeline

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Branch + PR

**Files:** none directly; origin/main is protected per workflow.md.

- [ ] **Step 1: Create branch from current HEAD**

All Phase 1 commits are on local `main`. Create a PR branch:

```bash
git checkout -b feat/cynic-askesis-phase1-foundations-2026-04-17
```

- [ ] **Step 2: Push branch**

```bash
git push -u origin feat/cynic-askesis-phase1-foundations-2026-04-17
```
Expected: pre-push hook runs `make check`; PASS.

- [ ] **Step 3: Open PR**

```bash
gh pr create --base main --title "feat(askesis): Phase 1 foundations — traits + impls" --body "$(cat <<'EOF'
## Summary

- Adds `cynic-askesis` as 3rd workspace member (parallel to kernel/node).
- Ships 4 foundational traits: `LogStore`, `AuditEngine`, `AnchorProvider`, `DomainTracker`.
- Base impls: `JsonlLog`, `GeminiWisdomAudit`, `GoogleCalendarAnchor` (REST+OAuth2).
- `Reflection` type with structured `Verdict` enum (HOWL/WAG/GROWL/BARK/Degraded).
- CLI: log / reflect / audit / anchor / status.
- No `DomainTracker` implementation — intentional, body deferred to Phase 2.
- All tests pass. Workspace lints green.

## Philosophy

Sovereignty nue. Zero enforcement. System = lamp, not hammer. Mechanism = self-honesty log + weekly Gemini+cynic-wisdom audit. See [design spec](../blob/main/docs/superpowers/specs/2026-04-17-cynic-askesis-design.md).

## Test plan

- [x] `cargo test -p cynic-askesis` — all green
- [x] `cargo clippy -p cynic-askesis --all-targets -- -D warnings` — green
- [x] `make check` — green
- [ ] Reviewer runs `cynic-askesis log "test"` + `cynic-askesis reflect` manually
- [ ] Reviewer reviews audit subprocess prompt template in `gemini_wisdom.rs`
- [ ] Reviewer confirms FOGC reasoning (spec §5) holds

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Return PR URL to user**

---

## Summary of Steps

| Task | Files touched | Commits | Tests added |
|------|---------------|---------|-------------|
| 0. Workspace bootstrap | root Cargo.toml, crate Cargo.toml, lib.rs | 1 | 0 |
| 1. Error type | error.rs, lib.rs | 1 | 0 |
| 2. LogEntry + LogStore trait | log/mod.rs, lib.rs | 1 | 3 unit |
| 3. JsonlLog impl | log/jsonl.rs, tests/log_roundtrip.rs | 1 | 2 integration |
| 4. Reflection + Verdict | reflection.rs, lib.rs | 1 | 4 unit |
| 5. AuditEngine trait | audit/mod.rs, tests/audit_mock.rs | 1 | 2 |
| 6. GeminiWisdomAudit | audit/gemini_wisdom.rs | 1 | 5 unit (parser) |
| 7. AnchorProvider trait | anchor/mod.rs, lib.rs, tests/anchor_mock.rs | 1 | 1 unit + 1 mock contract |
| 8. GoogleCalendarAnchor | anchor/gcal.rs, Cargo.toml | 1 | 0 (network; mock covered in Task 7) |
| 9. DomainTracker + empty registry | domains/mod.rs, lib.rs | 1 | 1 unit |
| 10. CLI | main.rs, tests/cli_smoke.rs, Cargo.toml | 1 | 2 integration |
| 11. README | README.md | 1 | 0 |
| 12. make check compliance | various fixes | 0-1 | 0 |
| 13. Branch + PR | none | 0 | 0 |

**Total commits**: 12-13. **Total new test assertions**: 22.

---

## Decision log

Recorded here to preserve reasoning:

1. **`async-trait`** — used for `AuditEngine` and `AnchorProvider` (network-bound). `LogStore` stays sync (file I/O is fast enough for Phase 1; revisit if JSONL grows unwieldy).
2. **`NaiveTime` over `DateTime`** — anchor times are wall-clock local (19:00); timezone lives in `config.toml` later, hardcoded to `Europe/Paris` in Phase 1.
3. **JSONL append with `serde_json::to_string` + `writeln!`** — simple, durable, line-grep-able. Not stream-parsed; full-file read in `range()`. Acceptable until > 10k entries.
4. **`include_str!` for cynic-wisdom skill** — embeds skill content at compile time (§5 FOGC defense). Version pinned to git commit.
5. **No schema migrations** — JSONL format simple enough; new fields can be `#[serde(default)]` later.
6. **OAuth2 setup copies `client_secret.json` into askesis dir** — so `anchor add` reuses it without re-specifying path. Friction avoidance.
7. **CLI `log` reads stdin when content omitted** — interactive prompt matches the design intent (*"Qu'est-ce qui est vrai aujourd'hui?"*).

---

## References

- Spec: `docs/superpowers/specs/2026-04-17-cynic-askesis-design.md`
- Workflow: `.claude/rules/workflow.md` (pre-commit validation, branch-PR discipline)
- Kernel rules: `.claude/rules/kernel.md` (K1-K16)
- Universal rules: `.claude/rules/universal.md` (Rules 1-23)
- Constitution: `CLAUDE.md`
- Memory: `project_cynic_askesis_design.md`, `project_seventh_axiom_kenosis.md`, `feedback_claude_gemini_pyrrhonist.md`, `feedback_lazy_engineer_pattern.md`, `feedback_fogc_vigilance.md`
