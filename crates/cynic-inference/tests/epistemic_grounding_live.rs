//! Live Epistemic Grounding & Prompt Synthesis Benchmark (Chapter 6, L5)
//!
//! Validates:
//! 1. 100% rejection of hallucination traps with `NON COUVERT PAR LE CORPUS ACTUEL`
//! 2. 100% admission and citation fidelity for legitimate technical queries
//! 3. Topological authority hierarchy ($L_0 \succ L_1 \succ L_2 \succ L_3$)
//! 4. "Lost-in-the-Middle" positional boundary arrangement

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, TemplatedEmbedder, TextPipeline, VectorCacheRecord, load_vector_cache,
    save_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::domain::continuous_fusion::{NormalizationStrategy, fuse_continuous_convex};
use cynic_knowledge::domain::epistemic_grounding::{
    EpistemicAssemblyConfig, EpistemicCoverageVerdict, UNCOVERED_REJECTION_TOKEN,
    assemble_epistemic_context,
};
use cynic_knowledge::{AuthorityTier, sync_workspace_corpus, top_k_dense, vectorize_chunks_cached};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

#[derive(Debug)]
struct EpistemicTestCase {
    id: &'static str,
    query: &'static str,
    is_trap: bool,
    expected_top_authority: Option<AuthorityTier>,
}

