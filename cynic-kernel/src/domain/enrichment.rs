//! TokenEnricher — port trait for on-chain token data enrichment.
//! When domain=token-analysis and content looks like a Solana address,
//! the pipeline enriches the stimulus before Dogs evaluate.
//! Pure domain — zero external dependencies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Enriched token data from on-chain sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// Mint address
    pub mint: String,
    /// Token name (e.g. "Jupiter")
    pub name: Option<String>,
    /// Token symbol (e.g. "JUP")
    pub symbol: Option<String>,
    /// Total supply (raw, before decimals)
    pub supply: Option<u64>,
    /// Decimal places
    pub decimals: Option<u8>,
    /// Price per token in USD
    pub price_usd: Option<f64>,
    /// Number of unique token holders
    pub holder_count: Option<u64>,
    /// Top 10 holder concentration (HHI-like)
    pub top10_concentration: Option<f64>,
    /// Whether the mint authority is revoked (good sign)
    pub mint_authority_revoked: bool,
    /// Whether the freeze authority is revoked (good sign)
    pub freeze_authority_revoked: bool,
    /// Token standard (Fungible, NonFungible, etc.)
    pub token_standard: Option<String>,
    /// Off-chain description from token metadata
    pub description: Option<String>,
    /// Creation date if available
    pub created_at: Option<String>,
}

impl TokenData {
    /// Format enriched data as a structured stimulus for Dogs.
    /// Dogs receive this instead of a raw base58 address.
    pub fn to_stimulus(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("Token: {}", self.mint));

        if let Some(ref name) = self.name {
            parts.push(format!("Name: {name}"));
        }
        if let Some(ref sym) = self.symbol {
            parts.push(format!("Symbol: {sym}"));
        }
        if let Some(price) = self.price_usd {
            parts.push(format!("Price: ${price:.6}"));
        }
        if let (Some(supply), Some(decimals)) = (self.supply, self.decimals) {
            let human_supply = supply as f64 / 10f64.powi(decimals as i32);
            parts.push(format!("Supply: {human_supply:.0}"));
        }
        if let Some(holders) = self.holder_count {
            parts.push(format!("Holders: {holders}"));
        }
        if let Some(conc) = self.top10_concentration {
            parts.push(format!("Top-10 concentration: {conc:.1}%"));
        }

        parts.push(format!(
            "Mint authority: {}",
            if self.mint_authority_revoked {
                "REVOKED"
            } else {
                "ACTIVE"
            }
        ));
        parts.push(format!(
            "Freeze authority: {}",
            if self.freeze_authority_revoked {
                "REVOKED"
            } else {
                "ACTIVE"
            }
        ));

        if let Some(ref std) = self.token_standard {
            parts.push(format!("Standard: {std}"));
        }
        if let Some(ref desc) = self.description {
            let truncated = if desc.len() > 200 {
                format!("{}...", &desc[..200])
            } else {
                desc.clone()
            };
            parts.push(format!("Description: {truncated}"));
        }

        parts.join("\n")
    }
}

/// Port trait for token enrichment.
/// Adapters implement this to fetch on-chain data from any source.
#[async_trait]
pub trait TokenEnricherPort: Send + Sync {
    /// Enrich a Solana mint address with on-chain data.
    /// Returns None if the address is not a valid token.
    async fn enrich(&self, mint_address: &str) -> Result<Option<TokenData>, EnrichmentError>;
}

/// Enrichment errors — separate from Dog/Judge errors.
#[derive(Debug, thiserror::Error)]
pub enum EnrichmentError {
    #[error("enrichment request failed: {0}")]
    RequestFailed(String),
    #[error("enrichment timed out")]
    Timeout,
    #[error("enrichment unavailable")]
    Unavailable,
}

/// Check if a string looks like a Solana base58 address (32-44 chars, base58 alphabet).
pub fn looks_like_solana_address(s: &str) -> bool {
    let len = s.len();
    (32..=44).contains(&len)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_solana_address() {
        assert!(looks_like_solana_address(
            "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
        ));
        assert!(looks_like_solana_address(
            "So11111111111111111111111111111111111111112"
        ));
        assert!(!looks_like_solana_address("hello world"));
        assert!(!looks_like_solana_address("too_short"));
        // Contains 'O' which is not in base58
        assert!(!looks_like_solana_address(
            "OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO"
        ));
    }

    #[test]
    fn test_token_data_to_stimulus() {
        let data = TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            name: Some("Jupiter".into()),
            symbol: Some("JUP".into()),
            supply: Some(6_863_982_190_903_847),
            decimals: Some(6),
            price_usd: Some(0.178),
            holder_count: Some(250_000),
            top10_concentration: Some(45.2),
            mint_authority_revoked: true,
            freeze_authority_revoked: true,
            token_standard: Some("Fungible".into()),
            description: Some("JUP is the governance token for Jupiter.".into()),
            created_at: None,
        };
        let stim = data.to_stimulus();
        assert!(stim.contains("Jupiter"));
        assert!(stim.contains("REVOKED"));
        assert!(stim.contains("250000"));
    }
}
