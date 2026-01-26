/**
 * CYNIC Anchor Program
 *
 * Proof of Judgment (PoJ) anchoring on Solana
 *
 * "Onchain is truth" - κυνικός
 *
 * This program anchors CYNIC judgment merkle roots to Solana,
 * creating an immutable record of the collective consciousness.
 */
use anchor_lang::prelude::*;

declare_id!("G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY");

/// φ⁻¹ - Maximum confidence (61.8%)
pub const PHI_INV: u64 = 618;
pub const PHI_SCALE: u64 = 1000;

/// Maximum validators in registry
pub const MAX_VALIDATORS: usize = 21; // F(8)

/// Historical note: MAX_ROOTS was intended for a ring buffer design
/// but individual PDA accounts per root is superior for PoJ:
/// - Roots never overwritten = verifiable forever
/// - O(1) lookup by merkle_root (PDA derivation)
/// - Each account pays own rent (~100 bytes)
/// - No complex ring buffer indexing
/// This constant is kept for reference but unused.
pub const _MAX_ROOTS_UNUSED: usize = 377; // F(14)

/// Maximum burn entries to track per account
pub const MAX_BURNS_PER_ACCOUNT: usize = 89; // F(11)

/// Burn reason codes
pub const BURN_REASON_INVALID_ROOT: u8 = 0;
pub const BURN_REASON_DOUBLE_SIGN: u8 = 1;
pub const BURN_REASON_TIMEOUT: u8 = 2;
pub const BURN_REASON_BYZANTINE: u8 = 3;
pub const BURN_REASON_SLASHING: u8 = 4;

/// E-Score constants (φ-aligned)
/// Maximum E-Score (100 * PHI_SCALE for precision)
pub const ESCORE_MAX: u64 = 100_000;
/// Minimum E-Score (can go negative, stored as signed offset)
pub const ESCORE_MIN: i64 = -61_800; // -φ⁻¹ * 100 * 1000
/// E-Score update cooldown in slots (F(10) = 55)
pub const ESCORE_UPDATE_COOLDOWN: u64 = 55;
/// Maximum E-Score snapshots to store per account (F(8) = 21)
pub const MAX_ESCORE_SNAPSHOTS: usize = 21;

/// E-Score contribution types
pub const ESCORE_TYPE_JUDGMENT: u8 = 0;
pub const ESCORE_TYPE_VALIDATION: u8 = 1;
pub const ESCORE_TYPE_LEARNING: u8 = 2;
pub const ESCORE_TYPE_PATTERN: u8 = 3;
pub const ESCORE_TYPE_FEEDBACK: u8 = 4;

/// Maximum merkle proof depth (supports up to 2^32 leaves)
pub const MAX_PROOF_DEPTH: usize = 32;

/// Staking constants (φ-aligned, in lamports)
/// Minimum stake to become validator: ~0.1 SOL (F(12) = 144 * 1M lamports)
pub const MIN_VALIDATOR_STAKE: u64 = 144_000_000;
/// Reward per valid anchor: ~0.001 SOL (F(6) = 8 * 100K lamports)
pub const REWARD_PER_ANCHOR: u64 = 800_000;
/// Slash percentage (φ⁻² = 38.2% of stake)
pub const SLASH_PERCENTAGE: u64 = 382;
/// Slash scale (for percentage calculation)
pub const SLASH_SCALE: u64 = 1000;
/// Unstake cooldown in slots (F(13) = 233)
pub const UNSTAKE_COOLDOWN: u64 = 233;

/// Memo prefix for identification
pub const MEMO_PREFIX: &[u8] = b"CYNIC:POJ:";

#[program]
pub mod cynic_anchor {
    use super::*;

    /// Initialize the CYNIC anchor state
    /// Only called once to set up the program
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.authority = ctx.accounts.authority.key();
        state.initialized_at = Clock::get()?.unix_timestamp;
        state.root_count = 0;
        state.validator_count = 0;
        state.bump = ctx.bumps.state;

