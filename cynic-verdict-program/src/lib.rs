//! CYNIC Verdict Program — on-chain verdict recording.
//!
//! ## Accounts
//!
//! ### CounterAccount (PDA: ["cynic-counter"])
//! Global counter of recorded verdicts. Initialized once.
//! Size: 8 bytes (u64).
//!
//! ### VerdictAccount (PDA: ["verdict", verdict_id: [u8;16]])
//! One account per verdict. Immutable after creation.
//! Size: VERDICT_ACCOUNT_LEN bytes.
//!
//! ## Instructions
//!
//! ### 0 — InitCounter
//! One-time init. Creates the CounterAccount PDA.
//! Accounts: [counter PDA (writable), payer (signer+writable), system_program]
//!
//! ### 1 — RecordVerdict
//! Creates a VerdictAccount PDA and increments the counter.
//! Accounts: [verdict PDA (writable), counter PDA (writable), recorder (signer+writable), system_program]
//! Data: RecordVerdictArgs

use solana_program::{
    account_info::{AccountInfo, next_account_info},
    clock::Clock,
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

declare_id!("AKjCbxzdjXHcTmTqN37K7eZM2RUsCYTmaXUriTd6csBH");

// ── Verdict kind enum ─────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerdictKind {
    Bark = 0,
    Growl = 1,
    Wag = 2,
    Howl = 3,
    Epoche = 4,
}

impl TryFrom<u8> for VerdictKind {
    type Error = ProgramError;
    fn try_from(v: u8) -> Result<Self, ProgramError> {
        match v {
            0 => Ok(Self::Bark),
            1 => Ok(Self::Growl),
            2 => Ok(Self::Wag),
            3 => Ok(Self::Howl),
            4 => Ok(Self::Epoche),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// ── Account layouts ───────────────────────────────────────────────────────────

/// CounterAccount — 8 bytes, PDA ["cynic-counter"]
pub const COUNTER_LEN: usize = 8;

pub struct CounterAccount<'a>(&'a mut [u8]);

impl<'a> CounterAccount<'a> {
    pub fn count(&self) -> u64 {
        u64::from_le_bytes(self.0[..8].try_into().unwrap())
    }
    pub fn increment(&mut self) {
        let v = self.count().saturating_add(1);
        self.0[..8].copy_from_slice(&v.to_le_bytes());
    }
}

/// VerdictAccount — fixed layout, PDA ["verdict", verdict_id]
///
/// Layout (65 bytes):
///   [0..16]  verdict_id: [u8; 16]    UUID as raw bytes
///   [16]     kind: u8                VerdictKind
///   [17..21] q_score_millis: u32     q_score × 1000 (e.g. 0.456 → 456)
///   [21..53] recorder: [u8; 32]      wallet that recorded
///   [53..61] timestamp: i64          Unix timestamp (Clock::unix_timestamp)
pub const VERDICT_ACCOUNT_LEN: usize = 65;

pub struct VerdictAccount<'a>(&'a mut [u8]);

impl<'a> VerdictAccount<'a> {
    pub fn write(
        &mut self,
        verdict_id: [u8; 16],
        kind: VerdictKind,
        q_score_millis: u32,
        recorder: Pubkey,
        timestamp: i64,
    ) {
        self.0[0..16].copy_from_slice(&verdict_id);
        self.0[16] = kind as u8;
        self.0[17..21].copy_from_slice(&q_score_millis.to_le_bytes());
        self.0[21..53].copy_from_slice(recorder.as_ref());
        self.0[53..61].copy_from_slice(&timestamp.to_le_bytes());
    }
}

// ── PDA seeds ─────────────────────────────────────────────────────────────────

pub fn counter_seeds() -> [&'static [u8]; 1] {
    [b"cynic-counter"]
}

pub fn verdict_seeds(verdict_id: &[u8; 16]) -> [&[u8]; 2] {
    [b"verdict", verdict_id.as_slice()]
}

pub fn find_counter_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&counter_seeds(), &id())
}

pub fn find_verdict_pda(verdict_id: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&verdict_seeds(verdict_id), &id())
}

// ── Instruction data ──────────────────────────────────────────────────────────

/// RecordVerdictArgs: 21 bytes
/// [0..16]  verdict_id: [u8; 16]
/// [16]     kind: u8
/// [17..21] q_score_millis: u32 (little-endian)
pub struct RecordVerdictArgs {
    pub verdict_id: [u8; 16],
    pub kind: VerdictKind,
    pub q_score_millis: u32,
}

impl RecordVerdictArgs {
    pub const LEN: usize = 21;

    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        let verdict_id: [u8; 16] = data[0..16].try_into().unwrap();
        let kind = VerdictKind::try_from(data[16])?;
        let q_score_millis = u32::from_le_bytes(data[17..21].try_into().unwrap());
        Ok(Self {
            verdict_id,
            kind,
            q_score_millis,
        })
    }
}

