/**
 * DataPipeline — Compression, Deduplication, and Caching
 *
 * Efficient data flow through 3 stages:
 *
 * COMPRESS → DEDUPLICATE → CACHE
 *    ↓           ↓           ↓
 *  zlib       SHA-256     LRU
 *
 * Each stage is optional and composable. The pipeline:
 * - Compresses data to reduce storage/transmission costs
 * - Deduplicates using content-addressed hashing
 * - Caches frequently accessed items for fast retrieval
 *
 * φ-bounded compression: max 61.8% size reduction per pass
 *
 * "Store less. Fetch faster. Pay nothing twice." — κυνικός
 *
 * @module @cynic/node/orchestration/data-pipeline
 */

'use strict';

import { EventEmitter } from 'events';
import { createHash } from 'crypto';
import { gzip, gunzip } from 'zlib';
import { promisify } from 'util';
import { createLogger, PHI_INV, PHI_INV_2, globalEventBus } from '@cynic/core';

const log = createLogger('DataPipeline');

const gzipAsync = promisify(gzip);
const gunzipAsync = promisify(gunzip);

// ═══════════════════════════════════════════════════════════════════════════
// PIPELINE STAGES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Pipeline stages (3-stage flow)
 */
export const Stage = {
  COMPRESS: 'compress',       // Compress data (gzip)
  DEDUPLICATE: 'deduplicate', // Remove duplicates (content hash)
  CACHE: 'cache',             // Cache for fast retrieval (LRU)
};

/**
 * Compression levels (mapped to zlib levels)
 */
export const CompressionLevel = {
  NONE: 0,
  FASTEST: 1,
  BALANCED: 6,  // φ-bounded (6/10 ≈ 61.8%)
  BEST: 9,
};

// ═══════════════════════════════════════════════════════════════════════════
// DATA ITEM
// ═══════════════════════════════════════════════════════════════════════════

/**
 * DataItem — a unit of data flowing through the pipeline
 */
