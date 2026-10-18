//! SQLite adapter for CanonicalStorePort.
//!
//! Strict port of `.../adapters/store_sqlite.py`. The canonical layer is
//! AUTHORITATIVE: this store never loses a fact and never invents one.
//! Writes are idempotent by primary key; unobserved fields stay NULL.

use std::collections::HashMap;
use std::fmt;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::domain::{
    CorrectionKind, Edge, EdgeKind, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot,
    ToolCall, Turn,
};
use crate::ports::CanonicalStorePort;
use crate::session::NormalizedSession;

use super::content_store::ContentBlobStore;
use super::store_schema::SCHEMA_SQL;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownTable(String),
    UnknownValue(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "sqlite: {e}"),
            StoreError::Io(e) => write!(f, "io: {e}"),
            StoreError::Json(e) => write!(f, "json: {e}"),
            StoreError::UnknownTable(t) => write!(f, "unknown table {t}"),
            StoreError::UnknownValue(v) => write!(f, "unknown stored value {v}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Json(e)
    }
}

/// JSON-encode a mapping with sorted keys (deterministic, like the Python
/// `sort_keys=True`), or None for an empty mapping.
fn json_map(map: &HashMap<String, Value>) -> Option<String> {
    if map.is_empty() {
        return None;
    }
    let sorted: serde_json::Map<String, Value> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    serde_json::to_string(&sorted).ok()
}

fn json_list(items: &[String]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    serde_json::to_string(items).ok()
}

fn parse_map(text: Option<String>) -> HashMap<String, Value> {
    text.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn kind_from(s: &str) -> Result<EdgeKind, StoreError> {
    match s {
        "reply" => Ok(EdgeKind::Reply),
        "tool_call" => Ok(EdgeKind::ToolCall),
        "tool_result" => Ok(EdgeKind::ToolResult),
        "reasoning" => Ok(EdgeKind::Reasoning),
        "compaction" => Ok(EdgeKind::Compaction),
        "sidechain" => Ok(EdgeKind::Sidechain),
        "subagent" => Ok(EdgeKind::Subagent),
        "battle" => Ok(EdgeKind::Battle),
        "model_change" => Ok(EdgeKind::ModelChange),
        "sequence" => Ok(EdgeKind::Sequence),
        other => Err(StoreError::UnknownValue(format!("edge kind {other}"))),
    }
}

fn provenance_from(s: &str) -> Result<EdgeProvenance, StoreError> {
    match s {
        "native" => Ok(EdgeProvenance::Native),
        "derived" => Ok(EdgeProvenance::Derived),
        other => Err(StoreError::UnknownValue(format!("provenance {other}"))),
    }
}

fn correction_from(s: &str) -> Result<CorrectionKind, StoreError> {
    match s {
        "redirect" => Ok(CorrectionKind::Redirect),
        "constraint" => Ok(CorrectionKind::Constraint),
        "rejection" => Ok(CorrectionKind::Rejection),
        "approval" => Ok(CorrectionKind::Approval),
        "unknown" => Ok(CorrectionKind::Unknown),
        other => Err(StoreError::UnknownValue(format!("correction kind {other}"))),
    }
}

fn kind_str(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Reply => "reply",
        EdgeKind::ToolCall => "tool_call",
        EdgeKind::ToolResult => "tool_result",
        EdgeKind::Reasoning => "reasoning",
        EdgeKind::Compaction => "compaction",
        EdgeKind::Sidechain => "sidechain",
        EdgeKind::Subagent => "subagent",
        EdgeKind::Battle => "battle",
        EdgeKind::ModelChange => "model_change",
        EdgeKind::Sequence => "sequence",
    }
}

fn provenance_str(p: EdgeProvenance) -> &'static str {
    match p {
        EdgeProvenance::Native => "native",
        EdgeProvenance::Derived => "derived",
    }
}

fn correction_str(k: CorrectionKind) -> &'static str {
    match k {
        CorrectionKind::Redirect => "redirect",
        CorrectionKind::Constraint => "constraint",
        CorrectionKind::Rejection => "rejection",
        CorrectionKind::Approval => "approval",
        CorrectionKind::Unknown => "unknown",
    }
}

pub struct SqliteCanonicalStore {
    db_path: String,
    con: Connection,
    blobs: ContentBlobStore,
}

