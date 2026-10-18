//! Hermes-Agent normalizer — the reference vertical.
//!
//! Strict port of `.../normalizers/hermes.py` (plus `hermes_contract.py` for
//! the FIELD_FATE map and `_human.py` for the classifier). Reads the frozen
//! capture read-only; the producer's schema is the source of truth.

use std::collections::HashMap;
use std::path::Path;

use chrono::{TimeZone, Utc};
use rusqlite::types::Value;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::conservation::ConservationContract;
use crate::domain::{
    Edge, EdgeKind, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot, ToolCall, Turn,
};
use crate::kernel::errors::SourceSchemaDrift;
use crate::kernel::ids::{
    edge_uid, hex_bytes, human_uid, reasoning_uid, state_uid, tool_uid, turn_uid,
};
use crate::ports::NormalizerPort;
use crate::session::NormalizedSession;

use super::contract::{self, PRODUCER_SCHEMA_SOURCE, PRODUCER_SCHEMA_VERSION};
use super::human::classify_human_turn;

pub const HARNESS: &str = "hermes";

/// Tables this normalizer READS. Absence is fail-closed (SourceSchemaDrift),
/// never a quiet default.
static REQUIRED_TABLES: &[&str] = &[
    "messages",
    "sessions",
    "session_model_usage",
    "system_prompts",
    "async_delegations",
    "state_meta",
];

/// The columns read from `messages`. Every other column has a declared fate
/// in the contract, so nothing is dropped without a named home.
static MESSAGE_COLUMNS: &[&str] = &[
    "id",
    "role",
    "content",
    "tool_call_id",
    "tool_calls",
    "tool_name",
    "effect_disposition",
    "timestamp",
    "finish_reason",
    "reasoning",
    "reasoning_content",
    "reasoning_details",
    "codex_reasoning_items",
    "codex_message_items",
    "platform_message_id",
    "observed",
    "_compressed_summary",
    "active",
    "compacted",
    "api_content",
    "display_kind",
    "display_metadata",
    "display_identity",
    "display_order",
    "message_uid",
    "absorbed_message_uids",
    "tool_call_uids",
    "tool_call_uid",
];

/// The four homes the reasoning concept can live in.
static REASONING_HOMES: &[&str] = &[
    "reasoning_content",
    "reasoning",
    "reasoning_details",
    "codex_reasoning_items",
];

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Integer(n) => serde_json::Value::from(*n),
        Value::Real(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Text(s) => serde_json::Value::String(s.clone()),
        Value::Blob(b) => serde_json::Value::String(hex_bytes(b)),
    }
}

/// Decode with `replace`: the producer stores arbitrary bytes in TEXT
/// columns, and at least 2 rows are not valid UTF-8. The row survives
/// (lossy bytes visible as U+FFFD), never a silent skip.
fn lossy(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Text(s) => Some(s.clone()),
        Value::Blob(b) => Some(String::from_utf8_lossy(b).into_owned()),
        Value::Integer(n) => Some(n.to_string()),
        Value::Real(f) => Some(f.to_string()),
    }
}

fn opt_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Null => None,
        Value::Integer(n) => Some(*n),
        Value::Real(f) => Some(*f as i64),
        Value::Text(s) => s.trim().parse().ok(),
        Value::Blob(_) => None,
    }
}

/// Native error signal: JSON `success: false` or non-empty `error` field.
fn tool_is_error(content: Option<&str>) -> bool {
    let Some(text) = content else {
        return false;
    };
    if text.is_empty() {
        return false;
    }
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(text) else {
        return false;
    };
    if map.get("success") == Some(&serde_json::Value::Bool(false)) {
        return true;
    }
    match map.get("error") {
        Some(serde_json::Value::String(s)) => !s.is_empty(),
        Some(v) => !v.is_null(),
        None => false,
    }
}

