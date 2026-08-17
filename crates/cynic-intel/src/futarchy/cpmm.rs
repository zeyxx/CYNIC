use super::types::{
    ConditionalMarket, FUTARCHY_MARKET_SCHEMA_V1, MarketDecision, MarketResolution, PredictionBet,
    ProposalStatus,
};

pub fn create_market(
    market_id: &str,
    proposal_id: &str,
    question: &str,
    initial_liquidity_units: u64,
    min_conviction_threshold_bps: u32,
) -> Result<ConditionalMarket, String> {
    if initial_liquidity_units < 2 || initial_liquidity_units % 2 != 0 {
        return Err("initial liquidity must be a positive, even number of units".to_string());
    }
    if min_conviction_threshold_bps > 10_000 {
        return Err("conviction threshold must not exceed 10_000 bps".to_string());
    }

    let half_pool = initial_liquidity_units / 2;
    Ok(ConditionalMarket {
        schema: FUTARCHY_MARKET_SCHEMA_V1,
        market_id: market_id.to_string(),
        proposal_id: proposal_id.to_string(),
        question: question.to_string(),
        yes_pool_units: half_pool,
        no_pool_units: half_pool,
        total_liquidity_units: initial_liquidity_units,
        min_conviction_threshold_bps,
        status: ProposalStatus::Open,
        resolution: None,
    })
}

pub fn calculate_prices_bps(yes_pool: u64, no_pool: u64) -> (u32, u32) {
    let total = yes_pool.saturating_add(no_pool);
    if total == 0 {
        return (5_000, 5_000);
    }
    let yes_price = ((no_pool as u128 * 10_000) / total as u128) as u32;
    (yes_price, 10_000 - yes_price)
}

pub fn execute_bet(market: &mut ConditionalMarket, bet: PredictionBet) -> Result<u64, String> {
    if market.status != ProposalStatus::Open {
        return Err("cannot bet on a closed market".to_string());
    }
    if bet.amount_units == 0 {
        return Err("bet amount must be positive".to_string());
    }

    let (in_pool, out_pool) = if bet.is_yes {
        (&mut market.no_pool_units, &mut market.yes_pool_units)
    } else {
        (&mut market.yes_pool_units, &mut market.no_pool_units)
    };
    let dx = bet.amount_units as u128;
    let x = *in_pool as u128;
    let y = *out_pool as u128;
    let shares_bought = ((y * dx) / (x + dx)) as u64;
    if shares_bought == 0 {
        return Err("bet is too small to buy a share".to_string());
    }

    *in_pool = in_pool
        .checked_add(bet.amount_units)
        .ok_or_else(|| "market pool overflow".to_string())?;
    *out_pool = out_pool
        .checked_sub(shares_bought)
        .ok_or_else(|| "market share accounting underflow".to_string())?;
    market.total_liquidity_units = market
        .yes_pool_units
        .checked_add(market.no_pool_units)
        .ok_or_else(|| "market liquidity overflow".to_string())?;
    Ok(shares_bought)
}

pub fn resolve_market(
    market: &mut ConditionalMarket,
    timestamp: i64,
) -> Result<MarketResolution, String> {
    if market.status != ProposalStatus::Open {
        return Err("only an open market may be resolved".to_string());
    }
    let (yes_price, no_price) = calculate_prices_bps(market.yes_pool_units, market.no_pool_units);
    let decision = if yes_price >= market.min_conviction_threshold_bps {
        MarketDecision::Pass
    } else {
        MarketDecision::Fail
    };
    let resolution = MarketResolution {
        decision,
        yes_price_bps: yes_price,
        no_price_bps: no_price,
        payout_ratio_bps: if decision == MarketDecision::Pass {
            yes_price
        } else {
            no_price
        },
        resolved_at: timestamp,
    };
    market.status = ProposalStatus::Resolved;
    market.resolution = Some(resolution.clone());
    Ok(resolution)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yes_bet_changes_price_and_preserves_reported_pool_liquidity() {
        let mut market =
            create_market("market-1", "proposal-1", "deploy?", 1_000_000, 6_000).unwrap();
        let shares = execute_bet(
            &mut market,
            PredictionBet {
                bettor_wallet: "wallet-a".to_string(),
                is_yes: true,
                amount_units: 200_000,
                timestamp: 1,
            },
        )
        .unwrap();

        assert!(shares > 0);
        assert!(calculate_prices_bps(market.yes_pool_units, market.no_pool_units).0 > 5_000);
        assert_eq!(
            market.total_liquidity_units,
            market.yes_pool_units + market.no_pool_units
        );
    }

    #[test]
    fn market_requires_valid_initial_state_and_one_resolution() {
        assert!(create_market("market-1", "proposal-1", "deploy?", 1, 6_000).is_err());
        assert!(create_market("market-1", "proposal-1", "deploy?", 100, 10_001).is_err());

        let mut market = create_market("market-1", "proposal-1", "deploy?", 100, 6_000).unwrap();
        resolve_market(&mut market, 1).unwrap();
        assert!(resolve_market(&mut market, 2).is_err());
    }
}