impl std::fmt::Debug for SqliteCanonicalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteCanonicalStore")
            .field("db_path", &self.db_path)
            .finish_non_exhaustive()
    }
}

impl SqliteCanonicalStore {
    pub fn new(db_path: &str, blob_root: Option<&str>, bulk_load: bool) -> Result<Self, StoreError> {
        let con = Connection::open(db_path)?;
        if bulk_load {
            // Bulk load: no WAL, no fsync. Safe because the source is a frozen
            // capture — if the load dies, we re-capture and re-run.
            con.execute_batch(
                "PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -200000;",
            )?;
        } else {
            con.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        }
        con.execute_batch(SCHEMA_SQL)?;
        let root = blob_root.map(str::to_owned).unwrap_or_else(|| format!("{db_path}.blobs"));
        let blobs = ContentBlobStore::new(&root)?;
        Ok(Self {
            db_path: db_path.to_owned(),
            con,
            blobs,
        })
    }

    /// Store a payload, externalizing it if too large. None stays NULL;
    /// "" stays "" — distinct raw states, never collapsed.
    fn put_payload(&self, text: Option<&str>) -> Result<Option<String>, StoreError> {
        let Some(t) = text else {
            return Ok(None);
        };
        let (digest, inlined) = self.blobs.put(t)?;
        if inlined {
            return Ok(Some(t.to_owned()));
        }
        self.con.execute(
            "INSERT OR IGNORE INTO blobs (content_hash, size_bytes, created_at_iso) VALUES (?,?,?)",
            params![digest, t.len() as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(Some(digest))
    }

    fn get_payload(&self, value: Option<String>) -> Option<String> {
        match value {
            None => None,
            Some(v) if v.starts_with("sha256:") => self.blobs.get(&v),
            Some(v) => Some(v),
        }
    }

    pub fn session_uids(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .con
            .prepare("SELECT session_uid FROM sessions ORDER BY session_uid")?;
        Ok(stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?)
    }
}

impl CanonicalStorePort for SqliteCanonicalStore {
    type Error = StoreError;

    fn write(&self, session: &NormalizedSession) -> Result<(), StoreError> {
        // One transaction per session: a mid-write failure rolls back instead
        // of leaving a partial session. (Hardening beyond the Python port,
        // which commits per statement — D7/D8 demand the stronger guarantee.)
        self.con.execute_batch("BEGIN IMMEDIATE")?;
        let outcome = write_inner(self, session);
        match outcome {
            Ok(()) => self.con.execute_batch("COMMIT").map_err(StoreError::from),
            Err(e) => {
                let _ = self.con.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn read(&self, session_uid: &str) -> Result<Option<NormalizedSession>, StoreError> {
        let row: Option<(String, String, Option<String>)> = self
            .con
            .prepare("SELECT harness, source_ref, native_meta FROM sessions WHERE session_uid = ?")?
            .query_row(params![session_uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?;
        let Some((harness, source_ref, native_meta)) = row else {
            return Ok(None);
        };
        let mut s = NormalizedSession {
            session_uid: session_uid.to_owned(),
            harness,
            source_ref,
            turns: vec![],
            tools: vec![],
            reasoning: vec![],
            edges: vec![],
            states: vec![],
            human: vec![],
            native_meta: parse_map(native_meta),
        };
        let mut stmt = self.con.prepare(
            "SELECT turn_uid, session_uid, idx, role, content, timestamp_iso, native_model, native_fields FROM turns WHERE session_uid = ? ORDER BY idx",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            Ok(Turn {
                turn_uid: r.get(0)?,
                session_uid: r.get(1)?,
                index: r.get(2)?,
                role: r.get(3)?,
                content: r.get(4)?,
                timestamp_iso: r.get(5)?,
                native_model: r.get(6)?,
                native_fields: parse_map(r.get(7)?),
            })
        })? {
            let mut t = r?;
            t.content = self.get_payload(t.content);
            s.turns.push(t);
        }
        let mut stmt = self.con.prepare(
            "SELECT exec_uid, session_uid, turn_uid, idx, tool_name, call_id, command_or_args, raw_output, exit_code, is_error, parent_exec_uid, timestamp_iso, native_fields FROM tools WHERE session_uid = ? ORDER BY idx",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            Ok(ToolCall {
                exec_uid: r.get(0)?,
                session_uid: r.get(1)?,
                turn_uid: r.get(2)?,
                index: r.get(3)?,
                tool_name: r.get(4)?,
                call_id: r.get(5)?,
                command_or_args: r.get(6)?,
                raw_output: r.get(7)?,
                exit_code: r.get(8)?,
                is_error: r.get::<_, i64>(9)? != 0,
                parent_exec_uid: r.get(10)?,
                timestamp_iso: r.get(11)?,
                native_fields: parse_map(r.get(12)?),
            })
        })? {
            let mut t = r?;
            t.command_or_args = self.get_payload(t.command_or_args);
            t.raw_output = self.get_payload(t.raw_output);
            s.tools.push(t);
        }
        let mut stmt = self.con.prepare(
            "SELECT trace_uid, session_uid, turn_uid, content, subject, model, timestamp_iso FROM reasoning WHERE session_uid = ?",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            Ok(ReasoningTrace {
                trace_uid: r.get(0)?,
                session_uid: r.get(1)?,
                turn_uid: r.get(2)?,
                content: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                subject: r.get(4)?,
                model: r.get(5)?,
                timestamp_iso: r.get(6)?,
            })
        })? {
            let mut t = r?;
            t.content = self.get_payload(Some(t.content)).unwrap_or_default();
            s.reasoning.push(t);
        }
        let mut stmt = self.con.prepare(
            "SELECT edge_uid, session_uid, kind, source_uids, target_uid, provenance, payload, timestamp_iso FROM edges WHERE session_uid = ?",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })? {
            let (edge_uid, sess_uid, kind, sources, target_uid, prov, payload, ts) = r?;
            let source_uids: Vec<String> = serde_json::from_str(&sources)?;
            s.edges.push(Edge {
                edge_uid,
                session_uid: sess_uid,
                kind: kind_from(&kind)?,
                source_uids,
                target_uid,
                provenance: provenance_from(&prov)?,
                payload: parse_map(payload),
                timestamp_iso: ts,
            });
        }
        let mut stmt = self.con.prepare(
            "SELECT snapshot_uid, session_uid, turn_uid, idx, model, cwd, sandbox_policy, approval_policy, context_prompt_tokens, context_nonmessage_tokens, cumulative_tokens, compacted, native_fields, timestamp_iso FROM states WHERE session_uid = ? ORDER BY idx",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            let compacted: Option<i64> = r.get(11)?;
            Ok(StateSnapshot {
                snapshot_uid: r.get(0)?,
                session_uid: r.get(1)?,
                turn_uid: r.get(2)?,
                index: r.get(3)?,
                model: r.get(4)?,
                cwd: r.get(5)?,
                sandbox_policy: r.get(6)?,
                approval_policy: r.get(7)?,
                context_prompt_tokens: r.get(8)?,
                context_nonmessage_tokens: r.get(9)?,
                cumulative_tokens: r.get(10)?,
                compacted: compacted.map(|v| v != 0),
                native_fields: parse_map(r.get(12)?),
                timestamp_iso: r.get(13)?,
            })
        })? {
            s.states.push(r?);
        }
        let mut stmt = self.con.prepare(
            "SELECT human_uid, session_uid, turn_uid, text, kind, markers, timestamp_iso FROM human_signals WHERE session_uid = ?",
        )?;
        for r in stmt.query_map(params![session_uid], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })? {
            let (human_uid, sess_uid, turn_uid, text, kind, markers, ts) = r?;
            let mut h = HumanSignal {
                human_uid,
                session_uid: sess_uid,
                turn_uid,
                text,
                kind: correction_from(&kind)?,
                markers: markers
                    .and_then(|m| serde_json::from_str(&m).ok())
                    .unwrap_or_default(),
                timestamp_iso: ts,
            };
            h.text = self.get_payload(h.text);
            s.human.push(h);
        }
        Ok(Some(s))
    }

    fn count(&self, table: &str) -> Result<i64, StoreError> {
        if ![
            "sessions",
            "turns",
            "tools",
            "reasoning",
            "edges",
            "states",
            "human_signals",
            "blobs",
        ]
        .contains(&table)
        {
            return Err(StoreError::UnknownTable(table.to_owned()));
        }
        Ok(self
            .con
            .prepare(&format!("SELECT count(*) FROM {table}"))?
            .query_row([], |r| r.get(0))?)
    }
}

fn write_inner(store: &SqliteCanonicalStore, session: &NormalizedSession) -> Result<(), StoreError> {
        store.con.execute(
            "INSERT OR REPLACE INTO sessions (session_uid, harness, source_ref, native_meta) VALUES (?,?,?,?)",
            params![
                session.session_uid,
                session.harness,
                session.source_ref,
                json_map(&session.native_meta)
            ],
        )?;
        // Idempotent re-write: clear this session's rows first.
        for table in ["turns", "tools", "reasoning", "edges", "states", "human_signals"] {
            store.con.execute(
                &format!("DELETE FROM {table} WHERE session_uid = ?"),
                params![session.session_uid],
            )?;
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO turns (turn_uid, session_uid, idx, role, content, timestamp_iso, native_model, native_fields) VALUES (?,?,?,?,?,?,?,?)",
            )?;
            for t in &session.turns {
                stmt.execute(params![
                    t.turn_uid,
                    t.session_uid,
                    t.index,
                    t.role,
                    store.put_payload(t.content.as_deref())?,
                    t.timestamp_iso,
                    t.native_model,
                    json_map(&t.native_fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                ])?;
            }
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO tools (exec_uid, session_uid, turn_uid, idx, tool_name, call_id, command_or_args, raw_output, exit_code, is_error, parent_exec_uid, timestamp_iso, native_fields) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )?;
            for t in &session.tools {
                stmt.execute(params![
                    t.exec_uid,
                    t.session_uid,
                    t.turn_uid,
                    t.index,
                    t.tool_name,
                    t.call_id,
                    store.put_payload(t.command_or_args.as_deref())?,
                    store.put_payload(t.raw_output.as_deref())?,
                    t.exit_code,
                    i64::from(t.is_error),
                    t.parent_exec_uid,
                    t.timestamp_iso,
                    json_map(&t.native_fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                ])?;
            }
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO reasoning (trace_uid, session_uid, turn_uid, content, subject, model, timestamp_iso) VALUES (?,?,?,?,?,?,?)",
            )?;
            for r in &session.reasoning {
                stmt.execute(params![
                    r.trace_uid,
                    r.session_uid,
                    r.turn_uid,
                    store.put_payload(Some(&r.content))?,
                    r.subject,
                    r.model,
                    r.timestamp_iso
                ])?;
            }
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO edges (edge_uid, session_uid, kind, source_uids, target_uid, provenance, payload, timestamp_iso) VALUES (?,?,?,?,?,?,?,?)",
            )?;
            for e in &session.edges {
                stmt.execute(params![
                    e.edge_uid,
                    e.session_uid,
                    kind_str(e.kind),
                    json_list(&e.source_uids).unwrap_or_else(|| "[]".to_owned()),
                    e.target_uid,
                    provenance_str(e.provenance),
                    json_map(&e.payload.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                    e.timestamp_iso
                ])?;
            }
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO states (snapshot_uid, session_uid, turn_uid, idx, model, cwd, sandbox_policy, approval_policy, context_prompt_tokens, context_nonmessage_tokens, cumulative_tokens, compacted, native_fields, timestamp_iso) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )?;
            for st in &session.states {
                stmt.execute(params![
                    st.snapshot_uid,
                    st.session_uid,
                    st.turn_uid,
                    st.index,
                    st.model,
                    st.cwd,
                    st.sandbox_policy,
                    st.approval_policy,
                    st.context_prompt_tokens,
                    st.context_nonmessage_tokens,
                    st.cumulative_tokens,
                    st.compacted.map(i64::from),
                    json_map(&st.native_fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                    st.timestamp_iso
                ])?;
            }
        }
        {
            let mut stmt = store.con.prepare(
                "INSERT INTO human_signals (human_uid, session_uid, turn_uid, text, kind, markers, timestamp_iso) VALUES (?,?,?,?,?,?,?)",
            )?;
            for h in &session.human {
                stmt.execute(params![
                    h.human_uid,
                    h.session_uid,
                    h.turn_uid,
                    store.put_payload(h.text.as_deref())?,
                    correction_str(h.kind),
                    json_list(&h.markers),
                    h.timestamp_iso
                ])?;
            }
        }
        Ok(())
    }