/// Unix seconds -> ISO-8601 UTC with trailing Z. 0/absent/unrepresentable -> None.
fn iso(ts: Option<i64>) -> Option<String> {
    let t = ts?;
    if t == 0 {
        return None;
    }
    Utc.timestamp_opt(t, 0)
        .single()
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Parse a producer JSON array of uids. `[]` for absent/malformed input —
/// a parse failure must never crash the session.
fn parse_uid_list(raw: Option<&str>) -> Vec<String> {
    let Some(text) = raw else {
        return vec![];
    };
    if text.is_empty() {
        return vec![];
    }
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(text) else {
        return vec![];
    };
    items
        .into_iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect()
}

fn json_object(map: &HashMap<String, Value>) -> serde_json::Value {
    serde_json::Value::Object(map.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect())
}

#[derive(Debug)]
pub struct HermesNormalizer {
    db_path: String,
}

impl HermesNormalizer {
    pub fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_owned(),
        }
    }

    fn connect(&self) -> Result<Connection, String> {
        let uri = format!("file:{}?mode=ro", self.db_path);
        let con = Connection::open_with_flags(
            uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|e| format!("open {}: {e}", self.db_path))?;
        let present: Vec<String> = con
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .and_then(|mut s| s.query_map([], |r| r.get(0))?.collect())
            .map_err(|e| format!("list tables: {e}"))?;
        let missing: Vec<&str> = REQUIRED_TABLES
            .iter()
            .filter(|t| !present.iter().any(|p| p == **t))
            .copied()
            .collect();
        if !missing.is_empty() {
            let drift = SourceSchemaDrift(format!(
                "{}: source is missing in-scope table(s) {missing:?}; \
                 the contract was derived from a different schema",
                self.db_path
            ));
            return Err(drift.to_string());
        }
        Ok(con)
    }

    fn read_map(
        con: &Connection,
        sql: &str,
        param: &str,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let cols: Vec<String> = con
            .prepare(sql)
            .map_err(|e| e.to_string())?
            .column_names()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let mut stmt = con.prepare(sql).map_err(|e| e.to_string())?;
        stmt.query_map([param], |r| {
            let mut map = HashMap::with_capacity(cols.len());
            for (i, name) in cols.iter().enumerate() {
                map.insert(name.clone(), r.get(i)?);
            }
            Ok(map)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())
    }
}

impl NormalizerPort for HermesNormalizer {
    fn harness(&self) -> &str {
        HARNESS
    }

    fn can_read(&self, source_ref: &str) -> bool {
        source_ref.ends_with(".db") || source_ref.ends_with(".sqlite")
    }

    fn conservation(&self) -> Option<ConservationContract> {
        Some(contract::contract())
    }

    fn session_refs(&self) -> Vec<String> {
        if !Path::new(&self.db_path).exists() {
            return vec![];
        }
        let Ok(con) = self.connect() else {
            return vec![];
        };
        con.prepare(
            "SELECT id FROM sessions \
             UNION \
             SELECT DISTINCT session_id FROM messages WHERE session_id IS NOT NULL \
             ORDER BY 1",
        )
        .and_then(|mut s| s.query_map([], |r| r.get::<_, String>(0))?.collect())
        .unwrap_or_default()
    }

