//! Content hashing and normalization for crystal IDs.

/// Deterministic content hash for crystal IDs. FNV-1a — not cryptographic,
/// just stable content addressing for deduplication. Changing this algo
/// would orphan all existing crystals in the DB.
pub fn content_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

/// Normalize content before FNV hashing to reduce fragmentation.
/// Lowercase + collapse whitespace + trim. Two stimuli that differ only
/// in casing or spacing will hash to the same crystal ID.
pub fn normalize_for_hash(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
