//! Filesystem-backed binary WAL adapter implementing `WalJournalPort`.
//! Supports crash-recovery with torn-tail truncation, SHA3-512 cryptographic verification,
//! and fdatasync durability boundaries.

pub mod crc;
pub mod frame;

use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::StorageError;
use crate::ports::WalJournalPort;
use crate::types::{
    Hash512, IntegrityReport, PoHTick, SequenceNumber, WalAppendInput, WalEntry, WalReceipt,
};
use frame::{
    FrameDecodeError, compute_commit_hash, compute_entry_hash, decode_frame, encode_frame,
};

/// Internal mutable state for the active WAL segment.
struct WalFsState {
    dir: PathBuf,
    active_file: File,
    active_segment_id: u64,
    latest_sequence: SequenceNumber,
    latest_entry_hash: Hash512,
    latest_poh_tick: PoHTick,
    entries: Vec<WalEntry>,
}

/// Filesystem adapter implementing `WalJournalPort`.
#[derive(Clone)]
pub struct WalFsAdapter {
    state: Arc<RwLock<WalFsState>>,
}

impl WalFsAdapter {
    /// Opens or initializes a WAL directory.
    /// Executes deterministic crash-recovery: torn tails at EOF are truncated,
    /// while mid-log corruption fails closed immediately.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, StorageError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;

        let active_path = dir.join("active.seg");
        let mut entries = Vec::new();
        let mut latest_seq = SequenceNumber::GENESIS;
        let mut latest_hash = Hash512::GENESIS;
        let mut latest_poh = PoHTick(0);
        let mut valid_file_len = 0u64;

        if active_path.exists() {
            let mut file = File::open(&active_path).map_err(|e| StorageError::Io(e.to_string()))?;
            let file_len = file
                .metadata()
                .map_err(|e| StorageError::Io(e.to_string()))?
                .len();
            let mut reader = BufReader::new(&mut file);
            let mut expected_prev_hash = Hash512::GENESIS;

            loop {
                let frame_start_offset = reader
                    .stream_position()
                    .map_err(|e| StorageError::Io(e.to_string()))?;
                match decode_frame(&mut reader) {
                    Ok(Some(entry)) => {
                        // Verify sequential integrity
                        if entry.previous_hash != expected_prev_hash {
                            return Err(StorageError::HashChainCorrupt {
                                sequence: entry.sequence,
                                expected_prev: expected_prev_hash,
                                actual_prev: entry.previous_hash,
                            });
                        }
                        expected_prev_hash = entry.entry_hash;
                        latest_seq = entry.sequence;
                        latest_hash = entry.entry_hash;
                        latest_poh = entry.poh_tick;
                        valid_file_len = reader
                            .stream_position()
                            .map_err(|e| StorageError::Io(e.to_string()))?;
                        entries.push(entry);
                    }
                    Ok(None) => {
                        // Clean EOF
                        break;
                    }
                    Err(e) => {
                        let remaining = file_len.saturating_sub(frame_start_offset);
                        if remaining < frame::MIN_FRAME_LEN as u64 {
                            // Recoverable torn tail strictly at EOF (< MIN_FRAME_LEN bytes)
                            tracing::warn!(
                                "Recovering torn tail in active WAL segment at offset {frame_start_offset} ({remaining} bytes). Truncating to {valid_file_len} bytes."
                            );
                            drop(reader);
                            drop(file);
                            let trunc_file = OpenOptions::new()
                                .write(true)
                                .open(&active_path)
                                .map_err(|e| StorageError::Io(e.to_string()))?;
                            trunc_file
                                .set_len(valid_file_len)
                                .map_err(|e| StorageError::Io(e.to_string()))?;
                            trunc_file
                                .sync_data()
                                .map_err(|e| StorageError::Io(e.to_string()))?;
                            break;
                        } else {
                            // Corruption with full frame length remaining: fail closed!
                            return Err(StorageError::from(e));
                        }
                    }
                }
            }
        }

