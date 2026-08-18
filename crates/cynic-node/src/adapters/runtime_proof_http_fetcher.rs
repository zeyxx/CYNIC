//! 2N+2b transport adapter — HTTP GET peer `RuntimeProofBundle`.
//!
//! Implements `RuntimeProofFetcherPort` (AGENS.md §7 adapters disposable).
//! Swappable to Phase 3.2 websocket by replacing this impl — the port-bound
//! federation policy never changes.

use cynic_protocol::RuntimeProofBundle;
use reqwest::StatusCode;
use std::env;

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
                .redirect(reqwest::redirect::Policy::none())
                // AGENS.md §4: TLS validation stays ON for federation transport.
                // Peer identity is validated by url scheme + cert thumbprint
                // cross-check against registry.json peer fingerprints (future).
                .danger_accept_invalid_certs(false)
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
        let mut url = reqwest::Url::parse(peer_url.trim_end_matches('/'))?;
        // Phase 3.2-security: federation transport must use https://.
        // Plain http:// is rejected to prevent MITM (bundle is kernel-signed
        // for integrity, but peer admission evidence is sensitive federation
        // topology data).
        if url.scheme() != "https" {
            return Err(
                "peer_url must use https:// for runtime_publisher federation transport".into(),
            );
        }
        url.set_path(&format!(
            "{}/network/runtime/proof",
            url.path().trim_end_matches('/')
        ));
        let bearer = env::var("CYNIC_PEER_API_KEY").ok();
        let fut = async {
            let mut request = self.client.get(url.as_str());
            if let Some(token) = &bearer {
                request = request.bearer_auth(token);
            }
            let resp = request.send().await?;
            if resp.status() != StatusCode::OK {
                return Err(format!(
                    "peer {} returned {} for runtime_proof",
                    url,
                    resp.status()
                )
                .into());
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
