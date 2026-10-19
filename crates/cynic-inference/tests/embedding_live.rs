//! Live embedding smoke test — ignored by default, needs a real endpoint.
//!
//! Run on the builder (same LAN as cynic-lxc-embedding):
//! `CYNIC_EMBED_URL=http://10.10.10.106:8080/v1 \
//!  CYNIC_EMBED_MODEL=embeddinggemma-2-bf16 \
//!  cargo test -p cynic-inference --test embedding_live -- --ignored --nocapture`

use cynic_inference::adapters::embedding::http::EmbeddingBackend;
use cynic_inference::ports::embedding_port::EmbeddingPort;

#[tokio::test]
#[ignore = "needs a live /v1/embeddings endpoint (CYNIC_EMBED_URL)"]
async fn live_embedding_roundtrip_reports_dimensions() {
    let backend =
        EmbeddingBackend::from_env().expect("CYNIC_EMBED_URL must point at a live endpoint");
    let start = std::time::Instant::now();
    let embedding = backend
        .embed("CYNIC memory probe")
        .await
        .expect("live embedding must succeed");
    let latency_ms = start.elapsed().as_millis();
    println!(
        "LIVE_EMBEDDING dims={} latency_ms={} tokens={:?} url={}",
        embedding.dimensions,
        latency_ms,
        embedding.prompt_tokens,
        backend.base_url()
    );
    assert!(embedding.dimensions > 0);
    assert_eq!(embedding.vector.len(), embedding.dimensions);
}
