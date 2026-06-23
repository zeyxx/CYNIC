#![no_std]

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

/// Ignis Vault Factory Entrypoint
/// Raw eBPF execution, zero-copy deserialization.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
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
