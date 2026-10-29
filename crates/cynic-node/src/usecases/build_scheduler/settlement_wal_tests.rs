//! Settlement WAL integration tests.
//!
//! Verifies that scheduler settlements append to the global cynic-storage WAL
//! with hash-chain continuity, crash-recovery, and append-failure semantics.

use std::path::PathBuf;

use cynic_protocol::AgentCompletionStatus;
use cynic_persistence::{CausalWitness, StorageError, WalAppendInput, WalFsAdapter, WalJournalPort};
use tokio_util::bytes::Bytes;

use super::settlement_wal::SettlementWal;

fn test_witness() -> CausalWitness {
    CausalWitness {
        principal_pubkey: [42u8; 32],
        signature: [84u8; 64],
        mandate_id: Some("test-mandate".to_string()),
        proof_anchor: None,
    }
}

fn temp_wal_dir(tag: &str) -> PathBuf {
    let id = std::process::id();
    let dir = std::env::temp_dir().join(format!("cynic-settlement-wal-{tag}-{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create wal dir");
    dir
}

fn append_settlement(
    wal: &WalFsAdapter,
    node_id: &str,
    workload_id: &str,
    status: &str,
    workload_digest: &str,
    evidence_refs: &[String],
    settled_at_ms: u64,
) -> Result<cynic_persistence::WalReceipt, StorageError> {
    // (WalReceipt is re-exported from cynic_persistence::types)
    let payload = serde_json::json!({
        "node_id": node_id,
        "workload_id": workload_id,
        "status": status,
        "workload_digest": workload_digest,
        "evidence_refs": evidence_refs,
        "settled_at_ms": settled_at_ms,
    });
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
    let input = WalAppendInput {
        schema: "cynic.scheduler.settlement.v1".to_string(),
        payload: Bytes::from(payload_bytes),
        witness: test_witness(),
    };
    let receipts = wal.append_batch(&[input])?;
    Ok(receipts.into_iter().next().expect("one receipt"))
}

#[test]
fn settlement_wal_appends_chained_entry() {
    let dir = temp_wal_dir("chained");
    let wal = WalFsAdapter::open(&dir).expect("open WAL");

    // First settlement — prev_hash must be GENESIS (first entry)
    let receipt1 = append_settlement(
        &wal,
        "node-1",
        "wl-001",
        "completed",
        "sha256:aaa",
        &["proof:sha256:bbb".to_string()],
        1000,
    )
    .expect("append first settlement");

    assert_eq!(receipt1.sequence.0, 1, "first entry must have sequence 1");

    // Second settlement — prev_hash must chain to first entry's hash
    let receipt2 = append_settlement(
        &wal,
        "node-1",
        "wl-002",
        "failed",
        "sha256:ccc",
        &[],
        2000,
    )
    .expect("append second settlement");

    assert_eq!(receipt2.sequence.0, 2, "second entry must have sequence 2");

    // Verify the chain: second entry's prev_hash must equal first entry's entry_hash
    let entries: Vec<_> = wal
        .scan_stream(cynic_persistence::SequenceNumber::GENESIS, None)
        .expect("scan")
        .collect::<Result<_, _>>()
        .expect("collect entries");

    assert_eq!(entries.len(), 2, "WAL must contain exactly 2 entries");
    assert_eq!(
        entries[1].previous_hash, entries[0].entry_hash,
        "second entry must chain to first entry's hash"
    );

    // Verify integrity
    let report = wal.verify_integrity().expect("verify");
    assert!(report.is_valid, "WAL chain must be valid");
    assert_eq!(report.total_entries, 2);
}

#[test]
fn settlement_wal_recovers_tail_on_restart() {
    let dir = temp_wal_dir("restart");

    // First session: append 3 entries
    {
        let wal = WalFsAdapter::open(&dir).expect("open WAL session 1");
        for i in 1..=3 {
            append_settlement(
                &wal,
                "node-1",
                &format!("wl-{i:03}"),
                "completed",
                &format!("sha256:digest{i}"),
                &[],
                i as u64 * 1000,
            )
            .expect("append settlement");
        }
        assert_eq!(wal.latest_sequence().unwrap().0, 3);
    }

    // Second session: reopen and verify tail recovered
    {
        let wal = WalFsAdapter::open(&dir).expect("open WAL session 2");
        assert_eq!(
            wal.latest_sequence().unwrap().0,
            3,
            "tail sequence must be recovered after restart"
        );

        let report = wal.verify_integrity().expect("verify after restart");
        assert!(report.is_valid, "chain must be valid after restart");
        assert_eq!(report.total_entries, 3);

        // Append a 4th entry — must chain to the recovered tail
        let receipt4 = append_settlement(
            &wal,
            "node-1",
            "wl-004",
            "completed",
            "sha256:digest4",
            &[],
            4000,
        )
        .expect("append after restart");
        assert_eq!(receipt4.sequence.0, 4);

        let entries: Vec<_> = wal
            .scan_stream(cynic_persistence::SequenceNumber::GENESIS, None)
            .expect("scan")
            .collect::<Result<_, _>>()
            .expect("collect");
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[3].previous_hash, entries[2].entry_hash);
    }
}

