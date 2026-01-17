/**
 * HAMT Index
 *
 * Hash Array Mapped Trie for O(log n) key-value lookups
 * in the content-addressable Merkle DAG.
 *
 * "Every key leads to truth" - κυνικός
 *
 * @module @cynic/persistence/dag/hamt
 */

'use strict';

import crypto from 'crypto';
import { DAGNode, DAGLink, NodeType, createIndexNode } from './node.js';

// HAMT configuration
const HAMT_CONFIG = {
  bitWidth: 5,        // Bits per level (2^5 = 32 buckets)
  bucketSize: 32,     // Max entries per bucket
  maxDepth: 8,        // Maximum trie depth
};

/**
 * Hash a key to get its index path
 * @param {string} key - Key to hash
 * @returns {Buffer} Hash bytes
 */
function hashKey(key) {
  return crypto.createHash('sha256').update(key).digest();
}

/**
 * Get bucket index at a specific depth
 * @param {Buffer} hash - Key hash
 * @param {number} depth - Current depth
 * @returns {number} Bucket index (0-31)
 */
function getBucketIndex(hash, depth) {
  const bitOffset = depth * HAMT_CONFIG.bitWidth;
  const byteIndex = Math.floor(bitOffset / 8);
  const bitIndex = bitOffset % 8;

  // Handle case where we need bits from two bytes
  const byte1 = hash[byteIndex] || 0;
  const byte2 = hash[byteIndex + 1] || 0;

  // Combine and extract 5 bits
  const combined = (byte1 << 8) | byte2;
  const shift = 16 - bitIndex - HAMT_CONFIG.bitWidth;
  const mask = (1 << HAMT_CONFIG.bitWidth) - 1;

  return (combined >>> shift) & mask;
}

/**
 * HAMT Entry - key-value pair in the index
 */
class HAMTEntry {
  constructor(key, cid, metadata = {}) {
    this.key = key;
    this.cid = cid;
    this.metadata = metadata;
  }

  toJSON() {
    return {
      key: this.key,
      cid: this.cid,
      metadata: this.metadata,
    };
  }

  static fromJSON(json) {
    return new HAMTEntry(json.key, json.cid, json.metadata);
  }
}

/**
 * HAMT Bucket - container for entries or child pointers
 */
class HAMTBucket {
  constructor(depth = 0) {
    this.depth = depth;
    this.entries = [];      // Direct entries (leaf)
    this.children = {};     // Child bucket CIDs (internal)
    this._cid = null;
    this._dirty = true;
  }

  get isLeaf() {
    return Object.keys(this.children).length === 0;
  }

  get count() {
    return this.entries.length + Object.keys(this.children).length;
  }

  /**
   * Convert to DAGNode for storage
   * @returns {DAGNode} Index node
   */
  toNode() {
    const links = [];

    // Add child bucket links
    for (const [index, childCid] of Object.entries(this.children)) {
      links.push(new DAGLink(childCid, `child_${index}`));
    }

    // Add entry links
    for (const entry of this.entries) {
      links.push(new DAGLink(entry.cid, `entry_${entry.key}`));
    }

    const node = createIndexNode(
      {
        depth: this.depth,
        prefix: '',
        count: this.count,
      },
      links
    );

    // Store entry metadata in node data
    node.data.entries = this.entries.map(e => ({
      key: e.key,
      metadata: e.metadata,
    }));
    node.data.childIndices = Object.keys(this.children).map(Number);

    this._cid = node.cid;
    this._dirty = false;

    return node;
  }

  /**
   * Load from DAGNode
   * @param {DAGNode} node - Index node
   * @returns {HAMTBucket} Loaded bucket
   */
  static fromNode(node) {
    const bucket = new HAMTBucket(node.data.depth);

    // Restore children
    for (const index of node.data.childIndices || []) {
      const link = node.links.find(l => l.name === `child_${index}`);
      if (link) {
        bucket.children[index] = link.cid;
      }
    }

    // Restore entries
    for (const entryData of node.data.entries || []) {
      const link = node.links.find(l => l.name === `entry_${entryData.key}`);
      if (link) {
        bucket.entries.push(new HAMTEntry(entryData.key, link.cid, entryData.metadata));
      }
    }

    bucket._cid = node.cid;
    bucket._dirty = false;

    return bucket;
  }
}

