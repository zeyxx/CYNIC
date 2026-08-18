use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

#[account(zero_copy)]
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PohRecordAccount {
    pub epoch: u64,
    pub timestamp: i64,
    pub merkle_root: [u8; 32],
    pub round_digest: [u8; 32],
    pub node_authority: Pubkey,
    pub bump: u8,
    pub _reserved: [u8; 7],
}

unsafe impl Zeroable for PohRecordAccount {}
unsafe impl Pod for PohRecordAccount {}

#[account(zero_copy)]
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FutarchyMarketAccount {
    pub proposal_hash: [u8; 32],
    pub yes_pool_units: u64,
    pub no_pool_units: u64,
    pub min_conviction_threshold_bps: u32,
    pub status: u8,   // 0 = Open, 1 = Resolved, 2 = Executed, 3 = Canceled
    pub decision: u8, // 0 = Undecided, 1 = Pass, 2 = Fail
    pub bump: u8,
    pub _reserved: [u8; 5],
}

unsafe impl Zeroable for FutarchyMarketAccount {}
unsafe impl Pod for FutarchyMarketAccount {}

#[account(zero_copy)]
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct IgnisVaultAccount {
    pub mint: Pubkey,
    pub total_burned_units: u64,
    pub last_burn_timestamp: i64,
    pub bump: u8,
    pub _reserved: [u8; 7],
}

unsafe impl Zeroable for IgnisVaultAccount {}
unsafe impl Pod for IgnisVaultAccount {}