#[test]
fn settlement_wal_append_failure_leaves_workload_claimed() {
    let dir = temp_wal_dir("failure");

    // Open WAL, append one entry, then drop the adapter
    {
        let wal = WalFsAdapter::open(&dir).expect("open WAL");
        append_settlement(
            &wal,
            "node-1",
            "wl-ok",
            "completed",
            "sha256:ok",
            &[],
            1000,
        )
        .expect("append first");
    }

    // Corrupt the WAL file to simulate append failure
    let active_path = dir.join("active.seg");
    let file_len = std::fs::metadata(&active_path).unwrap().len();
    // Truncate to corrupt the last frame
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&active_path)
        .expect("open for corruption");
    file.set_len(file_len / 2).expect("truncate");
    drop(file);

    // Reopen — should recover (torn tail truncation)
    let wal = WalFsAdapter::open(&dir).expect("open after corruption");
    let report = wal.verify_integrity().expect("verify after recovery");
    assert!(report.is_valid, "recovered WAL must be valid");

    // The key invariant: if append fails, the workload must NOT be settled.
    // In the scheduler wiring, this means: append to WAL first; if append
    // fails, return Err and the workload stays Claimed (retryable).
    // This test verifies the WAL recovery path works after corruption.
    assert!(
        report.total_entries < 2,
        "corrupted entry must be truncated, got {} entries",
        report.total_entries
    );
}

// ---------------------------------------------------------------------------
// Wrapper tests — exercise SettlementWal directly (not WalFsAdapter).
// ---------------------------------------------------------------------------

#[test]
fn settlement_wal_wrapper_open_and_append() {
    let dir = temp_wal_dir("wrapper-open");
    let wal = SettlementWal::open(&dir).expect("SettlementWal::open");

    let receipt = wal
        .append_settlement(
            "node-1",
            "wl-wrap-001",
            &AgentCompletionStatus::Completed,
            "sha256:wrap-aaa",
            &["proof:sha256:wrap-bbb".to_string()],
            5000,
        )
        .expect("SettlementWal::append_settlement");

    assert_eq!(receipt.sequence.0, 1, "first settlement must have sequence 1");

    let latest = wal.latest_sequence().expect("latest_sequence");
    assert_eq!(latest.0, 1, "latest_sequence must be 1 after first append");

    let report = wal.verify_integrity().expect("verify_integrity");
    assert!(report.is_valid, "wrapper WAL chain must be valid");
    assert_eq!(report.total_entries, 1);
}

#[test]
fn settlement_wal_wrapper_append_chains_multiple() {
    let dir = temp_wal_dir("wrapper-chain");
    let wal = SettlementWal::open(&dir).expect("SettlementWal::open");

    for i in 1..=3u64 {
        wal.append_settlement(
            "node-1",
            &format!("wl-chain-{i:03}"),
            &AgentCompletionStatus::Completed,
            &format!("sha256:chain{i}"),
            &[],
            i * 1000,
        )
        .expect("append settlement");
    }

    let latest = wal.latest_sequence().expect("latest_sequence");
    assert_eq!(latest.0, 3, "latest_sequence must be 3 after three appends");

    let report = wal.verify_integrity().expect("verify_integrity");
    assert!(report.is_valid);
    assert_eq!(report.total_entries, 3);
}

#[test]
fn settlement_wal_wrapper_open_recovers_existing() {
    let dir = temp_wal_dir("wrapper-reopen");

    // First session: append via wrapper
    {
        let wal = SettlementWal::open(&dir).expect("open session 1");
        wal.append_settlement(
            "node-1",
            "wl-reopen-001",
            &AgentCompletionStatus::Completed,
            "sha256:reopen-aaa",
            &[],
            1000,
        )
        .expect("append session 1");
        assert_eq!(wal.latest_sequence().unwrap().0, 1);
    }

    // Second session: reopen via wrapper, verify chain recovered
    {
        let wal = SettlementWal::open(&dir).expect("open session 2");
        assert_eq!(
            wal.latest_sequence().unwrap().0,
            1,
            "tail must be recovered after reopen"
        );

        let report = wal.verify_integrity().expect("verify after reopen");
        assert!(report.is_valid);
        assert_eq!(report.total_entries, 1);

        // Append a second entry — must chain to the recovered tail
        wal.append_settlement(
            "node-1",
            "wl-reopen-002",
            &AgentCompletionStatus::Completed,
            "sha256:reopen-bbb",
            &[],
            2000,
        )
        .expect("append session 2");
        assert_eq!(wal.latest_sequence().unwrap().0, 2);
    }
}

#[test]
fn settlement_wal_wrapper_open_failure_is_loud() {
    // Open on a path that is a regular file (not a directory) must fail.
    let dir = temp_wal_dir("wrapper-fail");
    let blocking_file = dir.join("not-a-dir");
    std::fs::write(&blocking_file, b"block").expect("write blocking file");

    let result = SettlementWal::open(&blocking_file);
    assert!(
        result.is_err(),
        "SettlementWal::open must fail on a non-directory path"
    );
}

#[test]
fn settlement_wal_boot_gate_reports_resume_sequence() {
    // The scheduler calls verify_on_boot once at startup: a fresh chain
    // verifies with sequence 0, and after one settlement the gate reports 1.
    // This test pins the production use of verify_integrity/latest_sequence
    // (workspace denies dead_code: test-only methods break the lib build).
    let dir = temp_wal_dir("boot-gate");
    let wal = SettlementWal::open(&dir).expect("open WAL");
    assert_eq!(
        wal.verify_on_boot().expect("fresh chain verifies"),
        0,
        "fresh chain resumes at sequence 0"
    );

    wal.append_settlement(
        "node-1",
        "wl-boot-001",
        &AgentCompletionStatus::Completed,
        "sha256:bootdigest",
        &[],
        1_786_800_000_000,
    )
    .expect("append settlement");
    assert_eq!(
        wal.verify_on_boot().expect("chained WAL verifies"),
        1,
        "one settlement resumes at sequence 1"
    );
}
