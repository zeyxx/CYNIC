//! Solana on-chain constants — single source of truth for AMM IDs, burn/locker addresses.
//! K11: extracted at 2nd occurrence. K16: zero duplication.

/// Known AMM/DEX program IDs — token accounts owned by these are LP positions.
pub const AMM_PROGRAMS: &[&str] = &[
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM v4
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CLMM
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",  // Orca Whirlpool
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",  // Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", // Meteora pools
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", // Orca v1
    "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", // Orca v2 (aquafarm)
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",  // PumpSwap AMM
];

/// Known Solana burn addresses — tokens sent here are irrecoverable.
pub const BURN_ADDRESSES: &[&str] = &[
    "1nc1nerator11111111111111111111111111111111",
    "1111111111111111111111111111111111111111111",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // Raydium burn vault
];

/// Known locker programs — LP tokens held by these are locked, not burned.
pub const LOCKER_PROGRAMS: &[&str] = &[
    "8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX", // Streamflow
    "2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg", // Team.finance / Uncx
];

/// System Program — owner of regular wallets.
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

#[cfg(test)]
mod tests {
    use super::*;

    fn is_valid_base58(s: &str) -> bool {
        (32..=44).contains(&s.len())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
    }

    #[test]
    fn amm_programs_are_valid_base58() {
        for addr in AMM_PROGRAMS {
            assert!(
                is_valid_base58(addr),
                "invalid base58 in AMM_PROGRAMS: {addr}"
            );
        }
    }

    #[test]
    fn burn_addresses_valid_length() {
        for addr in BURN_ADDRESSES {
            assert!((32..=44).contains(&addr.len()), "wrong length: {addr}");
        }
    }

    #[test]
    fn no_address_in_multiple_categories() {
        for amm in AMM_PROGRAMS {
            assert!(!BURN_ADDRESSES.contains(amm), "AMM addr in BURN: {amm}");
            assert!(!LOCKER_PROGRAMS.contains(amm), "AMM addr in LOCKER: {amm}");
        }
        for burn in BURN_ADDRESSES {
            assert!(
                !LOCKER_PROGRAMS.contains(burn),
                "BURN addr in LOCKER: {burn}"
            );
        }
    }
}
