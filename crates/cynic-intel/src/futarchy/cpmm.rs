use super::types::{
<<<<<<< HEAD
    ConditionalMarket, FUTARCHY_MARKET_SCHEMA_V1, MarketDecision, MarketResolution, PredictionBet,
    ProposalStatus,
=======
    ConditionalMarket, FUTARCHY_MARKET_SCHEMA_V1, MarketDecision, MarketResolution,
    PredictionBet, ProposalStatus,
>>>>>>> github/main
};

pub fn create_market(
    market_id: &str,
    proposal_id: &str,
    question: &str,
    initial_liquidity_units: u64,
    min_conviction_threshold_bps: u32,
) -> ConditionalMarket {
    let half_pool = initial_liquidity_units / 2;
    ConditionalMarket {
<<<<<<< HEAD
        schema: FUTARCHY_MARKET_SCHEMA_V1.to_string(),
=======
        schema: FUTARCHY_MARKET_SCHEMA_V1,
>>>>>>> github/main
        market_id: market_id.to_string(),
        proposal_id: proposal_id.to_string(),
        question: question.to_string(),
        yes_pool_units: half_pool,
        no_pool_units: half_pool,
        total_liquidity_units: initial_liquidity_units,
        min_conviction_threshold_bps,
        status: ProposalStatus::Open,
        resolution: None,
    }
}

pub fn calculate_prices_bps(yes_pool: u64, no_pool: u64) -> (u32, u32) {
    let total = yes_pool + no_pool;
    if total == 0 {
        return (5000, 5000);
    }
    // Price of YES is proportional to NO pool weight in CPMM
    let yes_price = ((no_pool as u128 * 10_000) / total as u128) as u32;
    let no_price = 10_000 - yes_price;
    (yes_price, no_price)
}

pub fn execute_bet(market: &mut ConditionalMarket, bet: PredictionBet) -> Result<u64, String> {
    if market.status != ProposalStatus::Open {
        return Err("Cannot bet on a closed market".to_string());
    }

    if bet.amount_units == 0 {
        return Err("Bet amount must be positive".to_string());
    }

    let (in_pool, out_pool) = if bet.is_yes {
        (&mut market.no_pool_units, &mut market.yes_pool_units)
    } else {
        (&mut market.yes_pool_units, &mut market.no_pool_units)
    };

    // Constant product formula: dy = (y * dx) / (x + dx)
    let dx = bet.amount_units as u128;
    let x = *in_pool as u128;
    let y = *out_pool as u128;

    let dy = (y * dx) / (x + dx);
    let shares_bought = dy as u64;

    *in_pool += bet.amount_units;
    *out_pool = out_pool.saturating_sub(shares_bought);
    market.total_liquidity_units += bet.amount_units;

    Ok(shares_bought)
}

pub fn resolve_market(market: &mut ConditionalMarket, timestamp: i64) -> MarketResolution {
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

    resolution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpmm_market_creation_and_pricing() {
        let mut market = create_market(
            "mkt-001",
            "prop-deploy-node-13",
            "Should we deploy Node 13 to ProxMox fleet?",
            1_000_000,
            6_000, // 60% threshold
        );

        let (p_yes, p_no) = calculate_prices_bps(market.yes_pool_units, market.no_pool_units);
        assert_eq!(p_yes, 5000);
        assert_eq!(p_no, 5000);

        // Bet YES with 200_000 units
        let bet_yes = PredictionBet {
            bettor_wallet: "wallet-alice".to_string(),
            is_yes: true,
            amount_units: 200_000,
            timestamp: 1_720_000_000,
        };

        let shares = execute_bet(&mut market, bet_yes).expect("bet execution");
        assert!(shares > 0);

<<<<<<< HEAD
        let (new_p_yes, new_p_no) =
            calculate_prices_bps(market.yes_pool_units, market.no_pool_units);
        assert!(
            new_p_yes > 5000,
            "Price of YES should increase after YES bet"
        );
=======
        let (new_p_yes, new_p_no) = calculate_prices_bps(market.yes_pool_units, market.no_pool_units);
        assert!(new_p_yes > 5000, "Price of YES should increase after YES bet");
>>>>>>> github/main
        assert_eq!(new_p_yes + new_p_no, 10_000);

        // Resolve market
        let resolution = resolve_market(&mut market, 1_720_000_100);
        assert_eq!(market.status, ProposalStatus::Resolved);
        assert!(resolution.yes_price_bps > 5000);
    }
}
