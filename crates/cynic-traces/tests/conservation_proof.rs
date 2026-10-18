//! T4 exit gate: the Rust canonical store + conservation runner replace
//! `verify_conservation.py`.
//!
//! Full load of the frozen capture into a temp store, then:
//! source-vs-canonical counts, contract closure (alpha1–alpha4) on the live
//! DDL, store round-trip on samples, old-silver comparison when present.
//! Fails closed on any divergence. Skips when the frozen capture is absent.

use std::path::PathBuf;

use cynic_traces::adapters::hermes::contract;
use cynic_traces::conservation::parse_ddl_columns;
use cynic_traces::ports::{CanonicalStorePort, NormalizerPort};
use cynic_traces::usecases::normalize_source;
use cynic_traces::{HermesNormalizer, SqliteCanonicalStore};

// Absolute defaults are admitted debt (M3): every path below is overridable
// via environment so CI and other machines do not depend on this host.
fn source_path() -> String {
    std::env::var("CYNIC_TRACES_SOURCE").unwrap_or_else(|_| SOURCE.into())
}

fn silver_path() -> String {
    std::env::var("CYNIC_TRACES_SILVER").unwrap_or_else(|_| SILVER.into())
}

fn producer_src_path() -> String {
    std::env::var("CYNIC_PRODUCER_SRC")
        .unwrap_or_else(|_| "/home/user/.hermes/hermes-agent/hermes_state_common.py".into())
}

const SOURCE: &str = "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite";
const SILVER: &str = "/var/lib/cynic/vault/corpus/silver/corpus_master.sqlite";

