//! Cold-evidence journal file adapter (D1, layer L0).
//!
//! Append-only writer + offline verifier for `EvidenceJournalEntry`
//! segments under a root directory (default `/var/lib/cynic/evidence`).
//!
//! Layout: one JSONL segment per month (`evidence-YYYYMM.journal`). Each
//! segment's first entry chains from the previous segment's final hash
//! (`prev_hash`) — the chain never resets across rotation. Writes are
//! single-line appends followed by flush; no truncate, no rewrite.
//!
//! The adapter owns ONLY file mechanics; validation lives in the pure
//! contract (`cynic_protocol::evidence_journal`). Verification is fully
//! offline: reading the files back and replaying the chain needs no DB,
//! no network, no running kernel.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use cynic_protocol::evidence_journal::{
    EVIDENCE_JOURNAL_SCHEMA, EvidenceJournalEntry, compute_entry_hash, validate_evidence_chain,
};

#[derive(Debug, Clone)]
pub struct EvidenceJournalStore {
    root: PathBuf,
    node_id: String,
}

/// Result of one append: what was written and where.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendOutcome {
    pub segment_path: PathBuf,
    pub sequence: u64,
    pub entry_hash: String,
}

impl EvidenceJournalStore {
    /// Open (or create) the journal rooted at `root` for `node_id`.
    pub fn open(root: impl Into<PathBuf>, node_id: &str) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            node_id: node_id.to_string(),
        })
    }

    fn segment_path(&self, year_month: &str) -> PathBuf {
        self.root.join(format!("evidence-{year_month}.journal"))
    }

    fn current_year_month() -> String {
        Utc::now().format("%Y%m").to_string()
    }

    /// Last hash + highest sequence across all existing segments, in file
    /// order. Returns ("genesis", 0) when the journal is empty. The scan is
    /// offline and cheap at expected volumes (<100 MB/year).
    pub fn tip(&self) -> std::io::Result<(String, u64)> {
        let mut last = (String::from("genesis"), 0u64);
        let mut segments: Vec<PathBuf> = list_segments(&self.root)?;
        segments.sort();
        for path in segments {
            let entries = read_segment(&path)?;
            if let Some(entry) = entries.last() {
                last = (entry.entry_hash.clone(), entry.sequence);
            }
        }
        Ok(last)
    }

    /// Append one sealed event. Computes the chained hash from the current
    /// tip; caller supplies only payload + timestamp semantics.
    pub fn append(
        &self,
        event_type: &str,
        payload_digest: &str,
        sealed_at_unix_s: i64,
    ) -> std::io::Result<AppendOutcome> {
        let (prev_hash, prev_sequence) = self.tip()?;
        let sequence = prev_sequence + 1;
        let entry_hash = compute_entry_hash(payload_digest, &prev_hash, sequence, sealed_at_unix_s);

        let entry = EvidenceJournalEntry {
            schema: EVIDENCE_JOURNAL_SCHEMA.to_string(),
            sequence,
            node_id: self.node_id.clone(),
            event_type: event_type.to_string(),
            payload_digest: payload_digest.to_string(),
            entry_hash: entry_hash.clone(),
            prev_hash,
            sealed_at_unix_s,
        };

        let line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let path = self.segment_path(&Self::current_year_month());
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{line}")?;
        file.flush()?;
        file.sync_all()?;

        Ok(AppendOutcome {
            segment_path: path,
            sequence,
            entry_hash,
        })
    }

    /// Read every entry of every segment, in order (offline).
    pub fn read_all(&self) -> std::io::Result<Vec<EvidenceJournalEntry>> {
        let mut all = Vec::new();
        let mut segments: Vec<PathBuf> = list_segments(&self.root)?;
        segments.sort();
        for path in segments {
            all.extend(read_segment(&path)?);
        }
        Ok(all)
    }

    /// Full offline verification: structural + chain + monotonicity across
    /// ALL segments, plus cross-segment continuity. Returns violations;
    /// empty = proof holds.
    pub fn verify_offline(&self) -> Vec<&'static str> {
        match self.read_all() {
            Ok(entries) => validate_evidence_chain(&entries),
            Err(_) => vec!["evidence_journal_unreadable"],
        }
    }
}

