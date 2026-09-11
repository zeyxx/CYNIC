//! Binary WAL frame encoding, decoding, size validation, and cryptographic hashing.

use bytes::Bytes;
use sha3::{Digest, Sha3_512};
use std::io::{self, Read, Write};

use super::crc::crc32c;
use crate::error::StorageError;
use crate::types::{CausalWitness, Hash512, PoHTick, SequenceNumber, WalEntry};

/// Magic bytes identifying a CYNIC WAL frame ("CYNW").
pub const WAL_FRAME_MAGIC: [u8; 4] = [0x43, 0x59, 0x4E, 0x57];

/// Current binary frame codec version.
pub const WAL_FRAME_VERSION: u16 = 1;

/// Maximum payload size permitted in a single frame (16 MiB) to prevent allocation DOS.
pub const MAX_PAYLOAD_LEN: usize = 16 * 1024 * 1024;

/// Maximum schema identifier length (256 bytes).
pub const MAX_SCHEMA_LEN: usize = 256;

/// Maximum mandate identifier length (128 bytes).
pub const MAX_MANDATE_ID_LEN: usize = 128;

/// Minimum possible valid frame length on disk.
pub const MIN_FRAME_LEN: usize = 4  // magic
    + 2                             // version
    + 4                             // payload_len
    + 4                             // header_crc32c
    + 8                             // sequence
    + 8                             // poh_tick
    + 64                            // prev_hash
    + 2                             // schema_len
    + 0                             // schema (min 0)
    + 0                             // payload (min 0)
    + 32                            // principal_pubkey
    + 2                             // mandate_id_len
    + 1                             // proof_anchor_present
    + 64                            // signature
    + 64; // entry_hash = 255 bytes

/// Domain separation string for Stage 1 commit hash.
const COMMIT_HASH_DOMAIN: &[u8] = b"CYNIC_WAL_COMMIT_V1\0";

/// Domain separation string for Stage 2 entry hash.
const ENTRY_HASH_DOMAIN: &[u8] = b"CYNIC_WAL_ENTRY_V1\0";

/// Computes the Stage 1 commit hash covering event metadata and payload.
pub fn compute_commit_hash(
    seq: SequenceNumber,
    prev_hash: &Hash512,
    poh_tick: PoHTick,
    schema: &str,
    payload: &[u8],
    mandate_id: Option<&str>,
    proof_anchor: Option<&Hash512>,
) -> Hash512 {
    let mut hasher = Sha3_512::new();
    hasher.update(COMMIT_HASH_DOMAIN);
    hasher.update(seq.0.to_be_bytes());
    hasher.update(prev_hash.0);
    hasher.update(poh_tick.0.to_be_bytes());
    hasher.update((schema.len() as u16).to_be_bytes());
    hasher.update(schema.as_bytes());
    hasher.update((payload.len() as u32).to_be_bytes());
    hasher.update(payload);

    if let Some(mid) = mandate_id {
        hasher.update((mid.len() as u16).to_be_bytes());
        hasher.update(mid.as_bytes());
    } else {
        hasher.update(0u16.to_be_bytes());
    }

    if let Some(anchor) = proof_anchor {
        hasher.update([1u8]);
        hasher.update(anchor.0);
    } else {
        hasher.update([0u8]);
    }

    let result = hasher.finalize();
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&result);
    Hash512(arr)
}

/// Computes the Stage 2 entry hash binding commit hash to the cryptographic witness.
pub fn compute_entry_hash(
    commit_hash: &Hash512,
    principal_pubkey: &[u8; 32],
    signature: &[u8; 64],
) -> Hash512 {
    let mut hasher = Sha3_512::new();
    hasher.update(ENTRY_HASH_DOMAIN);
    hasher.update(commit_hash.0);
    hasher.update(principal_pubkey);
    hasher.update(signature);

    let result = hasher.finalize();
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&result);
    Hash512(arr)
}

/// Errors occurring during frame decoding.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// Zero bytes read; clean end of stream.
    CleanEof,
    /// Incomplete or truncated frame at the end of the file.
    TornTail { bytes_read: usize },
    /// Magic header mismatch.
    InvalidMagic([u8; 4]),
    /// Unsupported frame version.
    UnsupportedVersion(u16),
    /// Payload exceeds maximum permissible size.
    PayloadTooLarge { len: usize, max: usize },
    /// Header CRC32C checksum mismatch.
    HeaderCrcMismatch { expected: u32, actual: u32 },
    /// Commit or entry cryptographic hash mismatch.
    CryptographicHashMismatch {
        sequence: SequenceNumber,
        expected: Hash512,
        actual: Hash512,
    },
    /// UTF-8 decode error.
    InvalidUtf8(std::str::Utf8Error),
    /// Standard I/O error.
    Io(io::Error),
}

