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

/// Maximum roots to store (ring buffer)
pub const MAX_ROOTS: usize = 377; // F(14)

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

    /// Anchor a merkle root
    /// Only validators can anchor roots
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
pub struct AuthorityTransferred {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
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
}