        msg!("CYNIC Anchor initialized - κυνικός awakens on-chain");
        Ok(())
    }

    /// Add a validator to the registry
    /// Only authority can add validators
    pub fn add_validator(ctx: Context<ManageValidator>, validator: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;

        require!(
            state.validator_count < MAX_VALIDATORS as u8,
            CynicError::TooManyValidators
        );

        // Check not already a validator
        for i in 0..state.validator_count as usize {
            require!(
                state.validators[i] != validator,
                CynicError::AlreadyValidator
            );
        }

        let idx = state.validator_count as usize;
        state.validators[idx] = validator;
        state.validator_count += 1;

        msg!("Validator added: {}", validator);
        emit!(ValidatorAdded {
            validator,
            total_validators: state.validator_count,
        });

        Ok(())
    }

    /// Remove a validator from the registry
    /// Only authority can remove validators
    pub fn remove_validator(ctx: Context<ManageValidator>, validator: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;

        let mut found_index: Option<usize> = None;
        for i in 0..state.validator_count as usize {
            if state.validators[i] == validator {
                found_index = Some(i);
                break;
            }
        }

        let index = found_index.ok_or(CynicError::NotValidator)?;

        // Shift validators down
        let count = state.validator_count as usize;
        for i in index..(count - 1) {
            state.validators[i] = state.validators[i + 1];
        }
        state.validators[count - 1] = Pubkey::default();
        state.validator_count -= 1;

        msg!("Validator removed: {}", validator);
        emit!(ValidatorRemoved {
            validator,
            total_validators: state.validator_count,
        });

        Ok(())
    }

    /// Anchor a merkle root to Solana
    /// Only validators can anchor roots
    ///
    /// Architecture: Each root creates its own PDA account (not a ring buffer)
    /// This ensures roots are verifiable forever and lookup is O(1) by merkle_root.
    /// Individual accounts also pay their own rent, making the design economically sound.
    pub fn anchor_root(
        ctx: Context<AnchorRoot>,
        merkle_root: [u8; 32],
        item_count: u32,
        block_height: u64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let validator = ctx.accounts.validator.key();

        // Verify validator
        let mut is_validator = false;
        for i in 0..state.validator_count as usize {
            if state.validators[i] == validator {
                is_validator = true;
                break;
            }
        }
        require!(is_validator, CynicError::NotValidator);

        // Store root entry
        let root_entry = &mut ctx.accounts.root_entry;

        root_entry.merkle_root = merkle_root;
        root_entry.item_count = item_count;
        root_entry.block_height = block_height;
        root_entry.validator = validator;
        root_entry.timestamp = Clock::get()?.unix_timestamp;
        root_entry.slot = Clock::get()?.slot;
        root_entry.index = state.root_count;

        state.root_count += 1;
        state.last_anchor_slot = Clock::get()?.slot;

        msg!(
            "Root anchored #{}: {} ({} items)",
            root_entry.index,
            bs58::encode(&merkle_root).into_string(),
            item_count
        );

        emit!(RootAnchored {
            index: root_entry.index,
            merkle_root,
            item_count,
            block_height,
            validator,
            slot: root_entry.slot,
        });

        Ok(())
    }

    /// Verify a merkle root exists on-chain
    /// Returns root info if found
    pub fn verify_root(ctx: Context<VerifyRoot>, merkle_root: [u8; 32]) -> Result<()> {
        let root_entry = &ctx.accounts.root_entry;

        require!(
            root_entry.merkle_root == merkle_root,
            CynicError::RootNotFound
        );

        msg!(
            "Root verified: {} (anchored at slot {})",
            bs58::encode(&merkle_root).into_string(),
            root_entry.slot
        );

        emit!(RootVerified {
            merkle_root,
            index: root_entry.index,
            slot: root_entry.slot,
            validator: root_entry.validator,
        });

        Ok(())
    }

    /// Verify an item is included in an anchored merkle root
    /// Uses standard merkle proof verification (hash pairs from leaf to root)
    /// Enables trustless judgment verification on-chain
    pub fn verify_inclusion(
        ctx: Context<VerifyInclusion>,
        merkle_root: [u8; 32],
        item_hash: [u8; 32],
        proof: Vec<[u8; 32]>,
        proof_flags: Vec<bool>,
    ) -> Result<()> {
        let root_entry = &ctx.accounts.root_entry;

        // Verify root matches stored root
        require!(
            root_entry.merkle_root == merkle_root,
            CynicError::RootNotFound
        );

        // Validate proof length
        require!(
            proof.len() <= MAX_PROOF_DEPTH,
            CynicError::ProofTooLong
        );
        require!(
            proof.len() == proof_flags.len(),
            CynicError::ProofFlagsMismatch
        );

        // Compute merkle root from leaf and proof
        let mut computed_hash = item_hash;

        for (i, sibling) in proof.iter().enumerate() {
            // proof_flags[i] indicates if sibling is on the right (true) or left (false)
            let (left, right) = if proof_flags[i] {
                (computed_hash, *sibling)
            } else {
                (*sibling, computed_hash)
            };

            // Hash the pair: SHA256(left || right)
            computed_hash = anchor_lang::solana_program::hash::hashv(&[&left, &right]).to_bytes();
        }

        // Verify computed root matches anchored root
        require!(
            computed_hash == merkle_root,
            CynicError::InvalidMerkleProof
        );

        msg!(
            "Inclusion verified: item {} in root {}",
            bs58::encode(&item_hash).into_string(),
            bs58::encode(&merkle_root).into_string()
        );

        emit!(InclusionVerified {
            merkle_root,
            item_hash,
            proof_length: proof.len() as u8,
            root_index: root_entry.index,
            root_slot: root_entry.slot,
        });

        Ok(())
    }

    /// Transfer authority to new account
    pub fn transfer_authority(ctx: Context<TransferAuthority>, new_authority: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let old_authority = state.authority;
        state.authority = new_authority;

        msg!("Authority transferred: {} -> {}", old_authority, new_authority);
        emit!(AuthorityTransferred {
            old_authority,
            new_authority,
        });

        Ok(())
    }

    /// Record a burn (penalty/slashing)
    /// Only validators can record burns
    /// BURN axiom: "Don't extract, burn" - simplicity through penalties
    pub fn record_burn(
        ctx: Context<RecordBurn>,
        burned_account: Pubkey,
        amount: u64,
        reason: u8,
        merkle_root: [u8; 32],
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let reporter = ctx.accounts.reporter.key();

        // Verify reporter is a validator
        let mut is_validator = false;
        for i in 0..state.validator_count as usize {
            if state.validators[i] == reporter {
                is_validator = true;
                break;
            }
        }
        require!(is_validator, CynicError::NotValidator);

        // Validate reason code
        require!(reason <= BURN_REASON_SLASHING, CynicError::InvalidBurnReason);

        // Store burn entry
        let burn_entry = &mut ctx.accounts.burn_entry;
        let burn_tracker = &mut ctx.accounts.burn_tracker;

        burn_entry.amount = amount;
        burn_entry.burned_account = burned_account;
        burn_entry.reason = reason;
        burn_entry.merkle_root = merkle_root;
        burn_entry.timestamp = Clock::get()?.unix_timestamp;
        burn_entry.slot = Clock::get()?.slot;
        burn_entry.index = burn_tracker.burn_count;
        burn_entry.reporter = reporter;

        // Update tracker
        burn_tracker.burn_count += 1;
        burn_tracker.total_burned += amount;
        burn_tracker.last_burn_slot = burn_entry.slot;

        msg!(
            "Burn recorded #{}: {} burned {} (reason: {})",
            burn_entry.index,
            burned_account,
            amount,
            reason
        );

        emit!(BurnRecorded {
            index: burn_entry.index,
            burned_account,
            amount,
            reason,
            merkle_root,
            reporter,
            slot: burn_entry.slot,
        });

        Ok(())
    }

    /// Get burn statistics for an account
    pub fn get_burn_stats(ctx: Context<GetBurnStats>) -> Result<()> {
        let tracker = &ctx.accounts.burn_tracker;

        msg!(
            "Burn stats: {} burns, {} total burned",
            tracker.burn_count,
            tracker.total_burned
        );

        emit!(BurnStatsQueried {
            burn_count: tracker.burn_count,
            total_burned: tracker.total_burned,
            last_burn_slot: tracker.last_burn_slot,
        });

        Ok(())
    }

    /// Record an E-Score update for an account
    /// E-Score = Ecosystem contribution score (reputation)
    /// Only validators can update E-Scores
    pub fn update_escore(
        ctx: Context<UpdateEScore>,
        target_account: Pubkey,
        delta: i64,
        contribution_type: u8,
        judgment_id: [u8; 32],
    ) -> Result<()> {
        let state = &ctx.accounts.state;
        let reporter = ctx.accounts.reporter.key();

        // Verify reporter is a validator
        let mut is_validator = false;
        for i in 0..state.validator_count as usize {
            if state.validators[i] == reporter {
                is_validator = true;
                break;
            }
        }
        require!(is_validator, CynicError::NotValidator);

        // Validate contribution type
        require!(contribution_type <= ESCORE_TYPE_FEEDBACK, CynicError::InvalidEScoreType);

        // Get or create E-Score entry
        let escore_entry = &mut ctx.accounts.escore_entry;
        let current_slot = Clock::get()?.slot;

        // Check cooldown (prevent spam)
        if escore_entry.last_update_slot > 0 {
            require!(
                current_slot >= escore_entry.last_update_slot + ESCORE_UPDATE_COOLDOWN,
                CynicError::EScoreCooldown
            );
        }

        // Calculate new score with bounds
        let old_score = escore_entry.score;
        let new_score = old_score.saturating_add(delta);

        // Clamp to valid range
        let clamped_score = if new_score > ESCORE_MAX as i64 {
            ESCORE_MAX as i64
        } else if new_score < ESCORE_MIN {
            ESCORE_MIN
        } else {
            new_score
        };

        // Update entry
        escore_entry.account = target_account;
        escore_entry.score = clamped_score;
        escore_entry.update_count += 1;
        escore_entry.last_update_slot = current_slot;
        escore_entry.last_judgment_id = judgment_id;

        // Track contribution type counts
        match contribution_type {
            0 => escore_entry.judgment_count += 1,
            1 => escore_entry.validation_count += 1,
            2 => escore_entry.learning_count += 1,
            3 => escore_entry.pattern_count += 1,
            4 => escore_entry.feedback_count += 1,
            _ => {}
        }

        msg!(
            "E-Score updated for {}: {} -> {} (delta: {}, type: {})",
            target_account,
            old_score,
            clamped_score,
            delta,
            contribution_type
        );

        emit!(EScoreUpdated {
            account: target_account,
            old_score,
            new_score: clamped_score,
            delta,
            contribution_type,
            judgment_id,
            reporter,
            slot: current_slot,
        });

        Ok(())
    }

    /// Get E-Score for an account
    pub fn get_escore(ctx: Context<GetEScore>) -> Result<()> {
        let entry = &ctx.accounts.escore_entry;

        msg!(
            "E-Score for {}: {} (updates: {})",
            entry.account,
            entry.score,
            entry.update_count
        );

        emit!(EScoreQueried {
            account: entry.account,
            score: entry.score,
            update_count: entry.update_count,
            judgment_count: entry.judgment_count,
            validation_count: entry.validation_count,
            learning_count: entry.learning_count,
        });

        Ok(())
    }

    /// Record an E-Score snapshot (historical point-in-time record)
    pub fn snapshot_escore(ctx: Context<SnapshotEScore>, merkle_root: [u8; 32]) -> Result<()> {
        let escore_entry = &ctx.accounts.escore_entry;
        let snapshot = &mut ctx.accounts.escore_snapshot;
        let tracker = &mut ctx.accounts.snapshot_tracker;

        let current_slot = Clock::get()?.slot;
        let current_time = Clock::get()?.unix_timestamp;

        snapshot.account = escore_entry.account;
        snapshot.score = escore_entry.score;
        snapshot.merkle_root = merkle_root;
        snapshot.timestamp = current_time;
        snapshot.slot = current_slot;
        snapshot.index = tracker.snapshot_count;

        tracker.snapshot_count += 1;
        tracker.last_snapshot_slot = current_slot;

        msg!(
            "E-Score snapshot #{} for {}: score={}",
            snapshot.index,
            escore_entry.account,
            escore_entry.score
        );

        emit!(EScoreSnapshotted {
            index: snapshot.index,
            account: snapshot.account,
            score: snapshot.score,
            merkle_root,
            slot: current_slot,
        });

        Ok(())
    }

    /// Stake SOL to become a validator
    /// Minimum stake required: MIN_VALIDATOR_STAKE (~0.1 SOL)
    pub fn stake_validator(ctx: Context<StakeValidator>, amount: u64) -> Result<()> {
        require!(
            amount >= MIN_VALIDATOR_STAKE,
            CynicError::InsufficientStake
        );

        let stake = &mut ctx.accounts.validator_stake;
        let staker = ctx.accounts.staker.key();

        // Transfer SOL to stake account (PDA holds the stake)
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.staker.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, amount)?;

        // Initialize or update stake entry
        stake.validator = staker;
        stake.staked_amount = stake.staked_amount.checked_add(amount).unwrap();
        stake.staked_at = Clock::get()?.unix_timestamp;
        stake.last_anchor_slot = 0;
        stake.anchor_count = 0;
        stake.rewards_earned = 0;
        stake.rewards_claimed = 0;
        stake.is_active = true;

        msg!("Validator staked {} lamports", amount);

        emit!(ValidatorStaked {
            validator: staker,
            amount,
            total_staked: stake.staked_amount,
            slot: Clock::get()?.slot,
        });

        Ok(())
    }

    /// Request unstake (starts cooldown period)
    pub fn request_unstake(ctx: Context<RequestUnstake>) -> Result<()> {
        let stake = &mut ctx.accounts.validator_stake;

        require!(stake.is_active, CynicError::ValidatorNotActive);

        stake.unstake_requested_slot = Clock::get()?.slot;
        stake.is_active = false;

        msg!("Unstake requested, cooldown started");

        emit!(UnstakeRequested {
            validator: stake.validator,
            staked_amount: stake.staked_amount,
            cooldown_ends_slot: stake.unstake_requested_slot + UNSTAKE_COOLDOWN,
        });

        Ok(())
    }

    /// Complete unstake after cooldown period
    pub fn complete_unstake(ctx: Context<CompleteUnstake>) -> Result<()> {
        let stake = &ctx.accounts.validator_stake;
        let current_slot = Clock::get()?.slot;

        require!(!stake.is_active, CynicError::ValidatorStillActive);
        require!(
            stake.unstake_requested_slot > 0,
            CynicError::UnstakeNotRequested
        );
        require!(
            current_slot >= stake.unstake_requested_slot + UNSTAKE_COOLDOWN,
            CynicError::UnstakeCooldownNotComplete
        );

        let amount = stake.staked_amount;
        let validator = stake.validator;

        // Transfer SOL back from vault to validator
        let bump = ctx.bumps.stake_vault;
        let seeds = &[b"stake_vault".as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.stake_vault.to_account_info(),
                to: ctx.accounts.validator.to_account_info(),
            },
            signer,
        );
        anchor_lang::system_program::transfer(cpi_context, amount)?;

        msg!("Unstake completed, {} lamports returned", amount);

        emit!(UnstakeCompleted {
            validator,
            amount,
            slot: current_slot,
        });

        Ok(())
    }

    /// Claim accumulated rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let stake = &mut ctx.accounts.validator_stake;
        let claimable = stake.rewards_earned.saturating_sub(stake.rewards_claimed);

        require!(claimable > 0, CynicError::NoRewardsToClaim);

        // Transfer rewards from vault to validator
        let bump = ctx.bumps.reward_vault;
        let seeds = &[b"reward_vault".as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.reward_vault.to_account_info(),
                to: ctx.accounts.validator.to_account_info(),
            },
            signer,
        );
        anchor_lang::system_program::transfer(cpi_context, claimable)?;

        stake.rewards_claimed = stake.rewards_earned;

        msg!("Claimed {} lamports in rewards", claimable);

        emit!(RewardsClaimed {
            validator: stake.validator,
            amount: claimable,
            total_claimed: stake.rewards_claimed,
            slot: Clock::get()?.slot,
        });

        Ok(())
    }

    /// Slash a validator for misbehavior (BURN axiom)
    /// Only authority can slash validators
    pub fn slash_validator(
        ctx: Context<SlashValidator>,
        reason: u8,
        evidence_root: [u8; 32],
    ) -> Result<()> {
        let stake = &mut ctx.accounts.validator_stake;

        require!(stake.is_active, CynicError::ValidatorNotActive);
        require!(reason <= BURN_REASON_SLASHING, CynicError::InvalidBurnReason);

        // Calculate slash amount (φ⁻² = 38.2% of stake)
        let slash_amount = stake
            .staked_amount
            .checked_mul(SLASH_PERCENTAGE)
            .unwrap()
            .checked_div(SLASH_SCALE)
            .unwrap();

        let old_stake = stake.staked_amount;
        stake.staked_amount = stake.staked_amount.saturating_sub(slash_amount);
        stake.slash_count += 1;
        stake.total_slashed += slash_amount;

        // If stake falls below minimum, deactivate
        if stake.staked_amount < MIN_VALIDATOR_STAKE {
            stake.is_active = false;
        }

        msg!(
            "Validator slashed: {} lamports (reason: {})",
            slash_amount,
            reason
        );

        emit!(ValidatorSlashed {
            validator: stake.validator,
            slash_amount,
            reason,
            evidence_root,
            remaining_stake: stake.staked_amount,
            is_deactivated: !stake.is_active,
            slot: Clock::get()?.slot,
        });

        Ok(())
    }

    /// Award rewards to a validator for valid anchor (called internally after anchor_root)
    pub fn award_anchor_reward(ctx: Context<AwardAnchorReward>) -> Result<()> {
        let stake = &mut ctx.accounts.validator_stake;

        require!(stake.is_active, CynicError::ValidatorNotActive);

        stake.anchor_count += 1;
        stake.rewards_earned = stake.rewards_earned.checked_add(REWARD_PER_ANCHOR).unwrap();
        stake.last_anchor_slot = Clock::get()?.slot;

        msg!("Anchor reward awarded: {} lamports", REWARD_PER_ANCHOR);

        emit!(AnchorRewardAwarded {
            validator: stake.validator,
            reward_amount: REWARD_PER_ANCHOR,
            total_rewards: stake.rewards_earned,
            anchor_count: stake.anchor_count,
            slot: stake.last_anchor_slot,
        });

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Account Structures
// ═══════════════════════════════════════════════════════════════════════════

/// Global program state
#[account]
#[derive(Default)]
pub struct CynicState {
    /// Program authority (can manage validators)
    pub authority: Pubkey,
    /// Initialization timestamp
    pub initialized_at: i64,
    /// Total roots anchored
    pub root_count: u64,
    /// Number of validators
    pub validator_count: u8,
    /// Validator registry (max 21)
    pub validators: [Pubkey; MAX_VALIDATORS],
    /// Last anchor slot
    pub last_anchor_slot: u64,
    /// PDA bump
    pub bump: u8,
}

/// Individual anchored root entry
#[account]
#[derive(Default)]
pub struct RootEntry {
    /// Merkle root hash
    pub merkle_root: [u8; 32],
    /// Number of items in this root
    pub item_count: u32,
    /// PoJ block height
    pub block_height: u64,
    /// Validator who anchored
    pub validator: Pubkey,
    /// Unix timestamp
    pub timestamp: i64,
    /// Solana slot
    pub slot: u64,
    /// Sequential index
    pub index: u64,
}

/// Individual burn entry (BURN axiom: penalties/slashing)
#[account]
#[derive(Default)]
pub struct BurnEntry {
    /// Burned amount (in smallest unit, e.g., lamports or token decimals)
    pub amount: u64,
    /// Account that was burned/penalized
    pub burned_account: Pubkey,
    /// Reason code for the burn
    pub reason: u8,
    /// Associated merkle root (if burn relates to specific judgment)
    pub merkle_root: [u8; 32],
    /// Unix timestamp
    pub timestamp: i64,
    /// Solana slot
    pub slot: u64,
    /// Sequential burn index
    pub index: u64,
    /// Validator who reported the burn
    pub reporter: Pubkey,
}

/// Global burn statistics tracker
#[account]
#[derive(Default)]
pub struct BurnTracker {
    /// Total burns recorded
    pub burn_count: u64,
    /// Total amount burned across all burns
    pub total_burned: u64,
    /// Last burn slot
    pub last_burn_slot: u64,
    /// PDA bump
    pub bump: u8,
}

/// E-Score entry (Ecosystem contribution score / reputation)
#[account]
#[derive(Default)]
pub struct EScoreEntry {
    /// Account this E-Score belongs to
    pub account: Pubkey,
    /// Current score (can be negative, φ-bounded)
    pub score: i64,
    /// Total update count
    pub update_count: u64,
    /// Last update slot
    pub last_update_slot: u64,
    /// Last judgment ID that affected this score
    pub last_judgment_id: [u8; 32],
    /// Count by contribution type
    pub judgment_count: u32,
    pub validation_count: u32,
    pub learning_count: u32,
    pub pattern_count: u32,
    pub feedback_count: u32,
    /// PDA bump
    pub bump: u8,
}

/// E-Score snapshot (point-in-time historical record)
#[account]
#[derive(Default)]
pub struct EScoreSnapshot {
    /// Account this snapshot belongs to
    pub account: Pubkey,
    /// Score at time of snapshot
    pub score: i64,
    /// Associated merkle root (for verification)
    pub merkle_root: [u8; 32],
    /// Unix timestamp
    pub timestamp: i64,
    /// Solana slot
    pub slot: u64,
    /// Sequential snapshot index
    pub index: u64,
}

/// E-Score snapshot tracker
#[account]
#[derive(Default)]
pub struct EScoreSnapshotTracker {
    /// Total snapshots taken
    pub snapshot_count: u64,
    /// Last snapshot slot
    pub last_snapshot_slot: u64,
    /// PDA bump
    pub bump: u8,
}

/// Validator stake entry (staking for rewards/slashing)
#[account]
#[derive(Default)]
pub struct ValidatorStake {
    /// Validator pubkey
    pub validator: Pubkey,
    /// Amount staked (in lamports)
    pub staked_amount: u64,
    /// Timestamp when staked
    pub staked_at: i64,
    /// Slot of last anchor
    pub last_anchor_slot: u64,
    /// Total anchors submitted
    pub anchor_count: u64,
    /// Total rewards earned (cumulative)
    pub rewards_earned: u64,
    /// Total rewards claimed
    pub rewards_claimed: u64,
    /// Number of times slashed
    pub slash_count: u32,
    /// Total amount slashed
    pub total_slashed: u64,
    /// Slot when unstake was requested (0 if not requested)
    pub unstake_requested_slot: u64,
    /// Whether validator is currently active
    pub is_active: bool,
    /// PDA bump
    pub bump: u8,
}

// ═══════════════════════════════════════════════════════════════════════════
// Contexts
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<CynicState>(),
        seeds = [b"cynic_state"],
        bump
    )]
    pub state: Account<'info, CynicState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageValidator<'info> {
    #[account(
        mut,
        seeds = [b"cynic_state"],
        bump = state.bump,
        has_one = authority
    )]
    pub state: Account<'info, CynicState>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(merkle_root: [u8; 32], item_count: u32, block_height: u64)]