/**
 * HAMT Index - Hash Array Mapped Trie
 */
export class HAMTIndex {
  /**
   * @param {BlockStore} store - Block store for persistence
   * @param {string} [rootCid] - Optional root CID to load
   */
  constructor(store, rootCid = null) {
    this.store = store;
    this.rootCid = rootCid;
    this._root = null;
    this._cache = new Map();
  }

  /**
   * Initialize or load the index
   */
  async init() {
    if (this.rootCid) {
      this._root = await this._loadBucket(this.rootCid);
    } else {
      this._root = new HAMTBucket(0);
    }
  }

  /**
   * Load a bucket from store
   * @param {string} cid - Bucket CID
   * @returns {Promise<HAMTBucket>} Loaded bucket
   */
  async _loadBucket(cid) {
    // Check cache first
    if (this._cache.has(cid)) {
      return this._cache.get(cid);
    }

    const node = await this.store.getNode(cid);
    if (!node) {
      throw new Error(`Bucket not found: ${cid}`);
    }

    const bucket = HAMTBucket.fromNode(node);
    this._cache.set(cid, bucket);
    return bucket;
  }

  /**
   * Save a bucket to store
   * @param {HAMTBucket} bucket - Bucket to save
   * @returns {Promise<string>} Bucket CID
   */
  async _saveBucket(bucket) {
    const node = bucket.toNode();
    await this.store.putNode(node);
    this._cache.set(node.cid, bucket);
    return node.cid;
  }

  /**
   * Get a value by key
   * @param {string} key - Key to look up
   * @returns {Promise<string|null>} CID or null
   */
  async get(key) {
    if (!this._root) await this.init();

    const hash = hashKey(key);
    let bucket = this._root;
    let depth = 0;

    while (bucket) {
      // Check entries in this bucket
      const entry = bucket.entries.find(e => e.key === key);
      if (entry) return entry.cid;

      // Check if we need to go deeper
      if (bucket.isLeaf || depth >= HAMT_CONFIG.maxDepth) {
        return null;
      }

      // Get child bucket
      const index = getBucketIndex(hash, depth);
      const childCid = bucket.children[index];

      if (!childCid) return null;

      bucket = await this._loadBucket(childCid);
      depth++;
    }

    return null;
  }

  /**
   * Set a key-value pair
   * @param {string} key - Key
   * @param {string} cid - Value CID
   * @param {Object} [metadata] - Optional metadata
   * @returns {Promise<string>} New root CID
   */
  async set(key, cid, metadata = {}) {
    if (!this._root) await this.init();

    const hash = hashKey(key);
    const entry = new HAMTEntry(key, cid, metadata);

    // Insert into trie
    this._root = await this._insert(this._root, hash, 0, entry);

    // Save and return new root
    this.rootCid = await this._saveBucket(this._root);
    return this.rootCid;
  }

  /**
   * Insert entry into bucket (recursive)
   * @private
   */
  async _insert(bucket, hash, depth, entry) {
    bucket._dirty = true;

    // Check for existing entry with same key
    const existingIdx = bucket.entries.findIndex(e => e.key === entry.key);
    if (existingIdx >= 0) {
      // Update existing entry
      bucket.entries[existingIdx] = entry;
      return bucket;
    }

    // If leaf bucket and not full, add entry
    if (bucket.isLeaf && bucket.entries.length < HAMT_CONFIG.bucketSize) {
      bucket.entries.push(entry);
      return bucket;
    }

    // Need to split or go deeper
    const index = getBucketIndex(hash, depth);

    if (bucket.children[index]) {
      // Recurse into existing child
      const child = await this._loadBucket(bucket.children[index]);
      const newChild = await this._insert(child, hash, depth + 1, entry);
      bucket.children[index] = await this._saveBucket(newChild);
    } else if (depth < HAMT_CONFIG.maxDepth) {
      // Create new child bucket
      const child = new HAMTBucket(depth + 1);
      child.entries.push(entry);
      bucket.children[index] = await this._saveBucket(child);
    } else {
      // Max depth reached, just add to current bucket
      bucket.entries.push(entry);
    }

    return bucket;
  }