fn connect_ro(path: &str) -> rusqlite::Connection {
    rusqlite::Connection::open_with_flags(
        format!("file:{path}?mode=ro"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap()
}

fn count(con: &rusqlite::Connection, sql: &str) -> i64 {
    con.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn full_load_conservation_proof() {
    let source = source_path();
    if !PathBuf::from(&source).exists() {
        eprintln!("frozen capture absent, skipping conservation proof");
        return;
    }
    let tmp = std::env::temp_dir().join(format!("cynic-traces-t4-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let db_path = tmp.join("canonical.sqlite");
    let db_str = db_path.to_str().unwrap();

    let normalizer = HermesNormalizer::new(&source);
    let refs = normalizer.session_refs();
    assert!(!refs.is_empty(), "frozen capture has no sessions");

    let store = SqliteCanonicalStore::new(db_str, None, true).unwrap();
    let report = normalize_source(&normalizer, &store, 0);
    assert!(
        report.failures.is_empty(),
        "failed sessions: {:?}",
        &report.failures[..report.failures.len().min(5)]
    );
    assert_eq!(report.sessions_failed, 0);

    // ---- source vs canonical counts ----
    let src = connect_ro(&source);
    let src_sessions: i64 =
        count(&src, "SELECT count(DISTINCT session_id) FROM messages");
    let src_msgs: i64 = count(&src, "SELECT count(*) FROM messages");
    let src_user: i64 = count(&src, "SELECT count(*) FROM messages WHERE role='user'");
    // Any home NOT NULL (the concept's own rule: first non-null home wins,
    // empty string included — NOT the reasoning_content-only count, which
    // undercounts by design: it misses the other three homes).
    let src_reason: i64 = count(
        &src,
        "SELECT count(*) FROM messages WHERE reasoning_content IS NOT NULL OR reasoning IS NOT NULL OR reasoning_details IS NOT NULL OR codex_reasoning_items IS NOT NULL",
    );
    let src_compact: i64 =
        count(&src, "SELECT count(*) FROM messages WHERE compacted=1 OR active=0");

    // Canonical sessions = producer registry UNION orphan messages (366
    // message-less sessions carry session truth): self-consistency first,
    // then the source-anchored counts.
    assert_eq!(store.count("sessions").unwrap(), refs.len() as i64);
    assert_eq!(store.count("turns").unwrap(), src_msgs);
    assert_eq!(store.count("human_signals").unwrap(), src_user);
    assert_eq!(store.count("reasoning").unwrap(), src_reason);
    assert_eq!(store.count("states").unwrap(), src_compact);
    assert!(
        store.count("edges").unwrap() > 0,
        "edges were 0 in the old pipeline"
    );
    let _ = src_sessions;

    // ---- contract closure (alpha1–alpha4), Python-exact gates ----
    //
    // The observed schema comes from the FROZEN producer snapshot
    // (tests/parity/producer_schema.sql), never from a live path: the proof
    // is hermetic. When the live producer source exists, its DDL must match
    // the snapshot byte-for-byte — a mismatch means the producer bumped and
    // the snapshot + contract must be re-derived (see tools/).
    let parity_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity");
    let snapshot = std::fs::read_to_string(parity_dir.join("producer_schema.sql")).unwrap();
    let mut snapshot_lines = snapshot.lines();
    let header = snapshot_lines.next().unwrap_or("");
    let producer_version: i64 = header
        .split("SCHEMA_VERSION = ")
        .nth(1)
        .unwrap_or_else(|| panic!("snapshot header has no SCHEMA_VERSION"))
        .trim()
        .trim_matches(|c| c == ')' || c == '.')
        .parse()
        .unwrap();
    let snapshot_ddl: String = snapshot_lines.skip(1).collect::<Vec<_>>().join("\n");
    let live_src = producer_src_path();
    if PathBuf::from(&live_src).exists() {
        let live_text = std::fs::read_to_string(&live_src).unwrap();
        let ddl_re = regex::Regex::new(r#"(?ms)^SCHEMA_SQL\s*=\s*"""(.*?)"""#).unwrap();
        let live_ddl = ddl_re
            .captures(&live_text)
            .unwrap_or_else(|| panic!("SCHEMA_SQL not found in {live_src}"))
            .get(1)
            .unwrap()
            .as_str();
        assert_eq!(
            live_ddl.trim_end(),
            snapshot_ddl.trim_end(),
            "producer bumped its DDL: re-freeze with tools/snapshot_producer_schema.py and re-derive the contract"
        );
    }
    let observed = parse_ddl_columns(&snapshot_ddl);
    let contract = contract::contract();
    assert_eq!(
        producer_version, contract.producer_schema_version,
        "producer bumped SCHEMA_VERSION: contract is stale"
    );
    assert!(
        contract.unmodeled_tables(&observed).is_empty(),
        "unmodeled tables: {:?}",
        contract.unmodeled_tables(&observed)
    );
    assert!(
        contract.uncovered_columns(&observed).is_empty(),
        "uncovered columns: {:?}",
        contract.uncovered_columns(&observed)
    );
    assert!(
        contract.out_of_scope_without_reason().is_empty(),
        "out-of-scope without reason: {:?}",
        contract.out_of_scope_without_reason()
    );
    assert!(
        contract.stale_out_of_scope(&observed).is_empty(),
        "stale out-of-scope: {:?}",
        contract.stale_out_of_scope(&observed)
    );
    assert!(
        contract.stale_fates(&observed).is_empty(),
        "fates for columns the producer does not have: {:?}",
        contract.stale_fates(&observed)
    );
    assert!(
        contract.malformed_fates().is_empty(),
        "malformed fates: {:?}",
        contract.malformed_fates()
    );
    // The store's own anchor agrees with the frozen capture.
    let schema_version: i64 = count(&src, "SELECT version FROM schema_version");
    assert_eq!(
        schema_version, contract.producer_schema_version,
        "frozen capture schema disagrees with the contract"
    );

    // ---- round-trip: store -> entities -> check + stats ----
    for session_ref in refs.iter().take(3) {
        let session_uid = format!("hermes:{session_ref}");
        let back = store
            .read(&session_uid)
            .unwrap()
            .unwrap_or_else(|| panic!("{session_uid} missing after write"));
        back.check().unwrap();
        let direct = normalizer.normalize(session_ref).unwrap();
        assert_eq!(back.stats(), direct.stats(), "{session_uid} round-trip");
    }

    // ---- old silver vs new canonical (when present) ----
    let silver = silver_path();
    if PathBuf::from(&silver).exists() {
        let sil = connect_ro(&silver);
        let old_turns: i64 = count(
            &sil,
            "SELECT count(*) FROM dialogue_turns WHERE agent_family='hermes'",
        );
        let old_reason: i64 = count(
            &sil,
            "SELECT count(*) FROM reasoning_traces WHERE session_uid LIKE 'hermes:%'",
        );
        assert!(
            store.count("turns").unwrap() >= old_turns,
            "new canonical lost turns vs old silver"
        );
        assert!(
            store.count("reasoning").unwrap() >= old_reason,
            "new canonical lost reasoning vs old silver"
        );
        assert!(
            store.count("human_signals").unwrap() > 0,
            "human signal was untagged (0) in the old pipeline"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}