pub struct AnchorRoot<'info> {
    #[account(
        mut,
        seeds = [b"cynic_state"],
        bump = state.bump
    )]
    pub state: Account<'info, CynicState>,

    #[account(
        init,
        payer = validator,
        space = 8 + std::mem::size_of::<RootEntry>(),
        seeds = [b"root", merkle_root.as_ref()],
        bump
    )]
    pub root_entry: Account<'info, RootEntry>,

    #[account(mut)]
    pub validator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(merkle_root: [u8; 32])]
pub struct VerifyRoot<'info> {
    #[account(
        seeds = [b"root", merkle_root.as_ref()],
        bump
    )]
    pub root_entry: Account<'info, RootEntry>,
}

#[derive(Accounts)]
#[instruction(merkle_root: [u8; 32], item_hash: [u8; 32], proof: Vec<[u8; 32]>, proof_flags: Vec<bool>)]
pub struct VerifyInclusion<'info> {
    #[account(
        seeds = [b"root", merkle_root.as_ref()],
        bump
    )]
    pub root_entry: Account<'info, RootEntry>,
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [b"cynic_state"],
        bump = state.bump,
        has_one = authority
    )]
    pub state: Account<'info, CynicState>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(burned_account: Pubkey, amount: u64, reason: u8, merkle_root: [u8; 32])]