impl From<io::Error> for FrameDecodeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<FrameDecodeError> for StorageError {
    fn from(err: FrameDecodeError) -> Self {
        match err {
            FrameDecodeError::CleanEof => StorageError::Io("unexpected clean EOF".to_string()),
            FrameDecodeError::TornTail { bytes_read } => {
                StorageError::Io(format!("torn tail at EOF ({bytes_read} bytes)"))
            }
            FrameDecodeError::InvalidMagic(m) => {
                StorageError::Codec(format!("invalid WAL magic: {m:?}"))
            }
            FrameDecodeError::UnsupportedVersion(v) => {
                StorageError::Codec(format!("unsupported WAL version: {v}"))
            }
            FrameDecodeError::PayloadTooLarge { len, max } => {
                StorageError::PayloadTooLarge { len, max }
            }
            FrameDecodeError::HeaderCrcMismatch { expected, actual } => {
                StorageError::Codec(format!("header CRC mismatch: exp={expected}, act={actual}"))
            }
            FrameDecodeError::CryptographicHashMismatch {
                sequence,
                expected,
                actual,
            } => StorageError::HashChainCorrupt {
                sequence,
                expected_prev: expected,
                actual_prev: actual,
            },
            FrameDecodeError::InvalidUtf8(e) => StorageError::Codec(format!("invalid UTF-8: {e}")),
            FrameDecodeError::Io(e) => StorageError::Io(e.to_string()),
        }
    }
}

/// Encodes a single `WalEntry` into its canonical on-disk binary frame representation.
pub fn encode_frame(entry: &WalEntry) -> Result<Vec<u8>, StorageError> {
    if entry.payload.len() > MAX_PAYLOAD_LEN {
        return Err(StorageError::PayloadTooLarge {
            len: entry.payload.len(),
            max: MAX_PAYLOAD_LEN,
        });
    }
    if entry.schema.len() > MAX_SCHEMA_LEN {
        return Err(StorageError::Codec(format!(
            "schema len exceeds max: {} > {}",
            entry.schema.len(),
            MAX_SCHEMA_LEN
        )));
    }
    if let Some(mid) = &entry.witness.mandate_id {
        if mid.len() > MAX_MANDATE_ID_LEN {
            return Err(StorageError::Codec(format!(
                "mandate_id len exceeds max: {} > {}",
                mid.len(),
                MAX_MANDATE_ID_LEN
            )));
        }
    }

    let payload_len = entry.payload.len() as u32;

    // Fixed header prefix: magic(4) + version(2) + payload_len(4) = 10 bytes
    let mut header_prefix = [0u8; 10];
    header_prefix[0..4].copy_from_slice(&WAL_FRAME_MAGIC);
    header_prefix[4..6].copy_from_slice(&WAL_FRAME_VERSION.to_be_bytes());
    header_prefix[6..10].copy_from_slice(&payload_len.to_be_bytes());

    let header_crc = crc32c(&header_prefix);

    let mut buf = Vec::with_capacity(MIN_FRAME_LEN + entry.schema.len() + entry.payload.len());
    buf.extend_from_slice(&header_prefix);
    buf.extend_from_slice(&header_crc.to_be_bytes());

    // SequenceNumber + PoHTick + PrevHash
    buf.extend_from_slice(&entry.sequence.0.to_be_bytes());
    buf.extend_from_slice(&entry.poh_tick.0.to_be_bytes());
    buf.extend_from_slice(&entry.previous_hash.0);

    // Schema
    buf.extend_from_slice(&(entry.schema.len() as u16).to_be_bytes());
    buf.extend_from_slice(entry.schema.as_bytes());

    // Payload
    buf.extend_from_slice(&entry.payload);

    // Causal Witness
    buf.extend_from_slice(&entry.witness.principal_pubkey);
    if let Some(mid) = &entry.witness.mandate_id {
        buf.extend_from_slice(&(mid.len() as u16).to_be_bytes());
        buf.extend_from_slice(mid.as_bytes());
    } else {
        buf.extend_from_slice(&0u16.to_be_bytes());
    }

    if let Some(anchor) = &entry.witness.proof_anchor {
        buf.push(1u8);
        buf.extend_from_slice(&anchor.0);
    } else {
        buf.push(0u8);
    }

    buf.extend_from_slice(&entry.witness.signature);

    // Cryptographic Entry Hash
    buf.extend_from_slice(&entry.entry_hash.0);

    Ok(buf)
}

