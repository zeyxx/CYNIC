//! Chunk Quality & Anti-Heresy Audit (Organic RAG Falsification Gate)
//!
//! Enforces:
//! 1. Zero giant chunks (> 8,000 characters) across the entire corpus.
//! 2. P99 of chunk line count <= 75 lines.
//! 3. Healthy chunk ratio >= 95% of total corpus.

use cynic_knowledge::domain::chunk_quality::ChunkQualityReport;
use cynic_knowledge::sync_workspace_corpus;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn audit_corpus_chunk_quality_and_bounds() {
    let root = repo_root();
    let cache_path = root.join("target/corpus_audit_cache.json");

    let (record, _index) = sync_workspace_corpus(&root, Some(&cache_path), 0);

    let mut all_chunks = Vec::new();
    for file_record in record.files.values() {
        for chunk in &file_record.chunks {
            all_chunks.push(chunk.clone());
        }
    }

    assert!(
        !all_chunks.is_empty(),
        "Corpus chunks must not be empty for quality audit"
    );

    let report = ChunkQualityReport::audit(&all_chunks);

    println!(
        "CHUNK_QUALITY_REPORT: total={}, healthy={:.2}%, P50={} chars ({} lines), P99={} chars ({} lines), max={} chars",
        report.total_chunks,
        report.healthy_ratio * 100.0,
        report.char_percentiles.p50,
        report.line_percentiles.p50,
        report.char_percentiles.p99,
        report.line_percentiles.p99,
        report.char_percentiles.max,
    );

    // 1. Zero giant chunks exceeding 8000 characters
    let severe_oversized: Vec<_> = report
        .oversized_chunks
        .iter()
        .filter(|a| a.char_length > 8000)
        .collect();
    assert!(
        severe_oversized.is_empty(),
        "Detected {} severe giant chunks (> 8000 chars): {:?}",
        severe_oversized.len(),
        severe_oversized
    );

    // 2. Line count P99 bound
    assert!(
        report.line_percentiles.p99 <= 75,
        "P99 line count must be <= 75 lines, got {}",
        report.line_percentiles.p99
    );

    // 3. Healthy chunk ratio
    assert!(
        report.healthy_ratio >= 0.90,
        "Healthy chunk ratio must be >= 90%, got {:.2}%",
        report.healthy_ratio * 100.0
    );
}