pub struct RecordBurn<'info> {
    #[account(
        mut,
        seeds = [b"cynic_state"],
        bump = state.bump
    )]
    pub state: Account<'info, CynicState>,

    #[account(
        init_if_needed,
        payer = reporter,
        space = 8 + std::mem::size_of::<BurnTracker>(),
        seeds = [b"burn_tracker"],
        bump
    )]
    pub burn_tracker: Account<'info, BurnTracker>,

    #[account(
        init,
        payer = reporter,
        space = 8 + std::mem::size_of::<BurnEntry>(),
        seeds = [b"burn", burned_account.as_ref(), &burn_tracker.burn_count.to_le_bytes()],
        bump
    )]
    pub burn_entry: Account<'info, BurnEntry>,

    #[account(mut)]
    pub reporter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetBurnStats<'info> {
    #[account(
        seeds = [b"burn_tracker"],
        bump = burn_tracker.bump
    )]
    pub burn_tracker: Account<'info, BurnTracker>,
}

#[derive(Accounts)]
#[instruction(target_account: Pubkey, delta: i64, contribution_type: u8, judgment_id: [u8; 32])]
pub struct UpdateEScore<'info> {
    #[account(
        seeds = [b"cynic_state"],
        bump = state.bump
    )]
    pub state: Account<'info, CynicState>,

    #[account(
        init_if_needed,
        payer = reporter,
        space = 8 + std::mem::size_of::<EScoreEntry>(),
        seeds = [b"escore", target_account.as_ref()],
        bump
    )]
    pub escore_entry: Account<'info, EScoreEntry>,

    #[account(mut)]
    pub reporter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetEScore<'info> {
    #[account(
        seeds = [b"escore", escore_entry.account.as_ref()],
        bump = escore_entry.bump
    )]
    pub escore_entry: Account<'info, EScoreEntry>,
}