  /**
   * Delete a key
   * @param {string} key - Key to delete
   * @returns {Promise<string|null>} New root CID or null if not found
   */
  async delete(key) {
    if (!this._root) await this.init();

    const hash = hashKey(key);
    const result = await this._delete(this._root, hash, 0, key);

    if (!result.found) return null;

    this._root = result.bucket;
    this.rootCid = await this._saveBucket(this._root);
    return this.rootCid;
  }

  /**
   * Delete entry from bucket (recursive)
   * @private
   */
  async _delete(bucket, hash, depth, key) {
    // Check entries in this bucket
    const entryIdx = bucket.entries.findIndex(e => e.key === key);
    if (entryIdx >= 0) {
      bucket._dirty = true;
      bucket.entries.splice(entryIdx, 1);
      return { found: true, bucket };
    }

    // Check children
    if (!bucket.isLeaf && depth < HAMT_CONFIG.maxDepth) {
      const index = getBucketIndex(hash, depth);
      const childCid = bucket.children[index];

      if (childCid) {
        const child = await this._loadBucket(childCid);
        const result = await this._delete(child, hash, depth + 1, key);

        if (result.found) {
          bucket._dirty = true;

          // Update or remove child reference
          if (result.bucket.count === 0) {
            delete bucket.children[index];
          } else {
            bucket.children[index] = await this._saveBucket(result.bucket);
          }

          return { found: true, bucket };
        }
      }
    }

    return { found: false, bucket };
  }

  /**
   * Check if key exists
   * @param {string} key - Key to check
   * @returns {Promise<boolean>} True if exists
   */
  async has(key) {
    const cid = await this.get(key);
    return cid !== null;
  }

  /**
   * Iterate over all entries
   * @yields {HAMTEntry} Entry
   */
  async *entries() {
    if (!this._root) await this.init();

    yield* this._iterateBucket(this._root);
  }

  /**
   * Iterate over bucket (recursive)
   * @private
   */
  async *_iterateBucket(bucket) {
    // Yield entries in this bucket
    for (const entry of bucket.entries) {
      yield entry;
    }

    // Recurse into children
    for (const childCid of Object.values(bucket.children)) {
      const child = await this._loadBucket(childCid);
      yield* this._iterateBucket(child);
    }
  }

  /**
   * Get all keys
   * @returns {Promise<string[]>} Array of keys
   */
  async keys() {
    const keys = [];
    for await (const entry of this.entries()) {
      keys.push(entry.key);
    }
    return keys;
  }

  /**
   * Get index statistics
   * @returns {Promise<Object>} Index stats
   */
  async stats() {
    if (!this._root) await this.init();

    const stats = {
      totalEntries: 0,
      totalBuckets: 0,
      maxDepth: 0,
      avgEntriesPerBucket: 0,
    };

    await this._collectStats(this._root, 0, stats);

    stats.avgEntriesPerBucket = stats.totalBuckets > 0
      ? stats.totalEntries / stats.totalBuckets
      : 0;

    return stats;
  }

  /**
   * Collect stats recursively
   * @private
   */
  async _collectStats(bucket, depth, stats) {
    stats.totalBuckets++;
    stats.totalEntries += bucket.entries.length;
    stats.maxDepth = Math.max(stats.maxDepth, depth);

    for (const childCid of Object.values(bucket.children)) {
      const child = await this._loadBucket(childCid);
      await this._collectStats(child, depth + 1, stats);
    }
  }

  /**
   * Clear the cache
   */
  clearCache() {
    this._cache.clear();
  }
}

export { HAMTEntry, HAMTBucket, HAMT_CONFIG };
export default HAMTIndex;