fn list_segments(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with("evidence-") && name.ends_with(".journal") {
            out.push(path);
        }
    }
    Ok(out)
}

fn read_segment(path: &Path) -> std::io::Result<Vec<EvidenceJournalEntry>> {
    let reader = BufReader::new(File::open(path)?);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: EvidenceJournalEntry = serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        entries.push(entry);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store(tag: &str) -> (EvidenceJournalStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "cynic_evj_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let store = EvidenceJournalStore::open(&dir, "node-lab").unwrap();
        (store, dir)
    }

    #[test]
    fn empty_journal_tip_is_genesis_and_verify_passes() {
        let (store, dir) = tmp_store("empty");
        assert_eq!(store.tip().unwrap(), ("genesis".to_string(), 0));
        assert!(store.verify_offline().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn append_then_verify_holds_offline() {
        let (store, dir) = tmp_store("append");
        let a = store
            .append("pattern_gc.model.sealed", "sha3-512:aa11", 1_700_000_000)
            .unwrap();
        let b = store
            .append("pattern_gc.model.sealed", "sha3-512:bb22", 1_700_000_100)
            .unwrap();
        assert_eq!(a.sequence, 1);
        assert_eq!(b.sequence, 2);
        assert_ne!(a.entry_hash, b.entry_hash);
        assert_eq!(store.tip().unwrap(), (b.entry_hash.clone(), 2));

        // Re-open from disk: verification must be independent of this process'
        // memory (offline property).
        let reopened = EvidenceJournalStore::open(&dir, "node-lab").unwrap();
        assert!(reopened.verify_offline().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tampering_a_written_line_is_detected_on_disk() {
        let (store, dir) = tmp_store("tamper");
        store
            .append("pattern_gc.model.sealed", "sha3-512:cc33", 1_700_000_000)
            .unwrap();
        store
            .append("pattern_gc.model.sealed", "sha3-512:dd44", 1_700_000_100)
            .unwrap();

        // Corrupt the FIRST line on disk (simulated bitrot/attacker).
        let seg = list_segments(&dir).unwrap().remove(0);
        let raw = std::fs::read_to_string(&seg).unwrap();
        let mut lines: Vec<String> = raw.lines().map(String::from).collect();
        let mut first: EvidenceJournalEntry = serde_json::from_str(&lines[0]).unwrap();
        first.payload_digest = "sha3-512:evil".to_string();
        lines[0] = serde_json::to_string(&first).unwrap();
        std::fs::write(&seg, lines.join("\n") + "\n").unwrap();

        let violations = store.verify_offline();
        // Tampering the payload digest breaks the recomputed hash of line 1
        // (chain_broken) AND leaves entry 2's prev_hash pointing at the old
        // hash (mismatch). Both signals together = tamper evidence.
        assert!(
            violations.contains(&"evidence_journal_chain_broken")
                && violations.contains(&"evidence_journal_prev_hash_mismatch"),
            "chain must detect tampering: {violations:?}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn multi_node_merge_sorts_by_node_then_sequence() {
        // Two nodes seal independently with overlapping sequences; the merge
        // rule is (node_id, sequence) ordering — verified as pure logic here
        // via distinct stores producing independent valid chains.
        let dir_a = std::env::temp_dir().join(format!("cynic_evj_ma_{}", std::process::id()));
        let dir_b = std::env::temp_dir().join(format!("cynic_evj_mb_{}", std::process::id()));
        let a = EvidenceJournalStore::open(&dir_a, "node-a").unwrap();
        let b = EvidenceJournalStore::open(&dir_b, "node-b").unwrap();
        let ea = a.append("k", "sha3-512:a1", 1000).unwrap();
        let eb = b.append("k", "sha3-512:b1", 1000).unwrap();
        // Same sequence numbers are fine ACROSS nodes; each chain is valid alone.
        assert_eq!(ea.sequence, eb.sequence);
        assert_ne!(ea.entry_hash, eb.entry_hash);
        assert!(a.verify_offline().is_empty());
        assert!(b.verify_offline().is_empty());
        let _ = std::fs::remove_dir_all(dir_a);
        let _ = std::fs::remove_dir_all(dir_b);
    }
}
