use cynic_knowledge::{AuthorityTier, DenseHit, DocumentChunk, SearchHit, fuse_reciprocal_ranks};

#[test]
fn test_reciprocal_rank_fusion_combines_sparse_and_dense_signals() {
    let doc_l0 = DocumentChunk::new(
        "docs/foundation/CONSTITUTION.md".into(),
        1,
        15,
        vec!["Constitution".into()],
        "Separation of powers and sovereign autonomy".into(),
        AuthorityTier::L0Constitution,
    );

    let doc_l1_exact = DocumentChunk::new(
        "docs/specs/network.md".into(),
        1,
        20,
        vec!["Network".into()],
        "Mesh wireguard protocol topology".into(),
        AuthorityTier::L1Specification,
    );

    let doc_l2_semantic = DocumentChunk::new(
        "docs/design/mesh_routing.md".into(),
        1,
        30,
        vec!["Design".into()],
        "Decentralized peer node communication pathways".into(),
        AuthorityTier::L2Design,
    );

    // Sparse BM25 ranking:
    // #1: doc_l1_exact (exact keyword match)
    // #2: doc_l0 (slight keyword overlap)
    let sparse_hits = vec![
        SearchHit {
            chunk: &doc_l1_exact,
            score: 15.0,
        },
        SearchHit {
            chunk: &doc_l0,
            score: 5.0,
        },
    ];

    // Dense Vector ranking (semantic synonym overlap without exact words):
    // #1: doc_l2_semantic (semantic embedding match)
    // #2: doc_l1_exact (also semantically related)
    let dense_hits = vec![
        DenseHit {
            chunk: &doc_l2_semantic,
            similarity: 0.94,
        },
        DenseHit {
            chunk: &doc_l1_exact,
            similarity: 0.88,
        },
    ];

    let fused = fuse_reciprocal_ranks(&sparse_hits, &dense_hits, 60.0);

    // Three unique documents recovered
    assert_eq!(fused.len(), 3);

    // doc_l1_exact benefited from dual-presence (in both sparse #1 and dense #2)
    // and its L1Specification altitude (2.0)
    assert_eq!(fused[0].chunk.source_path, "docs/specs/network.md");

    // doc_l2_semantic was successfully pulled into results via pure dense similarity!
    assert!(
        fused
            .iter()
            .any(|h| h.chunk.source_path == "docs/design/mesh_routing.md"),
        "Dense-only hit must be present in fused results"
    );
}
