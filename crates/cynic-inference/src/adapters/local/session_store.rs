//! Filesystem session store under principal chat state dir.
//!
//! Layout:
//!   {root}/index.json
//!   {root}/sessions/{id}.jsonl

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::chat_session::{ChatEvent, SessionMeta};
use crate::ports::chat_session_port::{ChatSessionStore, SessionStoreError};

#[derive(Debug)]
pub struct FsChatSessionStore {
    root: PathBuf,
}

impl FsChatSessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, SessionStoreError> {
        let root = root.into();
        fs::create_dir_all(root.join("sessions")).map_err(|e| SessionStoreError::Io(e.to_string()))?;
        let store = Self { root };
        if !store.index_path().exists() {
            store.write_index(&[])?;
        }
        Ok(store)
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn session_path(&self, id: &str) -> PathBuf {
        // sanitize id for filesystem
        let safe: String = id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.root.join("sessions").join(format!("{safe}.jsonl"))
    }

    fn read_index(&self) -> Result<Vec<SessionMeta>, SessionStoreError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(vec![]);
        }
        let raw = fs::read_to_string(&path).map_err(|e| SessionStoreError::Io(e.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| SessionStoreError::Protocol(e.to_string()))
    }

    fn write_index(&self, sessions: &[SessionMeta]) -> Result<(), SessionStoreError> {
        let raw = serde_json::to_string_pretty(sessions)
            .map_err(|e| SessionStoreError::Protocol(e.to_string()))?;
        fs::write(self.index_path(), raw).map_err(|e| SessionStoreError::Io(e.to_string()))
    }

    fn upsert_meta(&self, meta: &SessionMeta) -> Result<(), SessionStoreError> {
        let mut sessions = self.read_index()?;
        if let Some(existing) = sessions.iter_mut().find(|s| s.id == meta.id) {
            *existing = meta.clone();
        } else {
            sessions.push(meta.clone());
        }
        sessions.sort_by(|a, b| b.updated_at_unix_ms.cmp(&a.updated_at_unix_ms));
        self.write_index(&sessions)
    }
}

impl ChatSessionStore for FsChatSessionStore {
    fn create_session(
        &self,
        id: &str,
        title: Option<&str>,
        model: Option<&str>,
        endpoint: Option<&str>,
    ) -> Result<SessionMeta, SessionStoreError> {
        if self.session_path(id).exists() {
            return self.get_meta(id);
        }
        let now = now_ms();
        let meta = SessionMeta {
            id: id.to_string(),
            title: title.map(|s| s.to_string()),
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
            model: model.map(|s| s.to_string()),
            endpoint: endpoint.map(|s| s.to_string()),
            event_count: 0,
        };
        // touch empty jsonl
        fs::write(self.session_path(id), "").map_err(|e| SessionStoreError::Io(e.to_string()))?;
        self.upsert_meta(&meta)?;
        Ok(meta)
    }

    fn get_meta(&self, id: &str) -> Result<SessionMeta, SessionStoreError> {
        self.read_index()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| SessionStoreError::NotFound(id.to_string()))
    }

    fn list_sessions(&self) -> Result<Vec<SessionMeta>, SessionStoreError> {
        self.read_index()
    }

    fn append(&self, id: &str, event: ChatEvent) -> Result<(), SessionStoreError> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(SessionStoreError::NotFound(id.to_string()));
        }
        let line = serde_json::to_string(&event)
            .map_err(|e| SessionStoreError::Protocol(e.to_string()))?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| SessionStoreError::Io(e.to_string()))?;
        writeln!(f, "{line}").map_err(|e| SessionStoreError::Io(e.to_string()))?;

        let mut meta = self.get_meta(id)?;
        meta.event_count = meta.event_count.saturating_add(1);
        meta.updated_at_unix_ms = now_ms();
        self.upsert_meta(&meta)?;
        Ok(())
    }

    fn load_events(&self, id: &str) -> Result<Vec<ChatEvent>, SessionStoreError> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(SessionStoreError::NotFound(id.to_string()));
        }
        let raw = fs::read_to_string(&path).map_err(|e| SessionStoreError::Io(e.to_string()))?;
        let mut events = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let e: ChatEvent = serde_json::from_str(line).map_err(|e| {
                SessionStoreError::Protocol(format!("line {}: {e}", i + 1))
            })?;
            events.push(e);
        }
        Ok(events)
    }

    fn export_jsonl(&self, id: &str) -> Result<String, SessionStoreError> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(SessionStoreError::NotFound(id.to_string()));
        }
        fs::read_to_string(&path).map_err(|e| SessionStoreError::Io(e.to_string()))
    }

    fn root_dir(&self) -> &Path {
        &self.root
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat_session::ChatEvent;
    use tempfile::tempdir;

    #[test]
    fn append_list_export_resume_roundtrip() {
        let dir = tempdir().unwrap();
        let store = FsChatSessionStore::new(dir.path()).unwrap();
        let meta = store
            .create_session("sess-1", Some("test"), Some("mock-model"), Some("http://local"))
            .unwrap();
        assert_eq!(meta.id, "sess-1");
        assert_eq!(meta.event_count, 0);

        store
            .append("sess-1", ChatEvent::user(1, "hello", 1000))
            .unwrap();
        store
            .append(
                "sess-1",
                ChatEvent::tool_call(2, "call_1", "echo", r#"{"text":"hi"}"#, 1001),
            )
            .unwrap();
        store
            .append(
                "sess-1",
                ChatEvent::tool_result(3, "call_1", "echo", "hi", 1002),
            )
            .unwrap();
        store
            .append("sess-1", ChatEvent::assistant(4, "done", 1003))
            .unwrap();

        let listed = store.list_sessions().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].event_count, 4);

        let events = store.load_events("sess-1").unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].kind, crate::domain::chat_session::ChatEventKind::User);
        assert_eq!(
            events[1].kind,
            crate::domain::chat_session::ChatEventKind::ToolCall
        );
        assert_eq!(
            events[2].kind,
            crate::domain::chat_session::ChatEventKind::ToolResult
        );
        assert_eq!(
            events[3].kind,
            crate::domain::chat_session::ChatEventKind::Assistant
        );
        assert_eq!(events[1].tool_name.as_deref(), Some("echo"));
        assert_eq!(events[2].content.as_deref(), Some("hi"));

        let export = store.export_jsonl("sess-1").unwrap();
        assert!(export.contains("\"kind\":\"user\""));
        assert!(export.contains("\"kind\":\"tool_call\""));
        assert!(export.contains("\"kind\":\"tool_result\""));

        // resume = load + append more
        store
            .append("sess-1", ChatEvent::user(5, "again", 2000))
            .unwrap();
        let events2 = store.load_events("sess-1").unwrap();
        assert_eq!(events2.len(), 5);
        assert_eq!(events2[4].content.as_deref(), Some("again"));
    }
}