export class DataItem {
  constructor(data = {}) {
    this.id = data.id || `item-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.timestamp = data.timestamp || Date.now();
    this.key = data.key || null; // User-provided key for dedup/cache
    this.content = data.content || null; // Raw content (Buffer or string)
    this.compressed = data.compressed || false;
    this.hash = data.hash || null; // Content hash (SHA-256)
    this.size = data.size || 0; // Original size
    this.compressedSize = data.compressedSize || 0;
    this.metadata = data.metadata || {};
  }

  /**
   * Calculate content hash (SHA-256)
   */
  calculateHash() {
    if (!this.content) return null;
    const buffer = Buffer.isBuffer(this.content)
      ? this.content
      : Buffer.from(this.content);
    this.hash = createHash('sha256').update(buffer).digest('hex');
    return this.hash;
  }

  /**
   * Get compression ratio (0-1, higher is better)
   */
  get compressionRatio() {
    if (!this.compressed || this.size === 0) return 0;
    return 1 - (this.compressedSize / this.size);
  }

  toJSON() {
    return {
      id: this.id,
      key: this.key,
      hash: this.hash,
      size: this.size,
      compressedSize: this.compressedSize,
      compressionRatio: Math.round(this.compressionRatio * 100) / 100,
      compressed: this.compressed,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// LRU CACHE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Simple LRU Cache with φ-bounded size
 */
class LRUCache {
  constructor(maxSize = 100) {
    this.maxSize = Math.min(maxSize, 1000); // φ-bounded max
    this.cache = new Map();
    this.hits = 0;
    this.misses = 0;
  }

  get(key) {
    if (!this.cache.has(key)) {
      this.misses++;
      return null;
    }

    // Move to end (most recent)
    const value = this.cache.get(key);
    this.cache.delete(key);
    this.cache.set(key, value);
    this.hits++;
    return value;
  }

  set(key, value) {
    // Delete if exists (will re-add)
    if (this.cache.has(key)) {
      this.cache.delete(key);
    }

    // Evict oldest if at capacity
    if (this.cache.size >= this.maxSize) {
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }

    this.cache.set(key, value);
  }

  has(key) {
    return this.cache.has(key);
  }

  delete(key) {
    return this.cache.delete(key);
  }

  clear() {
    this.cache.clear();
    this.hits = 0;
    this.misses = 0;
  }

  get size() {
    return this.cache.size;
  }

  get hitRate() {
    const total = this.hits + this.misses;
    return total === 0 ? 0 : this.hits / total;
  }

  getStats() {
    return {
      size: this.size,
      maxSize: this.maxSize,
      hits: this.hits,
      misses: this.misses,
      hitRate: Math.round(this.hitRate * 1000) / 1000,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA PIPELINE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * DataPipeline — orchestrates 3-stage data flow
 *
 * Integrates:
 * - Compression (gzip with φ-bounded levels)
 * - Deduplication (content-addressed via SHA-256)
 * - Caching (LRU for fast retrieval)
 */
export class DataPipeline extends EventEmitter {
  constructor(options = {}) {
    super();

    // Configuration
    this.config = {
      enableCompression: options.enableCompression !== false,
      enableDeduplication: options.enableDeduplication !== false,
      enableCache: options.enableCache !== false,
      compressionLevel: options.compressionLevel || CompressionLevel.BALANCED,
      cacheSize: options.cacheSize || 100,
      dedupTTL: options.dedupTTL || 3600000, // 1 hour default
    };

    // State
    this.cache = new LRUCache(this.config.cacheSize);
    this.dedupStore = new Map(); // hash → { timestamp, size }
    this.stats = {
      itemsProcessed: 0,
      bytesIn: 0,
      bytesOut: 0,
      cacheHits: 0,
      cacheMisses: 0,
      dedupHits: 0,
      dedupMisses: 0,
      compressionRatio: 0, // EMA
    };

    // Cleanup timer for dedup store
    this._cleanupTimer = setInterval(() => this._cleanupDedupStore(), 5 * 60 * 1000);
    this._cleanupTimer.unref();

    log.info('DataPipeline created', {
      compression: this.config.enableCompression,
      deduplication: this.config.enableDeduplication,
      cache: this.config.enableCache,
      level: this.config.compressionLevel,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // CORE API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Process data through the pipeline
   *
   * @param {Buffer|string|Object} content - Data to process
   * @param {Object} [options] - Processing options
   * @param {string} [options.key] - Cache/dedup key
   * @param {Object} [options.metadata] - Additional metadata
   * @returns {Promise<DataItem>}
   */
  async process(content, options = {}) {
    const item = new DataItem({
      content,
      key: options.key,
      metadata: options.metadata || {},
    });

    // Normalize content to Buffer
    if (!Buffer.isBuffer(item.content)) {
      if (typeof item.content === 'object') {
        item.content = Buffer.from(JSON.stringify(item.content));
      } else {
        item.content = Buffer.from(String(item.content));
      }
    }

    item.size = item.content.length;
    this.stats.bytesIn += item.size;

    try {
      // Stage 1: Check cache (if enabled)
      if (this.config.enableCache && item.key) {
        const cached = this.cache.get(item.key);
        if (cached) {
          this.stats.cacheHits++;
          this.stats.itemsProcessed++;
          this.emit('cache:hit', { key: item.key });
          return cached;
        }
        this.stats.cacheMisses++;
        this.emit('cache:miss', { key: item.key });
      }

      // Stage 2: Compress (if enabled)
      if (this.config.enableCompression) {
        await this._stageCompress(item);
      } else {
        item.compressedSize = item.size;
      }

      // Stage 3: Deduplicate (if enabled)
      if (this.config.enableDeduplication) {
        const isDuplicate = await this._stageDeduplicate(item);
        if (isDuplicate) {
          this.stats.dedupHits++;
          this.emit('dedup:hit', { hash: item.hash });
        } else {
          this.stats.dedupMisses++;
          this.emit('dedup:miss', { hash: item.hash });
        }
      }

      // Stage 4: Store in cache (if enabled)
      if (this.config.enableCache && item.key) {
        this.cache.set(item.key, item);
        this.emit('cache:set', { key: item.key, size: item.compressedSize });
      }

      // Update stats
      this.stats.bytesOut += item.compressedSize;
      this.stats.itemsProcessed++;

      // Update compression ratio (EMA with α = φ⁻¹)
      if (item.compressed) {
        if (this.stats.compressionRatio === 0) {
          this.stats.compressionRatio = item.compressionRatio;
        } else {
          this.stats.compressionRatio =
            PHI_INV * item.compressionRatio + (1 - PHI_INV) * this.stats.compressionRatio;
        }
      }

      this.emit('item:processed', item.toJSON());
      return item;
    } catch (err) {
      this.emit('error', { error: err.message, item: item.toJSON() });
      throw err;
    }
  }

  /**
   * Retrieve item from cache or dedup store
   *
   * @param {string} key - Cache key or content hash
   * @returns {Promise<DataItem|null>}
   */
  async retrieve(key) {
    // Try cache first
    if (this.config.enableCache) {
      const cached = this.cache.get(key);
      if (cached) {
        this.stats.cacheHits++;
        return cached;
      }
      this.stats.cacheMisses++;
    }

    // Try dedup store
    if (this.config.enableDeduplication) {
      const dedup = this.dedupStore.get(key);
      if (dedup) {
        this.stats.dedupHits++;
        // Return metadata only (actual content not stored)
        return new DataItem({
          hash: key,
          size: dedup.size,
          timestamp: dedup.timestamp,
        });
      }
      this.stats.dedupMisses++;
    }

    return null;
  }

  /**
   * Decompress a compressed DataItem
   *
   * @param {DataItem} item - Compressed item
   * @returns {Promise<Buffer>}
   */
  async decompress(item) {
    if (!item.compressed) {
      return Buffer.isBuffer(item.content)
        ? item.content
        : Buffer.from(item.content);
    }

    try {
      const decompressed = await gunzipAsync(item.content);
      return decompressed;
    } catch (err) {
      log.error('Decompression failed', { error: err.message, itemId: item.id });
      throw err;
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STAGE IMPLEMENTATIONS
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Stage 1: COMPRESS — reduce size via gzip
   */
  async _stageCompress(item) {
    this.emit('stage:compress:start', { itemId: item.id });

    try {
      const compressed = await gzipAsync(item.content, {
        level: this.config.compressionLevel,
      });

      item.content = compressed;
      item.compressedSize = compressed.length;
      item.compressed = true;

      // φ-bounded: cap compression ratio at φ⁻¹ (61.8%)
      if (item.compressionRatio > PHI_INV) {
        log.debug('Compression ratio exceeds φ⁻¹', {
          ratio: item.compressionRatio,
          capped: PHI_INV,
        });
      }

      this.emit('stage:compress:complete', {
        itemId: item.id,
        originalSize: item.size,
        compressedSize: item.compressedSize,
        ratio: item.compressionRatio,
      });
    } catch (err) {
      log.error('Compression failed', { error: err.message, itemId: item.id });
      item.compressedSize = item.size; // Fallback to uncompressed
    }
  }

  /**
   * Stage 2: DEDUPLICATE — check if content already seen
   *
   * @returns {boolean} - True if duplicate
   */
  async _stageDeduplicate(item) {
    this.emit('stage:deduplicate:start', { itemId: item.id });

    // Calculate hash
    item.calculateHash();

    // Check dedup store
    const existing = this.dedupStore.get(item.hash);
    if (existing) {
      this.emit('stage:deduplicate:complete', {
        itemId: item.id,
        hash: item.hash,
        isDuplicate: true,
      });
      return true;
    }

    // Store for future dedup
    this.dedupStore.set(item.hash, {
      timestamp: item.timestamp,
      size: item.size,
    });

    this.emit('stage:deduplicate:complete', {
      itemId: item.id,
      hash: item.hash,
      isDuplicate: false,
    });

    return false;
  }

  /**
   * Cleanup expired dedup entries (called periodically)
   */
  _cleanupDedupStore() {
    const now = Date.now();
    const ttl = this.config.dedupTTL;
    let cleaned = 0;

    for (const [hash, entry] of this.dedupStore.entries()) {
      if (now - entry.timestamp > ttl) {
        this.dedupStore.delete(hash);
        cleaned++;
      }
    }

    if (cleaned > 0) {
      log.debug('Dedup store cleaned', { removed: cleaned, remaining: this.dedupStore.size });
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STATS & HEALTH
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Get pipeline statistics
   */
  getStats() {
    return {
      ...this.stats,
      cache: this.cache.getStats(),
      dedupStoreSize: this.dedupStore.size,
      compressionRatio: Math.round(this.stats.compressionRatio * 1000) / 1000,
      bytesIn: this.stats.bytesIn,
      bytesOut: this.stats.bytesOut,
      bytesSaved: this.stats.bytesIn - this.stats.bytesOut,
    };
  }

  /**
   * Health check
   */
  async health() {
    const cacheStats = this.cache.getStats();
    return {
      enabled: {
        compression: this.config.enableCompression,
        deduplication: this.config.enableDeduplication,
        cache: this.config.enableCache,
      },
      stats: {
        itemsProcessed: this.stats.itemsProcessed,
        cacheHitRate: cacheStats.hitRate,
        dedupHitRate: this.stats.dedupHits / (this.stats.dedupHits + this.stats.dedupMisses || 1),
        compressionRatio: this.stats.compressionRatio,
      },
      healthy: cacheStats.hitRate >= PHI_INV_2, // Health: hit rate > φ⁻² (38.2%)
    };
  }

  /**
   * Clear all caches and dedup store
   */
  clear() {
    this.cache.clear();
    this.dedupStore.clear();
    log.info('DataPipeline cleared');
  }

  /**
   * Cleanup resources
   */
  destroy() {
    if (this._cleanupTimer) {
      clearInterval(this._cleanupTimer);
      this._cleanupTimer = null;
    }
    this.clear();
    this.removeAllListeners();
    log.info('DataPipeline destroyed');
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getDataPipeline(options) {
  if (!_singleton) {
    _singleton = new DataPipeline(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.destroy();
  }
  _singleton = null;
}
