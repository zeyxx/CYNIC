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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_pod_layouts_and_sizes() {
        // 1. PohRecordAccount
        let poh = PohRecordAccount {
            epoch: 42,
            timestamp: 1_720_000_000,
            merkle_root: [1u8; 32],
            round_digest: [2u8; 32],
            node_authority: Pubkey::default(),
            bump: 254,
            _reserved: [0u8; 7],
        };

        let bytes = bytemuck::bytes_of(&poh);
        let deserialized: &PohRecordAccount = bytemuck::from_bytes(bytes);
        assert_eq!(deserialized.epoch, 42);
        assert_eq!(deserialized.merkle_root, [1u8; 32]);

        // 2. FutarchyMarketAccount
        let market = FutarchyMarketAccount {
            proposal_hash: [3u8; 32],
            yes_pool_units: 1_000_000,
            no_pool_units: 1_000_000,
            min_conviction_threshold_bps: 6000,
            status: 0,
            decision: 0,
            bump: 255,
            _reserved: [0u8; 5],
        };
        let m_bytes = bytemuck::bytes_of(&market);
        let m_deserialized: &FutarchyMarketAccount = bytemuck::from_bytes(m_bytes);
        assert_eq!(m_deserialized.yes_pool_units, 1_000_000);
        assert_eq!(m_deserialized.min_conviction_threshold_bps, 6000);
    }
}