    fn normalize(&self, session_ref: &str) -> Result<NormalizedSession, String> {
        let session_uid = format!("{HARNESS}:{session_ref}");
        let con = self.connect()?;

        let session_cols: Vec<String> = con
            .prepare("SELECT * FROM sessions LIMIT 0")
            .map_err(|e| e.to_string())?
            .column_names()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let session_row: Option<HashMap<String, Value>> = con
            .prepare("SELECT * FROM sessions WHERE id = ?")
            .map_err(|e| e.to_string())?
            .query_row([session_ref], |r| {
                let mut map = HashMap::with_capacity(session_cols.len());
                for (i, name) in session_cols.iter().enumerate() {
                    map.insert(name.clone(), r.get(i)?);
                }
                Ok(map)
            })
            .optional()
            .map_err(|e| e.to_string())?;

        let usage_rows = Self::read_map(
            &con,
            "SELECT * FROM session_model_usage WHERE session_id = ? ORDER BY first_seen",
            session_ref,
        )?;

        let msg_sql = format!(
            "SELECT {} FROM messages WHERE session_id = ? ORDER BY id ASC",
            MESSAGE_COLUMNS.join(", ")
        );
        let msg_rows = Self::read_map(&con, &msg_sql, session_ref)?;

        let mut order_stmt = con
            .prepare("SELECT id, display_order FROM messages WHERE session_id = ?")
            .map_err(|e| e.to_string())?;
        let display_orders: HashMap<i64, Option<i64>> = order_stmt
            .query_map([session_ref], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
            .unwrap_or_default();

        let sp_hash: Option<String> = session_row
            .as_ref()
            .and_then(|m| m.get("system_prompt_hash"))
            .and_then(lossy);
        let system_prompt_resolved: Option<HashMap<String, Value>> = match sp_hash {
            Some(h) => con
                .prepare("SELECT hash, prompt FROM system_prompts WHERE hash = ?")
                .map_err(|e| e.to_string())?
                .query_row([h], |r| {
                    let mut m = HashMap::new();
                    m.insert("hash".to_owned(), r.get(0)?);
                    m.insert("prompt".to_owned(), r.get(1)?);
                    Ok(m)
                })
                .optional()
                .map_err(|e| e.to_string())?,
            None => None,
        };

        let delegations: Vec<HashMap<String, Value>> = Self::read_map(
            &con,
            "SELECT * FROM async_delegations WHERE origin_session = ? ORDER BY dispatched_at",
            session_ref,
        )?
        .into_iter()
        .map(|mut m| {
            m.remove("owner_pid");
            m.remove("owner_started_at");
            m
        })
        .collect();

        let mut meta_stmt = con
            .prepare("SELECT key, value FROM state_meta")
            .map_err(|e| e.to_string())?;
        let store_meta: HashMap<String, String> = meta_stmt
            .query_map([], |r| {
                Ok((
                    lossy(&r.get::<_, Value>(0)?).unwrap_or_default(),
                    lossy(&r.get::<_, Value>(1)?).unwrap_or_default(),
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;

        let mut distinct_orders: Vec<i64> = display_orders
            .values()
            .filter_map(|o| *o)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        distinct_orders.sort_unstable();
        let order_rank: HashMap<i64, usize> = distinct_orders
            .into_iter()
            .enumerate()
            .map(|(i, o)| (o, i))
            .collect();

        let mut models_in_session: Vec<serde_json::Value> = Vec::new();
        for u in &usage_rows {
            let m = u.get("model").map(value_to_json).unwrap_or_default();
            if !models_in_session.contains(&m) {
                models_in_session.push(m);
            }
        }
        let session_meta: HashMap<String, Value> = session_row.clone().unwrap_or_default();
        let native_model: Option<String> = session_meta
            .get("model")
            .and_then(lossy);

        let mut native_meta: HashMap<String, serde_json::Value> = HashMap::new();
        native_meta.insert(
            "native_model".into(),
            native_model
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        );
        native_meta.insert(
            "models_in_session".into(),
            serde_json::Value::Array(models_in_session.clone()),
        );
        native_meta.insert(
            "model_usage".into(),
            serde_json::Value::Array(usage_rows.iter().map(json_object).collect()),
        );
        native_meta.insert(
            "multi_model".into(),
            serde_json::Value::Bool(models_in_session.len() > 1),
        );
        native_meta.insert("session".into(), json_object(&session_meta));
        native_meta.insert(
            "system_prompt_resolved".into(),
            system_prompt_resolved
                .as_ref()
                .map_or(serde_json::Value::Null, json_object),
        );
        native_meta.insert(
            "async_delegations".into(),
            serde_json::Value::Array(delegations.iter().map(json_object).collect()),
        );
        native_meta.insert(
            "store_meta".into(),
            serde_json::Value::Object(
                store_meta
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            ),
        );
        native_meta.insert(
            "ordering".into(),
            serde_json::Value::String("messages.id".into()),
        );
        native_meta.insert(
            "producer_schema_version".into(),
            serde_json::Value::from(PRODUCER_SCHEMA_VERSION),
        );
        native_meta.insert(
            "producer_schema_source".into(),
            serde_json::Value::String(PRODUCER_SCHEMA_SOURCE.into()),
        );

        let contract = contract::contract();
        let reasoning = contract.concept("reasoning").cloned();

        let mut sess = NormalizedSession {
            session_uid: session_uid.clone(),
            harness: HARNESS.into(),
            source_ref: format!("{}#{session_ref}", self.db_path),
            turns: vec![],
            tools: vec![],
            reasoning: vec![],
            edges: vec![],
            states: vec![],
            human: vec![],
            native_meta,
        };

        let mut prev_turn: Option<String> = None;
        let mut call_id_to_exec: HashMap<String, String> = HashMap::new();
        let mut turn_idx: i64 = 0;
        let mut tool_idx: i64 = 0;
        let mut reason_idx: i64 = 0;
        let mut edge_idx: i64 = 0;
        let mut state_idx: i64 = 0;
        let mut drift: Vec<String> = vec![];

        let get = |row: &HashMap<String, Value>, col: &str| -> Value {
            row.get(col).cloned().unwrap_or(Value::Null)
        };

        for row in &msg_rows {
            let t_uid = turn_uid(&session_uid, turn_idx);
            let ts_iso = iso(opt_i64(&get(row, "timestamp")));
            let role = lossy(&get(row, "role"))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "unknown".into());

            if let Some(prev) = &prev_turn {
                sess.edges.push(Edge {
                    edge_uid: edge_uid(&session_uid, "sequence", edge_idx),
                    session_uid: session_uid.clone(),
                    kind: EdgeKind::Sequence,
                    source_uids: vec![prev.clone()],
                    target_uid: Some(t_uid.clone()),
                    provenance: EdgeProvenance::Derived,
                    payload: [(
                        "rule".to_owned(),
                        serde_json::Value::from("messages.id order"),
                    )]
                    .into(),
                    timestamp_iso: ts_iso.clone(),
                });
                edge_idx += 1;
            }

            let mut turn_fields: HashMap<String, serde_json::Value> = [
                ("native_id", value_to_json(&get(row, "id"))),
                (
                    "logical_index",
                    opt_i64(&get(row, "display_order"))
                        .and_then(|o| order_rank.get(&o))
                        .map(|i| serde_json::Value::from(*i as i64))
                        .unwrap_or(serde_json::Value::Null),
                ),
                ("finish_reason", value_to_json(&get(row, "finish_reason"))),
                (
                    "effect_disposition",
                    value_to_json(&get(row, "effect_disposition")),
                ),
                (
                    "codex_message_items",
                    value_to_json(&get(row, "codex_message_items")),
                ),
                (
                    "platform_message_id",
                    value_to_json(&get(row, "platform_message_id")),
                ),
                ("observed", value_to_json(&get(row, "observed"))),
                (
                    "_compressed_summary",
                    value_to_json(&get(row, "_compressed_summary")),
                ),
                ("active", value_to_json(&get(row, "active"))),
                ("api_content", value_to_json(&get(row, "api_content"))),
                ("display_kind", value_to_json(&get(row, "display_kind"))),
                (
                    "display_metadata",
                    value_to_json(&get(row, "display_metadata")),
                ),
                (
                    "display_identity",
                    value_to_json(&get(row, "display_identity")),
                ),
                ("display_order", value_to_json(&get(row, "display_order"))),
                ("message_uid", value_to_json(&get(row, "message_uid"))),
                (
                    "absorbed_message_uids",
                    value_to_json(&get(row, "absorbed_message_uids")),
                ),
                (
                    "tool_call_uids",
                    value_to_json(&get(row, "tool_call_uids")),
                ),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
            for home in REASONING_HOMES {
                if !matches!(get(row, home), Value::Null) {
                    turn_fields.insert(
                        format!("reasoning_home.{home}"),
                        value_to_json(&get(row, home)),
                    );
                }
            }

            sess.turns.push(Turn {
                turn_uid: t_uid.clone(),
                session_uid: session_uid.clone(),
                index: turn_idx,
                role: role.clone(),
                content: lossy(&get(row, "content")),
                timestamp_iso: ts_iso.clone(),
                native_model: native_model.clone(),
                native_fields: turn_fields,
            });

            if let Some(concept) = &reasoning {
                let home_map: HashMap<String, serde_json::Value> = REASONING_HOMES
                    .iter()
                    .map(|h| ((*h).to_owned(), value_to_json(&get(row, h))))
                    .collect();
                let (home, value) = concept.resolve(&home_map);
                if let (Some(h), Some(v)) = (home, value) {
                    let expected = concept.expected_home(native_model.as_deref());
                    if let Some(exp) = expected {
                        if !exp.is_empty() && exp != h {
                            drift.push(format!(
                                "{}: reasoning in {h}, expected {exp}",
                                native_model.clone().unwrap_or_default()
                            ));
                        }
                    }
                    let content = match &v {
                        serde_json::Value::String(s) => s.clone(),
                        other => serde_json::to_string(other).unwrap_or_default(),
                    };
                    sess.reasoning.push(ReasoningTrace {
                        trace_uid: reasoning_uid(&session_uid, reason_idx),
                        session_uid: session_uid.clone(),
                        turn_uid: Some(t_uid.clone()),
                        content,
                        subject: Some(format!("Hermes CoT ({h})")),
                        model: native_model.clone(),
                        timestamp_iso: ts_iso.clone(),
                    });
                    reason_idx += 1;
                }
            }

            if let Some(tcalls_raw) = lossy(&get(row, "tool_calls")) {
                if !tcalls_raw.is_empty() {
                    if let Ok(serde_json::Value::Array(tcalls)) =
                        serde_json::from_str::<serde_json::Value>(&tcalls_raw)
                    {
                        for tc in tcalls {
                            let (func_name, func_args, call_id) = match &tc {
                                serde_json::Value::Object(map) => {
                                    let func = map.get("function");
                                    (
                                        func.and_then(|f| f.get("name"))
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("tool")
                                            .to_owned(),
                                        func.and_then(|f| f.get("arguments"))
                                            .and_then(|a| a.as_str())
                                            .unwrap_or("")
                                            .to_owned(),
                                        map.get("id")
                                            .and_then(|i| i.as_str())
                                            .unwrap_or("")
                                            .to_owned(),
                                    )
                                }
                                _ => ("tool".to_owned(), String::new(), String::new()),
                            };
                            let exec_uid = tool_uid(&session_uid, tool_idx);
                            sess.tools.push(ToolCall {
                                exec_uid: exec_uid.clone(),
                                session_uid: session_uid.clone(),
                                turn_uid: Some(t_uid.clone()),
                                index: tool_idx,
                                tool_name: func_name.clone(),
                                call_id: if call_id.is_empty() {
                                    None
                                } else {
                                    Some(call_id.clone())
                                },
                                command_or_args: Some(func_args),
                                raw_output: None,
                                exit_code: None,
                                is_error: false,
                                parent_exec_uid: None,
                                timestamp_iso: ts_iso.clone(),
                                native_fields: [(
                                    "tool_call_uid".to_owned(),
                                    value_to_json(&get(row, "tool_call_uid")),
                                )]
                                .into(),
                            });
                            if !call_id.is_empty() {
                                call_id_to_exec.insert(call_id.clone(), exec_uid);
                            }
                            sess.edges.push(Edge {
                                edge_uid: edge_uid(&session_uid, "tool_call", edge_idx),
                                session_uid: session_uid.clone(),
                                kind: EdgeKind::ToolCall,
                                source_uids: vec![t_uid.clone()],
                                target_uid: None,
                                provenance: EdgeProvenance::Native,
                                payload: [
                                    (
                                        "tool_name".to_owned(),
                                        serde_json::Value::from(func_name),
                                    ),
                                    ("call_id".to_owned(), serde_json::Value::from(call_id)),
                                ]
                                .into(),
                                timestamp_iso: ts_iso.clone(),
                            });
                            edge_idx += 1;
                            tool_idx += 1;
                        }
                    }
                }
            }

            if role == "tool" {
                let call_id = lossy(&get(row, "tool_call_id")).unwrap_or_default();
                let parent = call_id_to_exec.get(&call_id).cloned();
                let content = lossy(&get(row, "content"));
                sess.tools.push(ToolCall {
                    exec_uid: tool_uid(&session_uid, tool_idx),
                    session_uid: session_uid.clone(),
                    turn_uid: Some(t_uid.clone()),
                    index: tool_idx,
                    tool_name: "tool_result".into(),
                    call_id: if call_id.is_empty() {
                        None
                    } else {
                        Some(call_id.clone())
                    },
                    command_or_args: None,
                    raw_output: content.clone(),
                    exit_code: None,
                    is_error: tool_is_error(content.as_deref()),
                    parent_exec_uid: parent.clone(),
                    timestamp_iso: ts_iso.clone(),
                    native_fields: [
                        (
                            "native_tool_name".to_owned(),
                            value_to_json(&get(row, "tool_name")),
                        ),
                        (
                            "tool_call_uid".to_owned(),
                            value_to_json(&get(row, "tool_call_uid")),
                        ),
                    ]
                    .into(),
                });
                let (sources, provenance) = match parent {
                    Some(p) => (vec![p], EdgeProvenance::Native),
                    None => (vec![t_uid.clone()], EdgeProvenance::Derived),
                };
                sess.edges.push(Edge {
                    edge_uid: edge_uid(&session_uid, "tool_result", edge_idx),
                    session_uid: session_uid.clone(),
                    kind: EdgeKind::ToolResult,
                    source_uids: sources,
                    target_uid: Some(t_uid.clone()),
                    provenance,
                    payload: [(
                        "tool_call_id".to_owned(),
                        serde_json::Value::from(call_id),
                    )]
                    .into(),
                    timestamp_iso: ts_iso.clone(),
                });
                edge_idx += 1;
                tool_idx += 1;
            }

            let compacted = opt_i64(&get(row, "compacted")).is_some_and(|v| v != 0);
            let superseded = opt_i64(&get(row, "active")) == Some(0);
            if compacted || superseded {
                sess.states.push(StateSnapshot {
                    snapshot_uid: state_uid(&session_uid, state_idx),
                    session_uid: session_uid.clone(),
                    turn_uid: Some(t_uid.clone()),
                    index: state_idx,
                    model: None,
                    cwd: None,
                    sandbox_policy: None,
                    approval_policy: None,
                    context_prompt_tokens: None,
                    context_nonmessage_tokens: None,
                    cumulative_tokens: None,
                    compacted: if compacted { Some(true) } else { None },
                    native_fields: [
                        ("active".to_owned(), value_to_json(&get(row, "active"))),
                        (
                            "superseded_by_compaction".to_owned(),
                            serde_json::Value::Bool(superseded),
                        ),
                        (
                            "_compressed_summary".to_owned(),
                            value_to_json(&get(row, "_compressed_summary")),
                        ),
                    ]
                    .into(),
                    timestamp_iso: ts_iso.clone(),
                });
                if compacted {
                    sess.edges.push(Edge {
                        edge_uid: edge_uid(&session_uid, "compaction", edge_idx),
                        session_uid: session_uid.clone(),
                        kind: EdgeKind::Compaction,
                        source_uids: vec![t_uid.clone()],
                        target_uid: Some(t_uid.clone()),
                        provenance: EdgeProvenance::Native,
                        payload: [(
                            "trigger".to_owned(),
                            serde_json::Value::from("hermes_context_compaction"),
                        )]
                        .into(),
                        timestamp_iso: ts_iso.clone(),
                    });
                    edge_idx += 1;
                }
                state_idx += 1;
            }

            let absorbed =
                parse_uid_list(lossy(&get(row, "absorbed_message_uids")).as_deref());
            if !absorbed.is_empty() {
                let payload_uids: Vec<serde_json::Value> = absorbed
                    .iter()
                    .cloned()
                    .map(serde_json::Value::from)
                    .collect();
                sess.edges.push(Edge {
                    edge_uid: edge_uid(&session_uid, "compaction_absorb", edge_idx),
                    session_uid: session_uid.clone(),
                    kind: EdgeKind::Compaction,
                    source_uids: absorbed.clone(),
                    target_uid: Some(t_uid.clone()),
                    provenance: EdgeProvenance::Native,
                    payload: [(
                        "absorbed_message_uids".to_owned(),
                        serde_json::Value::Array(payload_uids),
                    )]
                    .into(),
                    timestamp_iso: ts_iso.clone(),
                });
                edge_idx += 1;
            }

            if role == "user" {
                let (kind, markers) =
                    classify_human_turn(&lossy(&get(row, "content")).unwrap_or_default());
                sess.human.push(HumanSignal {
                    human_uid: human_uid(&session_uid, &t_uid),
                    session_uid: session_uid.clone(),
                    turn_uid: t_uid.clone(),
                    text: lossy(&get(row, "content")),
                    kind,
                    markers,
                    timestamp_iso: ts_iso.clone(),
                });
            }

            prev_turn = Some(t_uid);
            turn_idx += 1;
        }

        if !drift.is_empty() {
            sess.native_meta.insert(
                "reasoning_drift".into(),
                serde_json::Value::Array(
                    drift.into_iter().map(serde_json::Value::from).collect(),
                ),
            );
        }

        sess.check().map_err(|e| e.to_string())?;
        Ok(sess)
    }
}
