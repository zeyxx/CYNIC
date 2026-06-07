//! Lo-fi token enrichment fallback.
//!
//! This adapter is intentionally conservative: it uses free public sources
//! to keep token-analysis sighted when Helius is not configured, but it marks
//! uncertain fields as unknown instead of synthesizing confidence.

use crate::backends::dexscreener::DexScreenerClient;
use crate::domain::enrichment::{EnrichmentError, TokenData, TokenEnricherPort};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const RUGCHECK_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct LofiEnricher {
    client: Client,
    dex: DexScreenerClient,
}

impl Default for LofiEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl LofiEnricher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(RUGCHECK_TIMEOUT)
                .build()
                .unwrap_or_default(),
            dex: DexScreenerClient::new(),
        }
    }

    async fn get_rugcheck_report(&self, mint: &str) -> Option<RugCheckReport> {
        let url = format!("https://api.rugcheck.xyz/v1/tokens/{mint}/report");
        let resp = self.client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            tracing::debug!(
                mint = %mint,
                status = resp.status().as_u16(),
                "RugCheck non-200"
            );
            return None;
        }
        resp.json().await.ok()
    }
}

#[async_trait]
impl TokenEnricherPort for LofiEnricher {
    async fn enrich(&self, mint_address: &str) -> Result<Option<TokenData>, EnrichmentError> {
        let dex_data = self.dex.get_market_data(mint_address).await;
        let rug_data = self.get_rugcheck_report(mint_address).await;

        if dex_data.is_none() && rug_data.is_none() {
            return Ok(None);
        }

        let mut data = TokenData {
            mint: mint_address.to_string(),
            lp_status: "unknown".into(),
            origin: Some("lofi-fallback".into()),
            ..Default::default()
        };

        if let Some(rug) = rug_data {
            data.name = rug.token_meta.name;
            data.symbol = rug.token_meta.symbol;
            data.supply = Some(rug.token_info.supply);
            data.decimals = Some(rug.token_info.decimals);
            data.mint_authority_active = rug.token_info.mint_authority.is_some();
            data.freeze_authority_active = rug.token_info.freeze_authority.is_some();

            data.holder_count = rug.top_holders.len() as u64;
            data.holder_count_is_exact = false;
            data.holder_data_available = !rug.top_holders.is_empty();

            if let Some(top1) = rug.top_holders.first() {
                data.top1_pct = top1.pct;
                data.top1_type = if top1.insider {
                    "insider".into()
                } else {
                    "wallet".into()
                };
            }

            data.top10_pct = rug.top_holders.iter().take(10).map(|h| h.pct).sum();

            if rug
                .risks
                .iter()
                .any(|risk| risk.name.to_ascii_lowercase().contains("lp") && risk.level == "danger")
            {
                data.lp_status = "unsecured".into();
            }
        }

        if let Some(dex) = dex_data {
            data.price_usd = dex.price_usd;
            data.volume_24h_usd = dex.volume_24h_usd;
            data.liquidity_usd = dex.liquidity_usd;
        }

        Ok(Some(data))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RugCheckReport {
    #[serde(rename = "token")]
    token_info: RugTokenInfo,
    #[serde(default)]
    token_meta: RugTokenMeta,
    #[serde(default)]
    top_holders: Vec<RugHolder>,
    #[serde(default)]
    risks: Vec<RugRisk>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RugTokenInfo {
    supply: u64,
    decimals: u8,
    mint_authority: Option<String>,
    freeze_authority: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RugTokenMeta {
    name: Option<String>,
    symbol: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RugHolder {
    pct: f64,
    #[serde(default)]
    insider: bool,
}

#[derive(Debug, Deserialize)]
struct RugRisk {
    name: String,
    level: String,
}