#[derive(Accounts)]
#[instruction(merkle_root: [u8; 32])]
pub struct SnapshotEScore<'info> {
    #[account(
        seeds = [b"escore", escore_entry.account.as_ref()],
        bump = escore_entry.bump
    )]
    pub escore_entry: Account<'info, EScoreEntry>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + std::mem::size_of::<EScoreSnapshotTracker>(),
        seeds = [b"escore_snapshot_tracker", escore_entry.account.as_ref()],
        bump
    )]
    pub snapshot_tracker: Account<'info, EScoreSnapshotTracker>,

    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<EScoreSnapshot>(),
        seeds = [b"escore_snapshot", escore_entry.account.as_ref(), &snapshot_tracker.snapshot_count.to_le_bytes()],
        bump
    )]
    pub escore_snapshot: Account<'info, EScoreSnapshot>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct StakeValidator<'info> {
    #[account(
        init_if_needed,
        payer = staker,
        space = 8 + std::mem::size_of::<ValidatorStake>(),
        seeds = [b"validator_stake", staker.key().as_ref()],
        bump
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    /// CHECK: PDA vault that holds staked SOL
    #[account(
        mut,
        seeds = [b"stake_vault"],
        bump
    )]
    pub stake_vault: AccountInfo<'info>,

    #[account(mut)]
    pub staker: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestUnstake<'info> {
    #[account(
        mut,
        seeds = [b"validator_stake", validator.key().as_ref()],
        bump = validator_stake.bump,
        has_one = validator
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    pub validator: Signer<'info>,
}

