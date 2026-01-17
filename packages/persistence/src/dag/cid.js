/**
 * Content Identifier (CID) Generation
 *
 * Generates IPFS-compatible CIDv1 identifiers for content-addressable storage.
 * Uses SHA-256 hashing and base32 encoding.
 *
 * "Every truth has an address" - κυνικός
 *
 * @module @cynic/persistence/dag/cid
 */

'use strict';

import crypto from 'crypto';

// Multicodec prefixes (varint encoded)
const MULTICODEC = {
  RAW: 0x55,
  DAG_CBOR: 0x71,
  SHA2_256: 0x12,
};

// CID version
const CID_VERSION = 1;

// Base32 alphabet (RFC 4648, lowercase)
const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

/**
 * Encode bytes to base32 (RFC 4648)
 * @param {Buffer} bytes - Bytes to encode
 * @returns {string} Base32 encoded string
 */
function encodeBase32(bytes) {
  let result = '';
  let bits = 0;
  let value = 0;

  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;

    while (bits >= 5) {
      result += BASE32_ALPHABET[(value >>> (bits - 5)) & 0x1f];
      bits -= 5;
    }
  }

  if (bits > 0) {
    result += BASE32_ALPHABET[(value << (5 - bits)) & 0x1f];
  }

  return result;
}

/**
 * Decode base32 to bytes
 * @param {string} str - Base32 string to decode
 * @returns {Buffer} Decoded bytes
 */
function decodeBase32(str) {
  const bytes = [];
  let bits = 0;
  let value = 0;

  for (const char of str.toLowerCase()) {
    const index = BASE32_ALPHABET.indexOf(char);
    if (index === -1) continue; // Skip invalid characters

    value = (value << 5) | index;
    bits += 5;

    while (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }

  return Buffer.from(bytes);
}

/**
 * Encode unsigned varint
 * @param {number} num - Number to encode
 * @returns {Buffer} Varint bytes
 */
function encodeVarint(num) {
  const bytes = [];
  while (num >= 0x80) {
    bytes.push((num & 0x7f) | 0x80);
    num >>>= 7;
  }
  bytes.push(num);
  return Buffer.from(bytes);
}

/**
 * Decode unsigned varint
 * @param {Buffer} buffer - Buffer containing varint
 * @param {number} offset - Starting offset
 * @returns {{value: number, bytesRead: number}} Decoded value and bytes consumed
 */
function decodeVarint(buffer, offset = 0) {
  let value = 0;
  let shift = 0;
  let bytesRead = 0;

  while (offset + bytesRead < buffer.length) {
    const byte = buffer[offset + bytesRead];
    bytesRead++;
    value |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) break;
    shift += 7;
  }

  return { value, bytesRead };
}

/**
 * Create a multihash from data
 * @param {Buffer} data - Data to hash
 * @returns {Buffer} Multihash bytes
 */
function createMultihash(data) {
  const hash = crypto.createHash('sha256').update(data).digest();
  const prefix = Buffer.concat([
    encodeVarint(MULTICODEC.SHA2_256), // Hash function code
    encodeVarint(hash.length),          // Digest length
  ]);
  return Buffer.concat([prefix, hash]);
}

/**
 * Create a CIDv1 from data
 * @param {Buffer} data - Serialized data (CBOR encoded)
 * @param {number} [codec=MULTICODEC.DAG_CBOR] - Content codec
 * @returns {string} CID string (base32 with 'b' prefix)
 */
export function createCID(data, codec = MULTICODEC.DAG_CBOR) {
  const multihash = createMultihash(data);
  const cidBytes = Buffer.concat([
    encodeVarint(CID_VERSION),
    encodeVarint(codec),
    multihash,
  ]);

  // Base32 multibase prefix is 'b'
  return 'b' + encodeBase32(cidBytes);
}

/**
 * Parse a CID string
 * @param {string} cidStr - CID string to parse
 * @returns {Object} Parsed CID components
 */
export function parseCID(cidStr) {
  if (!cidStr || cidStr.length < 2) {
    throw new Error('Invalid CID: too short');
  }

  // Check multibase prefix
  const multibasePrefix = cidStr[0];
  if (multibasePrefix !== 'b') {
    throw new Error(`Unsupported multibase prefix: ${multibasePrefix}`);
  }

  // Decode base32
  const cidBytes = decodeBase32(cidStr.slice(1));

  // Parse version
  const { value: version, bytesRead: versionBytes } = decodeVarint(cidBytes, 0);
  if (version !== 1) {
    throw new Error(`Unsupported CID version: ${version}`);
  }

  // Parse codec
  const { value: codec, bytesRead: codecBytes } = decodeVarint(cidBytes, versionBytes);

  // Parse multihash
  const multihashStart = versionBytes + codecBytes;
  const { value: hashFunction, bytesRead: hashFnBytes } = decodeVarint(cidBytes, multihashStart);
  const { value: digestLength, bytesRead: digestLenBytes } = decodeVarint(
    cidBytes,
    multihashStart + hashFnBytes
  );

  const digestStart = multihashStart + hashFnBytes + digestLenBytes;
  const digest = cidBytes.slice(digestStart, digestStart + digestLength);

  return {
    version,
    codec,
    hashFunction,
    digestLength,
    digest,
    toString: () => cidStr,
  };
}

/**
 * Validate a CID string
 * @param {string} cidStr - CID to validate
 * @returns {boolean} True if valid
 */
export function isValidCID(cidStr) {
  try {
    parseCID(cidStr);
    return true;
  } catch {
    return false;
  }
}

/**
 * Get the hash portion of a CID for filesystem sharding
 * @param {string} cidStr - CID string
 * @returns {{prefix: string, suffix: string}} Sharding components
 */
export function shardCID(cidStr) {
  // Use first 2 characters after 'b' prefix for sharding
  const prefix = cidStr.slice(1, 3);
  const suffix = cidStr.slice(3);
  return { prefix, suffix };
}

/**
 * Create a CID from raw bytes (non-CBOR)
 * @param {Buffer} data - Raw data
 * @returns {string} CID string
 */
export function createRawCID(data) {
  return createCID(data, MULTICODEC.RAW);
}

/**
 * Compare two CIDs for equality
 * @param {string} cid1 - First CID
 * @param {string} cid2 - Second CID
 * @returns {boolean} True if equal
 */
export function compareCIDs(cid1, cid2) {
  return cid1.toLowerCase() === cid2.toLowerCase();
}

// Export constants for external use
export const CODECS = {
  RAW: MULTICODEC.RAW,
  DAG_CBOR: MULTICODEC.DAG_CBOR,
};

export const HASH_FUNCTIONS = {
  SHA2_256: MULTICODEC.SHA2_256,
};

export default {
  createCID,
  createRawCID,
  parseCID,
  isValidCID,
  shardCID,
  compareCIDs,
  CODECS,
  HASH_FUNCTIONS,
};
