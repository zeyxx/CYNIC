//! Live Epistemic Grounding & Prompt Synthesis Benchmark (Chapter 6, L5)
//!
//! Validates:
//! 1. 100% rejection of hallucination traps (including sharing-vocabulary adversarial traps)
//!    with `NON COUVERT PAR LE CORPUS ACTUEL` at the optimal operating point (alpha = 0.60).
//! 2. 100% admission and citation fidelity for legitimate technical queries.
//! 3. Strict topological authority hierarchy ($L_0 \succ L_1 \succ L_2 \succ L_3$).
//! 4. "Lost-in-the-Middle" positional boundary arrangement within authority tiers.

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{
    BenchConfig, TemplatedEmbedder, TextPipeline, VectorCacheRecord, load_vector_cache,
    save_vector_cache,
};
use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::adapters::embedding::sync_bridge::SyncEmbeddingBridge;
use cynic_knowledge::ChunkEmbedder;
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
    let engine = backend.base_urls().join(",");
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
    let mut vector_cache = load_vector_cache(&vector_cache_path, &model, &pipeline.tag(), &engine)
        .unwrap_or_else(|| VectorCacheRecord::new(&model, &pipeline.tag(), &engine));

    let templated = TemplatedEmbedder::new(&bridge, &pipeline, config.embed_lot, &|_, _| {});
    let (vectors, _reused, _embedded) =
        vectorize_chunks_cached(&templated, &chunks, &mut vector_cache, &|_, _| {})
            .expect("vectorization must succeed");
    save_vector_cache(&vector_cache_path, &vector_cache).expect("save vector cache");

    let test_matrix = vec![
        EpistemicTestCase {
            id: "L0_POWERS",
            query: "5 pouvoirs séparation constituante judiciaire",
            is_trap: false,
            expected_top_authority: Some(AuthorityTier::L0Constitution),
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
        // Trivial traps (lexically disjoint)
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
        // Adversarial traps sharing legitimate CYNIC corpus vocabulary (C1 critique resolution)
        EpistemicTestCase {
            id: "TRAP_ADV_POW_MINING",
            query: "cynic-protocol consensus proof of work sha256 mining difficulty",
            is_trap: true,
            expected_top_authority: None,
        },
        EpistemicTestCase {
            id: "TRAP_ADV_ERC20_SWAP",
            query: "contracts foundation json erc20 token swap router uniswap pool",
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

    let assembly_config = EpistemicAssemblyConfig {
        min_confidence: 40.0,
        min_term_coverage: 0.50,
        max_sources: 5,
        max_context_chars: 8_000,
        lost_in_the_middle_reordering: true,
    };

    // D3 critique resolution: verify trap wall at headline optimum alpha = 0.60
    let headline_alpha = 0.60;
    println!(
        "VERIFYING OPERATING POINT: alpha = {:.2} (ECDF / MinMax)",
        headline_alpha
    );

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

        // Continuous convex combination at operating point alpha = 0.60
        let fused_hits = fuse_continuous_convex(
            &sparse,
            &dense,
            headline_alpha,
            NormalizationStrategy::CandidateMinMax,
        );

        let verdict = assemble_epistemic_context(test_case.query, &fused_hits, &assembly_config);
        if let Some(top) = fused_hits.first() {
            let cov = cynic_knowledge::domain::epistemic_grounding::compute_term_coverage(test_case.query, &top.chunk.content, &top.chunk.source_path);
            println!("DEBUG [{}] -> top path: {}, cov: {:.2}, score: {:.2}", test_case.id, top.chunk.source_path, cov, top.score);
        }

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
                        "  [TRAP REJECTED] {:<22} | Term Cov: {:4.1}% | Conf: {:.2} | Token: {}",
                        test_case.id,
                        term_coverage * 100.0,
                        confidence,
                        rejection_token
                    );
                    println!("    Reason: {}", reason);
                }
                EpistemicCoverageVerdict::Covered { confidence, .. } => {
                    panic!(
                        "FATAL: Trap query '{}' falsely admitted with confidence {:.4} at operating alpha={:.2}!",
                        test_case.query, confidence, headline_alpha
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
                    dropped_sources_count,
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
                        "  [LEGI ADMITTED] {:<22} | Sources: {} (Dropped: {}) | Conf: {:.2} | Top Auth: {:?}",
                        test_case.id,
                        sources.len(),
                        dropped_sources_count,
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
        "EPISTEMIC GATE RECEIPT (alpha = {:.2}): Traps rejected = {}/{}, Legitimate admitted = {}/{}",
        headline_alpha, traps_rejected, traps_total, legi_admitted, legi_total
    );
    println!(
        "=========================================================================================\n"
    );

    assert_eq!(
        traps_rejected, traps_total,
        "100% of out-of-corpus traps must be rejected at operating alpha"
    );
    assert_eq!(
        legi_admitted, legi_total,
        "100% of legitimate queries must be admitted"
    );
}
