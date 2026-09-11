//! Compile-time Castagnoli CRC32C calculator with zero external dependencies.
//! Uses polynomial 0x82F63B78 (reflected / reversed representation).

const fn make_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32C_TABLE: [u32; 256] = make_crc32c_table();

/// Compute Castagnoli CRC32C checksum over a byte slice.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        let index = ((crc ^ (byte as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[index];
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32c_standard_test_vector() {
        // Standard ASCII test vector: "123456789" has known Castagnoli CRC-32C value 0xE3069283
        let input = b"123456789";
        let result = crc32c(input);
        assert_eq!(result, 0xE306_9283);
    }

    #[test]
    fn crc32c_empty_vector() {
        assert_eq!(crc32c(b""), 0);
    }
}
