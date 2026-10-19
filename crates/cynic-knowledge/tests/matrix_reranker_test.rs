use cynic_knowledge::{AuthorityTier, DocumentChunk, LexicalIndex};

#[test]
fn test_matrix_reranker_constitutional_supremacy() {
    // 1. L3 code utility containing identical search words
    let chunk_l3 = DocumentChunk::new(
        "crates/cynic-cli/src/main.rs".into(),
        1,
        15,
        vec!["CLI".into(), "Main".into()],
        "fn execute_runtime() { check(); sovereignty(); admission(); }".into(),
        AuthorityTier::L3Operation,
    );

    // 2. L0 Constitution stating the sovereign invariant
    let chunk_l0 = DocumentChunk::new(
        "docs/foundation/CONSTITUTION.md".into(),
        1,
        25,
        vec!["Constitution".into(), "Power Division".into()],
        "Sovereign invariant: check sovereignty before admission of any workload.".into(),
        AuthorityTier::L0Constitution,
    );

    // 3. L1 Specification with superseded marker
    let chunk_superseded = DocumentChunk::new(
        "docs/specs/old_runtime.md".into(),
        1,
        20,
        vec!["Specification".into(), "Runtime".into()],
        "check sovereignty admission legacy mode [SUPERSEDED]".into(),
        AuthorityTier::L1Specification,
    );

    let index = LexicalIndex::build(vec![chunk_l3, chunk_l0, chunk_superseded]);
    let hits = index.search("check sovereignty admission", 3);

    assert_eq!(hits.len(), 3);

    // L0 Constitution must be #1 due to altitude 3.0
    assert_eq!(hits[0].chunk.source_path, "docs/foundation/CONSTITUTION.md");
    assert_eq!(hits[0].chunk.authority, AuthorityTier::L0Constitution);

    // Superseded L1 document must be heavily penalized below active L3 code
    assert_eq!(hits[2].chunk.source_path, "docs/specs/old_runtime.md");
    assert!(hits[2].chunk.content.contains("[SUPERSEDED]"));
    assert!(hits[0].score > hits[2].score * 4.0);
}
