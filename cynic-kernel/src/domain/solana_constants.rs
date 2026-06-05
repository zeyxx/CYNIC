//! Solana on-chain constants — single source of truth for AMM IDs, burn/locker addresses.
//! K11: extracted at 2nd occurrence. K16: zero duplication.

/// Known AMM/DEX program IDs — token accounts owned by these are LP positions.
pub const AMM_PROGRAMS: &[&str] = &[
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM v4
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CLMM
    "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", // Raydium CPMM
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",  // Orca Whirlpool
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",  // Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", // Meteora pools
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", // Orca v1
    "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", // Orca v2 (aquafarm)
    "2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c", // Lifinity v2
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",  // Pump.fun AMM (bonding curve, pre-graduation)
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",  // PumpSwap AMM (post-graduation)
];

/// Known Solana burn addresses — tokens sent here are irrecoverable.
pub const BURN_ADDRESSES: &[&str] = &[
    "1nc1nerator11111111111111111111111111111111",
    "1111111111111111111111111111111111111111111",
    "burnedFi11111111111111111111111111111111111", // burnedFi vanity burn address
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // Raydium burn vault
];

/// Known locker programs — LP tokens held by these are locked, not burned.
pub const LOCKER_PROGRAMS: &[&str] = &[
    "8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX", // Streamflow
    "2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg", // Team.finance / Uncx
];

/// Oracle programs — sources of truth for price and external data.
pub const ORACLE_PROGRAMS: &[&str] = &[
    "rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ", // Pyth Solana Receiver (Pull-based)
    "SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv", // Switchboard On-Demand (V3)
    "pythWSnswVUd12oZpeFP8e9CVaEqJg25g1Vtc2biRsT", // Pyth Price Feed
    "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2KPt", // Pyth Legacy
    "orac1eFjzWL5R3RbbdMV68K9H6TaCVVcL6LjvQQWAbz", // Switchboard Quote Program
];

/// Infrastructure and System programs.
pub const INFRA_PROGRAMS: &[&str] = &[
    "ComputeBudget111111111111111111111111111111", // Compute Budget Program
    "JitoTiPevE9D2x6pkA7P6CYnATWzjtneQLhbK7Yv3uS",  // Jito Tip Payment
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
    fn oracle_programs_are_valid_base58() {
        for addr in ORACLE_PROGRAMS {
            assert!(
                is_valid_base58(addr),
                "invalid base58 in ORACLE_PROGRAMS: {addr}"
            );
        }
    }

    #[test]
    fn infra_programs_are_valid_base58() {
        for addr in INFRA_PROGRAMS {
            assert!(
                is_valid_base58(addr),
                "invalid base58 in INFRA_PROGRAMS: {addr}"
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
        let all_cats = [
            ("AMM", AMM_PROGRAMS),
            ("BURN", BURN_ADDRESSES),
            ("LOCKER", LOCKER_PROGRAMS),
            ("ORACLE", ORACLE_PROGRAMS),
            ("INFRA", INFRA_PROGRAMS),
        ];

        for (i, (name1, cat1)) in all_cats.iter().enumerate() {
            for (name2, cat2) in all_cats.iter().skip(i + 1) {
                for addr in *cat1 {
                    assert!(
                        !cat2.contains(addr),
                        "Address {addr} found in both {name1} and {name2}"
                    );
                }
            }
        }
    }
}
