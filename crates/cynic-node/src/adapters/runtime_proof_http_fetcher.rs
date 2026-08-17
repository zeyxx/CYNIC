//! 2N+2b transport adapter — HTTP GET peer `RuntimeProofBundle`.
//!
//! Implements `RuntimeProofFetcherPort` (AGENS.md §7 adapters disposable).
//! Swappable to Phase 3.2 websocket by replacing this impl — the port-bound
//! federation policy never changes.

use cynic_protocol::RuntimeProofBundle;
use reqwest::StatusCode;

use crate::ports::runtime_proof_fetcher_port::RuntimeProofFetcherPort;

#[derive(Debug)]
pub struct HttpRuntimeProofFetcher {
    client: reqwest::Client,
}

impl HttpRuntimeProofFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client builds"),
        }
    }
}

impl RuntimeProofFetcherPort for HttpRuntimeProofFetcher {
    fn fetch_peer_proof_bundle(
        &self,
        peer_url: &str,
    ) -> Result<RuntimeProofBundle, Box<dyn std::error::Error>> {
        let url = format!("{}/network/runtime/proof", peer_url.trim_end_matches('/'));
        let fut = async {
            let resp = self.client.get(&url).send().await?;
            if resp.status() != StatusCode::OK {
                return Err(format!("peer {} returned {} for runtime_proof", peer_url, resp.status()).into());
            }
            let bundle: RuntimeProofBundle = resp.json().await?;
            Ok::<RuntimeProofBundle, Box<dyn std::error::Error>>(bundle)
        };
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(fut)
            }
        }
    }
}