        let active_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)
            .map_err(|e| StorageError::Io(e.to_string()))?;

        let state = WalFsState {
            dir,
            active_file,
            active_segment_id: 1,
            latest_sequence: latest_seq,
            latest_entry_hash: latest_hash,
            latest_poh_tick: latest_poh,
            entries,
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
        })
    }
}

impl WalJournalPort for WalFsAdapter {
    fn append_batch(&self, events: &[WalAppendInput]) -> Result<Vec<WalReceipt>, StorageError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let mut state = self.state.write();
        let mut receipts = Vec::with_capacity(events.len());
        let mut encoded_batch = Vec::new();
        let mut new_entries = Vec::with_capacity(events.len());

        let mut current_seq = state.latest_sequence;
        let mut current_prev_hash = state.latest_entry_hash;
        let mut current_poh = state.latest_poh_tick;

        for input in events {
            current_seq = current_seq.next();
            current_poh = PoHTick(current_poh.0 + 1);

            let commit_hash = compute_commit_hash(
                current_seq,
                &current_prev_hash,
                current_poh,
                &input.schema,
                &input.payload,
                input.witness.mandate_id.as_deref(),
                input.witness.proof_anchor.as_ref(),
            );

            let entry_hash = compute_entry_hash(
                &commit_hash,
                &input.witness.principal_pubkey,
                &input.witness.signature,
            );

            let entry = WalEntry {
                sequence: current_seq,
                poh_tick: current_poh,
                previous_hash: current_prev_hash,
                commit_hash,
                entry_hash,
                schema: input.schema.clone(),
                payload: input.payload.clone(),
                witness: input.witness.clone(),
            };

            let encoded = encode_frame(&entry)?;
            encoded_batch.extend_from_slice(&encoded);

            receipts.push(WalReceipt {
                sequence: current_seq,
                entry_hash,
                poh_tick: current_poh,
            });

            current_prev_hash = entry_hash;
            new_entries.push(entry);
        }

        // Write batch atomically and issue fdatasync
        state
            .active_file
            .write_all(&encoded_batch)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        state
            .active_file
            .sync_data()
            .map_err(|e| StorageError::Io(e.to_string()))?;

        // Update state
        state.latest_sequence = current_seq;
        state.latest_entry_hash = current_prev_hash;
        state.latest_poh_tick = current_poh;
        state.entries.extend(new_entries);