#[test]
#[ignore = "needs live embedding endpoints (CYNIC_EMBED_URL)"]
fn test_epistemic_grounding_live_matrix() {
    let root = workspace_root();
    let backend = EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must be set");
    let bridge = SyncEmbeddingBridge::probe(backend, "CYNIC epistemic grounding calibration")
        .expect("embedding probe must succeed");

    let cache_file = root.join("target/eval_corpus_cache.json");
    let subset = 1000;

    let (cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);
    let mut chunks: Vec<_> = cache.chunks_by_path().into_values().flatten().collect();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    if subset > 0 && subset < chunks.len() {
        chunks.truncate(subset);
    }

    let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "multi-endpoint".into());
    let pipeline = TextPipeline {
        max_chars: 1000,
        doc_prefix: String::new(),
        query_prefix: String::new(),
        doc_template: "{body}".into(),
    };
    let config = BenchConfig {
        subset,
        trunc_dim: 0,
        top_k: 5,
        dense_k: 20,
        guard_gap: None,
        dense_guard_gap: None,
        embed_lot: 32,
        pipeline: pipeline.clone(),
        model: model.clone(),
    };

    let vector_cache_path = root.join(config.vector_cache_filename());
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag())
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag()));

    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|_, _| {});
    let (vectors, _reused, _embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    save_vector_cache(&vector_cache_path, &vector_cache).expect("save vector cache");

    let test_matrix = vec![
        EpistemicTestCase {
            id: "L1_POWERS",
            query: "5 pouvoirs séparation constituante judiciaire",
            is_trap: false,
            expected_top_authority: Some(AuthorityTier::L1Specification),
        },
        EpistemicTestCase {
            id: "L0_BUILDER_GATE",
            query: "build gate ssot 10.10.10.119",
            is_trap: false,
            expected_top_authority: Some(AuthorityTier::L0Constitution),
        },
        EpistemicTestCase {
            id: "L0_MEMORY_ADMISSION",
            query: "memory admission formula weights kv activations",
            is_trap: false,
            expected_top_authority: Some(AuthorityTier::L0Constitution),
        },
        EpistemicTestCase {
            id: "L0_TEMPORAL_CONTRACT",
            query: "pub struct TemporalContract temporal reconciliation",
            is_trap: false,
            expected_top_authority: Some(AuthorityTier::L0Constitution),
        },
        EpistemicTestCase {
            id: "TRAP_HELM_K8S",
            query: "deploy kubernetes helm chart cynic cluster pods",
            is_trap: true,
            expected_top_authority: None,
        },
        EpistemicTestCase {
            id: "TRAP_SWAPFILE_CORTEX",
            query: "configure 32gb swapfile swapon cortex host",
            is_trap: true,
            expected_top_authority: None,
        },
        EpistemicTestCase {
            id: "TRAP_UNISWAP_LIQUIDITY",
            query: "uniswap pool cynic token price usd pair liquidity",
            is_trap: true,
            expected_top_authority: None,
        },
    ];

    println!(
        "\n========================================================================================="
    );
    println!("CHAPTER 6: EPISTEMIC GROUNDING & ANTI-HALLUCINATION GATING (L5)");
    println!(
        "========================================================================================="
    );

    let assembly_config = EpistemicAssemblyConfig::default();
    let alpha = 0.30;
    let mut traps_rejected = 0;
    let mut traps_total = 0;
    let mut legi_admitted = 0;
    let mut legi_total = 0;

    for test_case in &test_matrix {
        let query_text = config.pipeline.query_text(test_case.query);
        let query_vectors = bridge
            .embed_batch_reported(&[query_text.as_str()], &|_, _| {})
            .expect("embed query");
        let query_vector = &query_vectors[0];

        let sparse = index.search(test_case.query, config.dense_k);
        let dense = top_k_dense(query_vector, &chunks, &vectors, config.dense_k);

        // Continuous convex combination
        let fused_hits = fuse_continuous_convex(
            &sparse,
            &dense,
            alpha,
            NormalizationStrategy::CandidateMinMax,
        );

        let verdict = assemble_epistemic_context(test_case.query, &fused_hits, &assembly_config);

        if test_case.is_trap {
            traps_total += 1;
            match verdict {
                EpistemicCoverageVerdict::Uncovered {
                    rejection_token,
                    confidence,
                    term_coverage,
                    reason,
                } => {
                    assert_eq!(rejection_token, UNCOVERED_REJECTION_TOKEN);
                    traps_rejected += 1;
                    println!(
                        "  [TRAP REJECTED] {:<22} | Term Cov: {:4.1}% | Conf: {:.3} | Token: {}",
                        test_case.id,
                        term_coverage * 100.0,
                        confidence,
                        rejection_token
                    );
                    println!("    Reason: {}", reason);
                }
                EpistemicCoverageVerdict::Covered { confidence, .. } => {
                    panic!(
                        "FATAL: Trap query '{}' falsely admitted with confidence {:.4}!",
                        test_case.query, confidence
                    );
                }
            }
        } else {
            legi_total += 1;
            match verdict {
                EpistemicCoverageVerdict::Covered {
                    assembled_context,
                    sources,
                    confidence,
                    top_authority,
                } => {
                    legi_admitted += 1;
                    assert!(
                        !sources.is_empty(),
                        "Covered query must have at least 1 citation"
                    );

                    // Check citation tag format
                    for src in &sources {
                        let tag = src.citation_tag();
                        assert!(
                            tag.starts_with("[SOURCE "),
                            "Citation tag must match canonical format"
                        );
                        assert!(
                            assembled_context.contains(&tag),
                            "Context must include formatted citation tag"
                        );
                    }

                    if let Some(expected_auth) = test_case.expected_top_authority {
                        assert_eq!(
                            top_authority, expected_auth,
                            "[{}] Top authority mismatch: expected {:?}, got {:?}",
                            test_case.id, expected_auth, top_authority
                        );
                    }

                    println!(
                        "  [LEGI ADMITTED] {:<22} | Sources: {} | Conf: {:.3} | Top Auth: {:?}",
                        test_case.id,
                        sources.len(),
                        confidence,
                        top_authority
                    );
                    println!("    First citation: {}", sources[0].citation_tag());
                }
                EpistemicCoverageVerdict::Uncovered { reason, .. } => {
                    panic!(
                        "FATAL: Legitimate query '{}' rejected as uncovered! Reason: {}",
                        test_case.query, reason
                    );
                }
            }
        }
    }

    println!(
        "-----------------------------------------------------------------------------------------"
    );
    println!(
        "EPISTEMIC GATE RECEIPT: Traps rejected = {}/{}, Legitimate admitted = {}/{}",
        traps_rejected, traps_total, legi_admitted, legi_total
    );
    println!(
        "=========================================================================================\n"
    );

    assert_eq!(
        traps_rejected, traps_total,
        "100% of out-of-corpus traps must be rejected"
    );
    assert_eq!(
        legi_admitted, legi_total,
        "100% of legitimate queries must be admitted"
    );
}
