#![cfg_attr(target_os = "solana", no_std)]
#![allow(unexpected_cfgs)]
#![allow(unused_imports)]

use pinocchio::{
    account_info::AccountInfo, entrypoint, msg, program_error::ProgramError, pubkey::Pubkey,
};

#[cfg(target_os = "solana")]
entrypoint!(process_instruction);

/// Ignis Vault Factory Entrypoint
/// Raw eBPF execution, zero-copy deserialization.
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    msg!("Ignis Protocol: eBPF crank engaged.");

    // Instruction routing
    // 0 = Initialize Vault
    // 1 = Crank Buyback & Burn
    match instruction_data.first() {
        Some(&0) => {
            msg!("Instruction: InitializeVault");
            // Call processor::process_initialize_vault(program_id, accounts, &instruction_data[1..])
        }
        Some(&1) => {
            msg!("Instruction: CrankBuybackAndBurn");
            // Call processor::process_crank(program_id, accounts, &instruction_data[1..])
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}