/// Decodes a single `WalEntry` from an input reader.
/// Returns `Ok(Some(entry))` on success, `Ok(None)` on clean EOF, or `Err(FrameDecodeError)`.
pub fn decode_frame<R: Read>(reader: &mut R) -> Result<Option<WalEntry>, FrameDecodeError> {
    // 1. Read header prefix (14 bytes): magic(4) + version(2) + payload_len(4) + header_crc(4)
    let mut header_buf = [0u8; 14];
    let mut bytes_read = 0;
    while bytes_read < 14 {
        match reader.read(&mut header_buf[bytes_read..]) {
            Ok(0) => {
                if bytes_read == 0 {
                    return Ok(None); // Clean EOF
                } else {
                    return Err(FrameDecodeError::TornTail { bytes_read });
                }
            }
            Ok(n) => bytes_read += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FrameDecodeError::Io(e)),
        }
    }

    // Verify magic
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&header_buf[0..4]);
    if magic != WAL_FRAME_MAGIC {
        return Err(FrameDecodeError::InvalidMagic(magic));
    }

    // Verify version
    let version = u16::from_be_bytes([header_buf[4], header_buf[5]]);
    if version != WAL_FRAME_VERSION {
        return Err(FrameDecodeError::UnsupportedVersion(version));
    }

    // Verify payload length bound
    let payload_len =
        u32::from_be_bytes([header_buf[6], header_buf[7], header_buf[8], header_buf[9]]) as usize;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(FrameDecodeError::PayloadTooLarge {
            len: payload_len,
            max: MAX_PAYLOAD_LEN,
        });
    }

    // Verify header CRC32C
    let expected_crc = u32::from_be_bytes([
        header_buf[10],
        header_buf[11],
        header_buf[12],
        header_buf[13],
    ]);
    let actual_crc = crc32c(&header_buf[0..10]);
    if actual_crc != expected_crc {
        return Err(FrameDecodeError::HeaderCrcMismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    // 2. Read SequenceNumber(8) + PoHTick(8) + PrevHash(64) = 80 bytes
    let mut seq_poh_prev = [0u8; 80];
    reader.read_exact(&mut seq_poh_prev).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            FrameDecodeError::TornTail { bytes_read: 14 }
        } else {
            FrameDecodeError::Io(e)
        }
    })?;

    let sequence = SequenceNumber(u64::from_be_bytes(seq_poh_prev[0..8].try_into().unwrap()));
    let poh_tick = PoHTick(u64::from_be_bytes(seq_poh_prev[8..16].try_into().unwrap()));
    let mut prev_hash = [0u8; 64];
    prev_hash.copy_from_slice(&seq_poh_prev[16..80]);
    let previous_hash = Hash512(prev_hash);

    // 3. Read SchemaLen(2)
    let mut schema_len_buf = [0u8; 2];
    reader.read_exact(&mut schema_len_buf)?;
    let schema_len = u16::from_be_bytes(schema_len_buf) as usize;
    if schema_len > MAX_SCHEMA_LEN {
        return Err(FrameDecodeError::PayloadTooLarge {
            len: schema_len,
            max: MAX_SCHEMA_LEN,
        });
    }

    // 4. Read Schema bytes
    let mut schema_bytes = vec![0u8; schema_len];
    reader.read_exact(&mut schema_bytes)?;
    let schema = match String::from_utf8(schema_bytes) {
        Ok(s) => s,
        Err(e) => return Err(FrameDecodeError::InvalidUtf8(e.utf8_error())),
    };

    // 5. Read Payload
    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload)?;

    // 6. Read Causal Witness:
    // principal_pubkey(32)
    let mut pubkey = [0u8; 32];
    reader.read_exact(&mut pubkey)?;

    // mandate_id_len(2)
    let mut mandate_len_buf = [0u8; 2];
    reader.read_exact(&mut mandate_len_buf)?;
    let mandate_len = u16::from_be_bytes(mandate_len_buf) as usize;
    if mandate_len > MAX_MANDATE_ID_LEN {
        return Err(FrameDecodeError::PayloadTooLarge {
            len: mandate_len,
            max: MAX_MANDATE_ID_LEN,
        });
    }

    let mandate_id = if mandate_len > 0 {
        let mut mandate_bytes = vec![0u8; mandate_len];
        reader.read_exact(&mut mandate_bytes)?;
        Some(
            String::from_utf8(mandate_bytes)
                .map_err(|e| FrameDecodeError::InvalidUtf8(e.utf8_error()))?,
        )
    } else {
        None
    };

    // proof_anchor_present(1)
    let mut anchor_present_buf = [0u8; 1];
    reader.read_exact(&mut anchor_present_buf)?;
    let proof_anchor = if anchor_present_buf[0] == 1 {
        let mut anchor_bytes = [0u8; 64];
        reader.read_exact(&mut anchor_bytes)?;
        Some(Hash512(anchor_bytes))
    } else {
        None
    };

    // signature(64)
    let mut signature = [0u8; 64];
    reader.read_exact(&mut signature)?;

    // 7. Read EntryHashDigest(64)
    let mut entry_hash_bytes = [0u8; 64];
    reader.read_exact(&mut entry_hash_bytes)?;
    let entry_hash = Hash512(entry_hash_bytes);

    // 8. Recompute hashes to verify cryptographic integrity
    let commit_hash = compute_commit_hash(
        sequence,
        &previous_hash,
        poh_tick,
        &schema,
        &payload,
        mandate_id.as_deref(),
        proof_anchor.as_ref(),
    );

    let recomputed_entry_hash = compute_entry_hash(&commit_hash, &pubkey, &signature);
    if recomputed_entry_hash != entry_hash {
        return Err(FrameDecodeError::CryptographicHashMismatch {
            sequence,
            expected: entry_hash,
            actual: recomputed_entry_hash,
        });
    }

    let witness = CausalWitness {
        principal_pubkey: pubkey,
        signature,
        mandate_id,
        proof_anchor,
    };

    Ok(Some(WalEntry {
        sequence,
        poh_tick,
        previous_hash,
        commit_hash,
        entry_hash,
        schema,
        payload: Bytes::from(payload),
        witness,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_witness() -> CausalWitness {
        CausalWitness {
            principal_pubkey: [1u8; 32],
            signature: [2u8; 64],
            mandate_id: Some("mandate-001".to_string()),
            proof_anchor: Some(Hash512([3u8; 64])),
        }
    }

    #[test]
    fn frame_encode_decode_roundtrip() {
        let witness = dummy_witness();
        let payload = Bytes::from_static(b"hello cynic storage wal");
        let seq = SequenceNumber(1);
        let poh = PoHTick(100);
        let prev = Hash512([0u8; 64]);
        let schema = "cynic.test.event.v1".to_string();

        let commit_hash = compute_commit_hash(
            seq,
            &prev,
            poh,
            &schema,
            &payload,
            witness.mandate_id.as_deref(),
            witness.proof_anchor.as_ref(),
        );

        let entry_hash =
            compute_entry_hash(&commit_hash, &witness.principal_pubkey, &witness.signature);

        let entry = WalEntry {
            sequence: seq,
            poh_tick: poh,
            previous_hash: prev,
            commit_hash,
            entry_hash,
            schema,
            payload,
            witness,
        };

        let encoded = encode_frame(&entry).expect("encode should succeed");
        assert!(encoded.len() >= MIN_FRAME_LEN);

        let mut cursor = io::Cursor::new(encoded);
        let decoded = decode_frame(&mut cursor)
            .expect("decode should succeed")
            .expect("entry should be present");

        assert_eq!(decoded, entry);
    }

    #[test]
    fn frame_decode_detects_torn_tail_at_eof() {
        let witness = dummy_witness();
        let payload = Bytes::from_static(b"torn test");
        let seq = SequenceNumber(1);
        let poh = PoHTick(1);
        let prev = Hash512([0u8; 64]);
        let schema = "test".to_string();
        let commit_hash = compute_commit_hash(seq, &prev, poh, &schema, &payload, None, None);
        let entry_hash =
            compute_entry_hash(&commit_hash, &witness.principal_pubkey, &witness.signature);

        let entry = WalEntry {
            sequence: seq,
            poh_tick: poh,
            previous_hash: prev,
            commit_hash,
            entry_hash,
            schema,
            payload,
            witness,
        };

        let encoded = encode_frame(&entry).unwrap();
        // Truncate the encoded bytes to simulate mid-write crash at EOF
        let truncated = &encoded[..encoded.len() - 15];

        let mut cursor = io::Cursor::new(truncated);
        let err = decode_frame(&mut cursor);
        assert!(matches!(
            err,
            Err(FrameDecodeError::Io(_)) | Err(FrameDecodeError::TornTail { .. })
        ));
    }

    #[test]
    fn frame_rejects_payload_exceeding_max() {
        let witness = dummy_witness();
        let seq = SequenceNumber(1);
        let poh = PoHTick(1);
        let prev = Hash512([0u8; 64]);
        let schema = "test".to_string();

        let entry = WalEntry {
            sequence: seq,
            poh_tick: poh,
            previous_hash: prev,
            commit_hash: Hash512([0u8; 64]),
            entry_hash: Hash512([0u8; 64]),
            schema,
            payload: Bytes::from(vec![0u8; MAX_PAYLOAD_LEN + 1]),
            witness,
        };

        let err = encode_frame(&entry);
        assert!(matches!(err, Err(StorageError::PayloadTooLarge { .. })));
    }
}