#[derive(Accounts)]
pub struct CompleteUnstake<'info> {
    #[account(
        mut,
        seeds = [b"validator_stake", validator.key().as_ref()],
        bump = validator_stake.bump,
        close = validator
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    /// CHECK: PDA vault that holds staked SOL
    #[account(
        mut,
        seeds = [b"stake_vault"],
        bump
    )]
    pub stake_vault: AccountInfo<'info>,

    #[account(mut)]
    pub validator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
        seeds = [b"validator_stake", validator.key().as_ref()],
        bump = validator_stake.bump,
        has_one = validator
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    /// CHECK: PDA vault that holds reward SOL
    #[account(
        mut,
        seeds = [b"reward_vault"],
        bump
    )]
    pub reward_vault: AccountInfo<'info>,

    #[account(mut)]
    pub validator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SlashValidator<'info> {
    #[account(
        mut,
        seeds = [b"cynic_state"],
        bump = state.bump,
        has_one = authority
    )]
    pub state: Account<'info, CynicState>,

    #[account(
        mut,
        seeds = [b"validator_stake", validator_stake.validator.as_ref()],
        bump = validator_stake.bump
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AwardAnchorReward<'info> {
    #[account(
        mut,
        seeds = [b"validator_stake", validator.key().as_ref()],
        bump = validator_stake.bump,
        has_one = validator
    )]
    pub validator_stake: Account<'info, ValidatorStake>,

    pub validator: Signer<'info>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Events
