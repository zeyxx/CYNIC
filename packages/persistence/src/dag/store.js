/**
 * Block Store
 *
 * Filesystem-based content-addressable block storage.
 * Uses 2-character prefix sharding for scalability.
 *
 * "Every block finds its place" - κυνικός
 *
 * @module @cynic/persistence/dag/store
 */

'use strict';

import fs from 'fs/promises';
import path from 'path';
import { shardCID, isValidCID } from './cid.js';
import { DAGNode } from './node.js';

// Default configuration
const DEFAULT_CONFIG = {
  basePath: './data/blocks',
  shardDepth: 2,    // Number of characters for sharding prefix
  fileExtension: '.block',
};

/**
 * Filesystem Block Store
 *
 * Stores blocks in a sharded directory structure:
 * basePath/
 *   ab/
 *     cdef123...block
 *   xy/
 *     z789abc...block
 */
export class BlockStore {
  /**
   * @param {Object} [config] - Store configuration
   */
  constructor(config = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this._initialized = false;
  }

  /**
   * Initialize the store (create base directory)
   */
  async init() {
    if (this._initialized) return;

    await fs.mkdir(this.config.basePath, { recursive: true });
    this._initialized = true;
  }

  /**
   * Get the filesystem path for a CID
   * @param {string} cid - Content identifier
   * @returns {string} File path
   */
  _getPath(cid) {
    const { prefix, suffix } = shardCID(cid);
    return path.join(
      this.config.basePath,
      prefix,
      suffix + this.config.fileExtension
    );
  }

  /**
   * Get the shard directory for a CID
   * @param {string} cid - Content identifier
   * @returns {string} Directory path
   */
  _getShardDir(cid) {
    const { prefix } = shardCID(cid);
    return path.join(this.config.basePath, prefix);
  }