// ── Entrypoint ────────────────────────────────────────────────────────────────

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        0 => process_init_counter(program_id, accounts),
        1 => process_record_verdict(program_id, accounts, &instruction_data[1..]),
        _ => {
            msg!("Unknown instruction: {}", instruction_data[0]);
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

// ── Instruction 0: InitCounter ────────────────────────────────────────────────

fn process_init_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let counter_ai = next_account_info(iter)?;
    let payer_ai = next_account_info(iter)?;
    let system_ai = next_account_info(iter)?;

    let (counter_pda, bump) = find_counter_pda();
    if counter_ai.key != &counter_pda {
        msg!("Counter PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if !counter_ai.data_is_empty() {
        msg!("Counter already initialized");
        return Ok(());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(COUNTER_LEN);
    let seeds: &[&[u8]] = &[b"cynic-counter", &[bump]];

    invoke_signed(
        &system_instruction::create_account(
            payer_ai.key,
            &counter_pda,
            lamports,
            COUNTER_LEN as u64,
            program_id,
        ),
        &[payer_ai.clone(), counter_ai.clone(), system_ai.clone()],
        &[seeds],
    )?;

    msg!("CYNIC counter initialized at {}", counter_pda);
    Ok(())
}

// ── Instruction 1: RecordVerdict ──────────────────────────────────────────────

fn process_record_verdict(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let args = RecordVerdictArgs::unpack(data)?;

    let iter = &mut accounts.iter();
    let verdict_ai = next_account_info(iter)?;
    let counter_ai = next_account_info(iter)?;
    let recorder_ai = next_account_info(iter)?;
    let system_ai = next_account_info(iter)?;

    if !recorder_ai.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify verdict PDA
    let (verdict_pda, verdict_bump) = find_verdict_pda(&args.verdict_id);
    if verdict_ai.key != &verdict_pda {
        msg!("Verdict PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if !verdict_ai.data_is_empty() {
        msg!("Verdict already recorded: idempotent, skipping");
        return Ok(());
    }

    // Verify counter PDA
    let (counter_pda, _) = find_counter_pda();
    if counter_ai.key != &counter_pda {
        msg!("Counter PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Create verdict account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(VERDICT_ACCOUNT_LEN);
    let verdict_id = args.verdict_id;
    let seeds: &[&[u8]] = &[b"verdict", &verdict_id, &[verdict_bump]];

    invoke_signed(
        &system_instruction::create_account(
            recorder_ai.key,
            &verdict_pda,
            lamports,
            VERDICT_ACCOUNT_LEN as u64,
            program_id,
        ),
        &[recorder_ai.clone(), verdict_ai.clone(), system_ai.clone()],
        &[seeds],
    )?;

    // Write verdict data
    let timestamp = Clock::get()?.unix_timestamp;
    let mut data_ref = verdict_ai.try_borrow_mut_data()?;
    let mut acct = VerdictAccount(&mut data_ref);
    acct.write(
        args.verdict_id,
        args.kind,
        args.q_score_millis,
        *recorder_ai.key,
        timestamp,
    );

    // Increment counter
    let mut counter_data = counter_ai.try_borrow_mut_data()?;
    let mut counter = CounterAccount(&mut counter_data);
    let new_count = counter.count().saturating_add(1);
    counter.increment();

    msg!(
        "CYNIC verdict recorded: id={:?} kind={:?} q={} recorder={} ts={} total={}",
        args.verdict_id,
        args.kind,
        args.q_score_millis,
        recorder_ai.key,
        timestamp,
        new_count,
    );

    Ok(())
}

// ── Client helpers (excluded from on-chain build) ─────────────────────────────

#[cfg(not(target_os = "solana"))]
pub mod client {
    use super::*;
    use solana_program::instruction::{AccountMeta, Instruction};

    pub fn init_counter_ix(payer: Pubkey) -> Instruction {
        let (counter_pda, _) = find_counter_pda();
        Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(counter_pda, false),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
            ],
            data: vec![0u8],
        }
    }

    pub fn record_verdict_ix(
        recorder: Pubkey,
        verdict_id: [u8; 16],
        kind: VerdictKind,
        q_score_millis: u32,
    ) -> Instruction {
        let (verdict_pda, _) = find_verdict_pda(&verdict_id);
        let (counter_pda, _) = find_counter_pda();

        let mut data = vec![1u8]; // discriminator
        data.extend_from_slice(&verdict_id);
        data.push(kind as u8);
        data.extend_from_slice(&q_score_millis.to_le_bytes());

        Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(verdict_pda, false),
                AccountMeta::new(counter_pda, false),
                AccountMeta::new(recorder, true),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
            ],
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_kind_roundtrip() {
        for k in [
            VerdictKind::Bark,
            VerdictKind::Growl,
            VerdictKind::Wag,
            VerdictKind::Howl,
            VerdictKind::Epoche,
        ] {
            assert_eq!(VerdictKind::try_from(k as u8).unwrap() as u8, k as u8);
        }
    }

    #[test]
    fn record_verdict_args_unpack() {
        let id = [1u8; 16];
        let mut data = id.to_vec();
        data.push(VerdictKind::Growl as u8);
        data.extend_from_slice(&456u32.to_le_bytes());
        let args = RecordVerdictArgs::unpack(&data).unwrap();
        assert_eq!(args.verdict_id, id);
        assert_eq!(args.kind, VerdictKind::Growl);
        assert_eq!(args.q_score_millis, 456);
    }

    #[test]
    fn pda_derivation_stable() {
        let (pda, _) = find_counter_pda();
        let (pda2, _) = find_counter_pda();
        assert_eq!(pda, pda2);

        let id = [42u8; 16];
        let (vpda, _) = find_verdict_pda(&id);
        let (vpda2, _) = find_verdict_pda(&id);
        assert_eq!(vpda, vpda2);
    }
}