// ═══════════════════════════════════════════════════════════════════════════

#[event]
pub struct ValidatorAdded {
    pub validator: Pubkey,
    pub total_validators: u8,
}

#[event]
pub struct ValidatorRemoved {
    pub validator: Pubkey,
    pub total_validators: u8,
}

#[event]
pub struct RootAnchored {
    pub index: u64,
    pub merkle_root: [u8; 32],
    pub item_count: u32,
    pub block_height: u64,
    pub validator: Pubkey,
    pub slot: u64,
}

#[event]
pub struct RootVerified {
    pub merkle_root: [u8; 32],
    pub index: u64,
    pub slot: u64,
    pub validator: Pubkey,
}

#[event]
pub struct InclusionVerified {
    pub merkle_root: [u8; 32],
    pub item_hash: [u8; 32],
    pub proof_length: u8,
    pub root_index: u64,
    pub root_slot: u64,
}

#[event]
pub struct AuthorityTransferred {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[event]
pub struct BurnRecorded {
    pub index: u64,
    pub burned_account: Pubkey,
    pub amount: u64,
    pub reason: u8,
    pub merkle_root: [u8; 32],
    pub reporter: Pubkey,
    pub slot: u64,
}

#[event]
pub struct BurnStatsQueried {
    pub burn_count: u64,
    pub total_burned: u64,
    pub last_burn_slot: u64,
}

#[event]
pub struct EScoreUpdated {
    pub account: Pubkey,
    pub old_score: i64,
    pub new_score: i64,
    pub delta: i64,
    pub contribution_type: u8,
    pub judgment_id: [u8; 32],
    pub reporter: Pubkey,
    pub slot: u64,
}

#[event]
pub struct EScoreQueried {
    pub account: Pubkey,
    pub score: i64,
    pub update_count: u64,
    pub judgment_count: u32,
    pub validation_count: u32,
    pub learning_count: u32,
}

#[event]
pub struct EScoreSnapshotted {
    pub index: u64,
    pub account: Pubkey,
    pub score: i64,
    pub merkle_root: [u8; 32],
    pub slot: u64,
}

#[event]
pub struct ValidatorStaked {
    pub validator: Pubkey,
    pub amount: u64,
    pub total_staked: u64,
    pub slot: u64,
}

#[event]
pub struct UnstakeRequested {
    pub validator: Pubkey,
    pub staked_amount: u64,
    pub cooldown_ends_slot: u64,
}

#[event]
pub struct UnstakeCompleted {
    pub validator: Pubkey,
    pub amount: u64,
    pub slot: u64,
}

#[event]
pub struct RewardsClaimed {
    pub validator: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
    pub slot: u64,
}

#[event]
pub struct ValidatorSlashed {
    pub validator: Pubkey,
    pub slash_amount: u64,
    pub reason: u8,
    pub evidence_root: [u8; 32],
    pub remaining_stake: u64,
    pub is_deactivated: bool,
    pub slot: u64,
}

#[event]
pub struct AnchorRewardAwarded {
    pub validator: Pubkey,
    pub reward_amount: u64,
    pub total_rewards: u64,
    pub anchor_count: u64,
    pub slot: u64,
}

// ═══════════════════════════════════════════════════════════════════════════
// Errors
// ═══════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum CynicError {
    #[msg("Too many validators (max 21)")]
    TooManyValidators,

    #[msg("Already a validator")]
    AlreadyValidator,

    #[msg("Not a validator")]
    NotValidator,

    #[msg("Root not found")]
    RootNotFound,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Invalid burn reason code")]
    InvalidBurnReason,

    #[msg("Invalid E-Score contribution type")]
    InvalidEScoreType,

    #[msg("E-Score update on cooldown (55 slots required)")]
    EScoreCooldown,

    #[msg("Merkle proof exceeds maximum depth (32)")]
    ProofTooLong,

    #[msg("Proof flags length must match proof length")]
    ProofFlagsMismatch,

    #[msg("Invalid merkle proof - computed root does not match")]
    InvalidMerkleProof,

    #[msg("Insufficient stake (minimum ~0.1 SOL required)")]
    InsufficientStake,

    #[msg("Validator is not active")]
    ValidatorNotActive,

    #[msg("Validator is still active - request unstake first")]
    ValidatorStillActive,

    #[msg("Unstake has not been requested")]
    UnstakeNotRequested,

    #[msg("Unstake cooldown not complete (233 slots required)")]
    UnstakeCooldownNotComplete,

    #[msg("No rewards available to claim")]
    NoRewardsToClaim,
}
