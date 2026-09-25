use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::types::ConditionalMarket;

pub const LIQUIDITY_VAULT_SCHEMA_V1: &str = "cynic.liquidity_vault.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiquidityVault {
    pub schema: &'static str,
    pub vault_id: String,
    pub total_reserve_units: u64,
    pub allocated_liquidity_units: u64,
    pub active_markets_count: usize,
    pub markets: BTreeMap<String, ConditionalMarket>,
}

impl LiquidityVault {
    pub fn new(vault_id: &str, initial_reserve_units: u64) -> Self {
        Self {
            schema: LIQUIDITY_VAULT_SCHEMA_V1,
            vault_id: vault_id.to_string(),
            total_reserve_units: initial_reserve_units,
            allocated_liquidity_units: 0,
            active_markets_count: 0,
            markets: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, amount_units: u64) {
        self.total_reserve_units += amount_units;
    }

    pub fn allocate_market_liquidity(&mut self, market: ConditionalMarket) -> Result<(), String> {
        if self.allocated_liquidity_units + market.total_liquidity_units > self.total_reserve_units
        {
            return Err("Insufficient vault reserves to seed market".to_string());
        }

        self.allocated_liquidity_units += market.total_liquidity_units;
        self.active_markets_count += 1;
        self.markets.insert(market.market_id.clone(), market);

        Ok(())
    }

    pub fn available_reserve_units(&self) -> u64 {
        self.total_reserve_units
            .saturating_sub(self.allocated_liquidity_units)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futarchy::cpmm::create_market;

    #[test]
    fn test_vault_allocation_and_reserves() {
        let mut vault = LiquidityVault::new("vault-odysseum-alpha", 10_000_000);
        assert_eq!(vault.available_reserve_units(), 10_000_000);

        let market = create_market("mkt-001", "prop-1", "Should we deploy?", 2_000_000, 6000);

        vault.allocate_market_liquidity(market).expect("allocation");
        assert_eq!(vault.allocated_liquidity_units, 2_000_000);
        assert_eq!(vault.available_reserve_units(), 8_000_000);
    }
}