        Ok(receipts)
    }

    fn scan_stream<'a>(
        &'a self,
        from_seq: SequenceNumber,
        to_seq: Option<SequenceNumber>,
    ) -> Result<Box<dyn Iterator<Item = Result<WalEntry, StorageError>> + 'a>, StorageError> {
        let state = self.state.read();
        let filtered: Vec<WalEntry> = state
            .entries
            .iter()
            .filter(|e| e.sequence >= from_seq && to_seq.map_or(true, |to| e.sequence <= to))
            .cloned()
            .collect();

        Ok(Box::new(filtered.into_iter().map(Ok)))
    }

    fn latest_sequence(&self) -> Result<SequenceNumber, StorageError> {
        Ok(self.state.read().latest_sequence)
    }

    fn verify_integrity(&self) -> Result<IntegrityReport, StorageError> {
        let state = self.state.read();
        let mut expected_prev = Hash512::GENESIS;

        for entry in &state.entries {
            if entry.previous_hash != expected_prev {
                return Ok(IntegrityReport {
                    verified_up_to_seq: entry.sequence,
                    root_hash: expected_prev,
                    total_entries: state.entries.len() as u64,
                    is_valid: false,
                });
            }

            let commit_hash = compute_commit_hash(
                entry.sequence,
                &entry.previous_hash,
                entry.poh_tick,
                &entry.schema,
                &entry.payload,
                entry.witness.mandate_id.as_deref(),
                entry.witness.proof_anchor.as_ref(),
            );

            let recomputed_entry_hash = compute_entry_hash(
                &commit_hash,
                &entry.witness.principal_pubkey,
                &entry.witness.signature,
            );

            if recomputed_entry_hash != entry.entry_hash {
                return Ok(IntegrityReport {
                    verified_up_to_seq: entry.sequence,
                    root_hash: entry.entry_hash,
                    total_entries: state.entries.len() as u64,
                    is_valid: false,
                });
            }

            expected_prev = entry.entry_hash;
        }

        Ok(IntegrityReport {
            verified_up_to_seq: state.latest_sequence,
            root_hash: state.latest_entry_hash,
            total_entries: state.entries.len() as u64,
            is_valid: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CausalWitness;
    use bytes::Bytes;

    fn test_witness() -> CausalWitness {
        CausalWitness {
            principal_pubkey: [11u8; 32],
            signature: [22u8; 64],
            mandate_id: Some("mandate-test".to_string()),
            proof_anchor: Some(Hash512([33u8; 64])),
        }
    }

    #[test]
    fn wal_fs_adapter_lifecycle_roundtrip() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let adapter = WalFsAdapter::open(temp_dir.path()).expect("open WAL");

        assert_eq!(adapter.latest_sequence().unwrap(), SequenceNumber::GENESIS);

        let input1 = WalAppendInput {
            schema: "cynic.test.event.v1".to_string(),
            payload: Bytes::from_static(b"first entry"),
            witness: test_witness(),
        };

        let input2 = WalAppendInput {
            schema: "cynic.test.event.v1".to_string(),
            payload: Bytes::from_static(b"second entry"),
            witness: test_witness(),
        };

        let receipts = adapter
            .append_batch(&[input1, input2])
            .expect("append batch");
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].sequence, SequenceNumber(1));
        assert_eq!(receipts[1].sequence, SequenceNumber(2));

        let report = adapter.verify_integrity().expect("integrity check");
        assert!(report.is_valid);
        assert_eq!(report.total_entries, 2);

        // Reopen from disk and assert all entries are recovered identically
        drop(adapter);
        let reopened = WalFsAdapter::open(temp_dir.path()).expect("reopen WAL");
        assert_eq!(reopened.latest_sequence().unwrap(), SequenceNumber(2));

        let report_reopened = reopened
            .verify_integrity()
            .expect("integrity check reopened");
        assert!(report_reopened.is_valid);
        assert_eq!(report_reopened.total_entries, 2);
    }

    #[test]
    fn wal_fs_adapter_recovers_from_torn_tail_at_eof() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let adapter = WalFsAdapter::open(temp_dir.path()).expect("open WAL");

        let input = WalAppendInput {
            schema: "cynic.test.event.v1".to_string(),
            payload: Bytes::from_static(b"valid entry"),
            witness: test_witness(),
        };

        adapter.append_batch(&[input]).expect("append");
        assert_eq!(adapter.latest_sequence().unwrap(), SequenceNumber(1));
        drop(adapter);

        // Intentionally append partial junk bytes at EOF to simulate an ungraceful crash
        let active_path = temp_dir.path().join("active.seg");
        {
            let mut file = OpenOptions::new()
                .append(true)
                .open(&active_path)
                .expect("open to corrupt");
            file.write_all(b"partial-corrupted-frame-bytes")
                .expect("write junk");
            file.sync_data().expect("sync");
        }

        // Reopening should detect the torn tail, truncate it cleanly, and succeed
        let recovered = WalFsAdapter::open(temp_dir.path()).expect("reopen after crash");
        assert_eq!(recovered.latest_sequence().unwrap(), SequenceNumber(1));

        let report = recovered
            .verify_integrity()
            .expect("integrity after recovery");
        assert!(report.is_valid);
        assert_eq!(report.total_entries, 1);
    }
}
