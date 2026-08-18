use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Burn, Token2022};

pub mod state;
use state::*;

declare_id!("CYNiC11111111111111111111111111111111111111");

#[program]
pub mod cynic_anchor {
    use super::*;

    /// 1. Ancrage d'une racine Merkle d'E-Score et du digest Nightshift dans un compte Zero-Copy
    pub fn record_poh_checkpoint(
        ctx: Context<RecordPohCheckpoint>,
        epoch: u64,
        merkle_root: [u8; 32],
        round_digest: [u8; 32],
    ) -> Result<()> {
        let mut record = ctx.accounts.poh_record.load_init()?;
        record.epoch = epoch;
        record.timestamp = Clock::get()?.unix_timestamp;
        record.merkle_root = merkle_root;
        record.round_digest = round_digest;
        record.node_authority = ctx.accounts.node_authority.key();
        record.bump = ctx.bumps.poh_record;
        Ok(())
    }

    /// 2. Création d'un marché de futarchie conditionnel avec invariant CPMM k = x * y
    pub fn create_futarchy_market(
        ctx: Context<CreateFutarchyMarket>,
        proposal_hash: [u8; 32],
        initial_liquidity_units: u64,
        min_conviction_threshold_bps: u32,
    ) -> Result<()> {
        let mut market = ctx.accounts.market.load_init()?;
        let half = initial_liquidity_units / 2;
        market.proposal_hash = proposal_hash;
        market.yes_pool_units = half;
        market.no_pool_units = half;
        market.min_conviction_threshold_bps = min_conviction_threshold_bps;
        market.status = 0; // Open
        market.decision = 0; // Undecided
        market.bump = ctx.bumps.market;
        Ok(())
    }

    /// 3. Pari conditionnel avec formule à produit constant dy = (y * dx) / (x + dx)
    pub fn place_futarchy_bet(
        ctx: Context<PlaceFutarchyBet>,
        is_yes: bool,
        amount_units: u64,
    ) -> Result<()> {
        let mut market = ctx.accounts.market.load_mut()?;
        require!(market.status == 0, CynicAnchorError::MarketClosed);
        require!(amount_units > 0, CynicAnchorError::InvalidAmount);

        let (in_pool, out_pool) = if is_yes {
            (&mut market.no_pool_units, &mut market.yes_pool_units)
        } else {
            (&mut market.yes_pool_units, &mut market.no_pool_units)
        };

        let dx = amount_units as u128;
        let x = *in_pool as u128;
        let y = *out_pool as u128;

        let dy = (y * dx) / (x + dx);
        let shares_bought = dy as u64;

        *in_pool += amount_units;
        *out_pool = out_pool.saturating_sub(shares_bought);

        Ok(())
    }

    /// 4. Résolution du marché selon le prix en points de base (BPS)
    pub fn resolve_futarchy_market(ctx: Context<ResolveFutarchyMarket>) -> Result<()> {
        let mut market = ctx.accounts.market.load_mut()?;
        require!(market.status == 0, CynicAnchorError::MarketClosed);

        let total = market.yes_pool_units + market.no_pool_units;
        let yes_price_bps = if total == 0 {
            5000
        } else {
            ((market.no_pool_units as u128 * 10_000) / total as u128) as u32
        };

        market.status = 1; // Resolved
        if yes_price_bps >= market.min_conviction_threshold_bps {
            market.decision = 1; // Pass
        } else {
            market.decision = 2; // Fail
        }

        Ok(())
    }

    /// 5. Destruction déflationniste IGNIS avec Token-2022 Burn CPI
    pub fn execute_ignis_burn(ctx: Context<ExecuteIgnisBurn>, amount: u64) -> Result<()> {
        let mut vault = ctx.accounts.ignis_vault.load_mut()?;

        let cpi_accounts = Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.token_source.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token_2022::burn(cpi_ctx, amount)?;

        vault.total_burned_units += amount;
        vault.last_burn_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct RecordPohCheckpoint<'info> {
    #[account(
        init,
        payer = node_authority,
        space = 8 + std::mem::size_of::<PohRecordAccount>(),
        seeds = [b"cynic_poh", node_authority.key().as_ref(), &epoch.to_le_bytes()],
        bump
    )]
    pub poh_record: AccountLoader<'info, PohRecordAccount>,
    #[account(mut)]
    pub node_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_hash: [u8; 32])]
pub struct CreateFutarchyMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + std::mem::size_of::<FutarchyMarketAccount>(),
        seeds = [b"futarchy_market", &proposal_hash],
        bump
    )]
    pub market: AccountLoader<'info, FutarchyMarketAccount>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceFutarchyBet<'info> {
    #[account(mut)]
    pub market: AccountLoader<'info, FutarchyMarketAccount>,
    pub bettor: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResolveFutarchyMarket<'info> {
    #[account(mut)]
    pub market: AccountLoader<'info, FutarchyMarketAccount>,
    pub oracle_or_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteIgnisBurn<'info> {
    #[account(mut)]
    pub ignis_vault: AccountLoader<'info, IgnisVaultAccount>,
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub token_source: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[error_code]
pub enum CynicAnchorError {
    #[msg("The conditional market is closed or already resolved.")]
    MarketClosed,
    #[msg("Bet amount must be strictly greater than zero.")]
    InvalidAmount,
}