  /**
   * Check if a block exists
   * @param {string} cid - Content identifier
   * @returns {Promise<boolean>} True if exists
   */
  async has(cid) {
    if (!isValidCID(cid)) return false;

    try {
      await fs.access(this._getPath(cid));
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get a block by CID
   * @param {string} cid - Content identifier
   * @returns {Promise<Buffer|null>} Block data or null
   */
  async get(cid) {
    if (!isValidCID(cid)) return null;

    try {
      const data = await fs.readFile(this._getPath(cid));
      return data;
    } catch (err) {
      if (err.code === 'ENOENT') return null;
      throw err;
    }
  }

  /**
   * Get a block and decode as DAGNode
   * @param {string} cid - Content identifier
   * @returns {Promise<DAGNode|null>} DAGNode or null
   */
  async getNode(cid) {
    const data = await this.get(cid);
    if (!data) return null;

    return DAGNode.fromBytes(cid, data);
  }

  /**
   * Put a block
   * @param {string} cid - Content identifier
   * @param {Buffer} data - Block data
   * @returns {Promise<void>}
   */
  async put(cid, data) {
    await this.init();

    if (!isValidCID(cid)) {
      throw new Error('Invalid CID');
    }

    // Ensure shard directory exists
    const shardDir = this._getShardDir(cid);
    await fs.mkdir(shardDir, { recursive: true });

    // Write block atomically (write to temp, then rename)
    const filePath = this._getPath(cid);
    const tempPath = filePath + '.tmp';

    await fs.writeFile(tempPath, data);
    await fs.rename(tempPath, filePath);
  }

  /**
   * Put a DAGNode
   * @param {DAGNode} node - Node to store
   * @returns {Promise<string>} CID of stored node
   */
  async putNode(node) {
    const cid = node.cid;
    const data = node.encode();
    await this.put(cid, data);
    return cid;
  }

  /**
   * Delete a block
   * @param {string} cid - Content identifier
   * @returns {Promise<boolean>} True if deleted
   */
  async delete(cid) {
    if (!isValidCID(cid)) return false;

    try {
      await fs.unlink(this._getPath(cid));
      return true;
    } catch (err) {
      if (err.code === 'ENOENT') return false;
      throw err;
    }
  }

  /**
   * Get multiple blocks
   * @param {string[]} cids - Content identifiers
   * @returns {Promise<Map<string, Buffer>>} Map of CID to data
   */
  async getMany(cids) {
    const results = new Map();

    await Promise.all(
      cids.map(async cid => {
        const data = await this.get(cid);
        if (data) results.set(cid, data);
      })
    );

    return results;
  }

  /**
   * Put multiple blocks
   * @param {Map<string, Buffer>} blocks - Map of CID to data
   * @returns {Promise<void>}
   */
  async putMany(blocks) {
    await Promise.all(
      Array.from(blocks.entries()).map(([cid, data]) => this.put(cid, data))
    );
  }

  /**
   * List all CIDs in a shard
   * @param {string} prefix - Shard prefix
   * @returns {Promise<string[]>} CIDs in shard
   */
  async listShard(prefix) {
    const shardDir = path.join(this.config.basePath, prefix);

    try {
      const files = await fs.readdir(shardDir);
      return files
        .filter(f => f.endsWith(this.config.fileExtension))
        .map(f => 'b' + prefix + f.slice(0, -this.config.fileExtension.length));
    } catch (err) {
      if (err.code === 'ENOENT') return [];
      throw err;
    }
  }

  /**
   * List all shards
   * @returns {Promise<string[]>} Shard prefixes
   */
  async listShards() {
    try {
      const entries = await fs.readdir(this.config.basePath, { withFileTypes: true });
      return entries
        .filter(e => e.isDirectory() && e.name.length === this.config.shardDepth)
        .map(e => e.name);
    } catch (err) {
      if (err.code === 'ENOENT') return [];
      throw err;
    }
  }

  /**
   * Iterate over all blocks
   * @yields {Object} { cid, data }
   */
  async *iterate() {
    const shards = await this.listShards();

    for (const shard of shards) {
      const cids = await this.listShard(shard);

      for (const cid of cids) {
        const data = await this.get(cid);
        if (data) yield { cid, data };
      }
    }
  }

  /**
   * Get store statistics
   * @returns {Promise<Object>} Store stats
   */
  async stats() {
    const shards = await this.listShards();

    let totalBlocks = 0;
    let totalBytes = 0;

    for (const shard of shards) {
      const shardDir = path.join(this.config.basePath, shard);
      const files = await fs.readdir(shardDir);

      for (const file of files) {
        if (!file.endsWith(this.config.fileExtension)) continue;

        totalBlocks++;
        const stat = await fs.stat(path.join(shardDir, file));
        totalBytes += stat.size;
      }
    }

    return {
      totalBlocks,
      totalBytes,
      shardCount: shards.length,
      avgBlockSize: totalBlocks > 0 ? Math.round(totalBytes / totalBlocks) : 0,
    };
  }

  /**
   * Garbage collect orphaned blocks
   * @param {Set<string>} reachableCIDs - Set of reachable CIDs
   * @returns {Promise<Object>} GC results
   */
  async gc(reachableCIDs) {
    const deleted = [];

    for await (const { cid } of this.iterate()) {
      if (!reachableCIDs.has(cid)) {
        await this.delete(cid);
        deleted.push(cid);
      }
    }

    return {
      deletedCount: deleted.length,
      deleted,
    };
  }

  /**
   * Verify integrity of all blocks
   * @returns {Promise<Object>} Verification results
   */
  async verify() {
    const results = {
      total: 0,
      valid: 0,
      invalid: [],
    };

    for await (const { cid, data } of this.iterate()) {
      results.total++;

      try {
        const node = DAGNode.decode(data);
        if (node.cid === cid) {
          results.valid++;
        } else {
          results.invalid.push({ cid, error: 'CID mismatch' });
        }
      } catch (err) {
        results.invalid.push({ cid, error: err.message });
      }
    }

    return results;
  }

  /**
   * Export blocks to a CAR file format (simplified)
   * @param {string[]} roots - Root CIDs
   * @param {string} outputPath - Output file path
   * @returns {Promise<Object>} Export stats
   */
  async export(roots, outputPath) {
    const blocks = [];
    const visited = new Set();

    // BFS to collect all reachable blocks
    const queue = [...roots];

    while (queue.length > 0) {
      const cid = queue.shift();
      if (visited.has(cid)) continue;
      visited.add(cid);

      const data = await this.get(cid);
      if (!data) continue;

      blocks.push({ cid, data });

      // Parse node to find links
      try {
        const node = DAGNode.decode(data);
        for (const link of node.links) {
          if (!visited.has(link.cid)) {
            queue.push(link.cid);
          }
        }
      } catch {
        // Ignore decode errors for raw blocks
      }
    }

    // Write simple format (newline-delimited JSON)
    const output = {
      roots,
      blocks: blocks.map(b => ({
        cid: b.cid,
        data: b.data.toString('base64'),
      })),
    };

    await fs.writeFile(outputPath, JSON.stringify(output, null, 2));

    return {
      rootCount: roots.length,
      blockCount: blocks.length,
      totalBytes: blocks.reduce((sum, b) => sum + b.data.length, 0),
    };
  }

  /**
   * Import blocks from exported format
   * @param {string} inputPath - Input file path
   * @returns {Promise<Object>} Import stats
   */
  async import(inputPath) {
    const content = await fs.readFile(inputPath, 'utf-8');
    const { roots, blocks } = JSON.parse(content);

    let imported = 0;
    let skipped = 0;

    for (const block of blocks) {
      const exists = await this.has(block.cid);
      if (exists) {
        skipped++;
        continue;
      }

      const data = Buffer.from(block.data, 'base64');
      await this.put(block.cid, data);
      imported++;
    }

    return {
      roots,
      imported,
      skipped,
      total: blocks.length,
    };
  }
}

export default BlockStore;
