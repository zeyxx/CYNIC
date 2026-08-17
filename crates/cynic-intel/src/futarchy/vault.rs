use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::types::{ConditionalMarket, ProposalStatus};

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

    pub fn deposit(&mut self, amount_units: u64) -> Result<(), String> {
        self.total_reserve_units = self
            .total_reserve_units
            .checked_add(amount_units)
            .ok_or_else(|| "vault reserve overflow".to_string())?;
        Ok(())
    }

    pub fn allocate_market_liquidity(&mut self, market: ConditionalMarket) -> Result<(), String> {
        if market.status != ProposalStatus::Open {
            return Err("only an open market may receive an allocation".to_string());
        }
        if self.markets.contains_key(&market.market_id) {
            return Err("market is already allocated in this vault".to_string());
        }
        let next_allocation = self
            .allocated_liquidity_units
            .checked_add(market.total_liquidity_units)
            .ok_or_else(|| "vault allocation overflow".to_string())?;
        if next_allocation > self.total_reserve_units {
            return Err("insufficient vault reserves to seed market".to_string());
        }

        self.allocated_liquidity_units = next_allocation;
        self.active_markets_count = self
            .active_markets_count
            .checked_add(1)
            .ok_or_else(|| "active market count overflow".to_string())?;
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
    use crate::futarchy::create_market;

    #[test]
    fn vault_rejects_duplicate_market_without_double_allocation() {
        let mut vault = LiquidityVault::new("vault-1", 10_000);
        let market = create_market("market-1", "proposal-1", "deploy?", 2_000, 6_000).unwrap();
        vault.allocate_market_liquidity(market.clone()).unwrap();

        assert!(vault.allocate_market_liquidity(market).is_err());
        assert_eq!(vault.allocated_liquidity_units, 2_000);
        assert_eq!(vault.active_markets_count, 1);
    }
}
