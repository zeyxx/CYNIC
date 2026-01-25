/**
 * Library Cache Repository
 *
 * PostgreSQL persistence for Context7 documentation cache.
 * Durable storage with TTL management.
 *
 * "The dog remembers the scent" - κυνικός
 *
 * @module @cynic/persistence/repositories/library-cache
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

// Default TTL: 24 hours
const DEFAULT_TTL_HOURS = 24;

/**
 * Hash a query string for cache key
 */
function hashQuery(query) {
  return crypto.createHash('sha256')
    .update(query.toLowerCase().trim())
    .digest('hex');
}

/**
 * @extends BaseRepository
 */
export class LibraryCacheRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Get cached documentation
   * @param {string} libraryId - Library ID (e.g., '/vercel/next.js')
   * @param {string} query - Search query
   * @returns {Promise<Object|null>} Cached content or null
   */
  async get(libraryId, query) {
    const queryHash = hashQuery(query);

    const { rows } = await this.db.query(`
      UPDATE library_cache
      SET hit_count = hit_count + 1, last_hit_at = NOW()
      WHERE library_id = $1 AND query_hash = $2 AND expires_at > NOW()
      RETURNING content, metadata, hit_count, created_at, expires_at
    `, [libraryId, queryHash]);

    if (rows[0]) {
      return {
        content: rows[0].content,
        metadata: rows[0].metadata,
        hitCount: rows[0].hit_count,
        createdAt: rows[0].created_at,
        expiresAt: rows[0].expires_at,
        cached: true,
      };
    }

    return null;
  }

  /**
   * Store documentation in cache
   * @param {string} libraryId - Library ID
   * @param {string} query - Search query
   * @param {string} content - Documentation content
   * @param {Object} [metadata] - Additional metadata
   * @param {number} [ttlHours] - TTL in hours
   * @returns {Promise<Object>} Stored cache entry
   */
  async set(libraryId, query, content, metadata = {}, ttlHours = DEFAULT_TTL_HOURS) {
    const queryHash = hashQuery(query);

    const { rows } = await this.db.query(`
      INSERT INTO library_cache (library_id, query_hash, content, metadata, expires_at)
      VALUES ($1, $2, $3, $4, NOW() + INTERVAL '${ttlHours} hours')
      ON CONFLICT (library_id, query_hash)
      DO UPDATE SET
        content = EXCLUDED.content,
        metadata = EXCLUDED.metadata,
        expires_at = NOW() + INTERVAL '${ttlHours} hours',
        hit_count = 0,
        created_at = NOW()
      RETURNING *
    `, [libraryId, queryHash, content, JSON.stringify(metadata)]);

    return rows[0];
  }

  /**
   * Get all cached entries for a library
   * @param {string} libraryId - Library ID
   * @returns {Promise<Object[]>} Cached entries
   */
  async getByLibrary(libraryId) {
    const { rows } = await this.db.query(`
      SELECT library_id, query_hash, metadata, hit_count, created_at, expires_at
      FROM library_cache
      WHERE library_id = $1 AND expires_at > NOW()
      ORDER BY hit_count DESC
    `, [libraryId]);

    return rows;
  }

  /**
   * Delete expired entries
   * @returns {Promise<number>} Number of deleted entries
   */
  async cleanExpired() {
    const { rowCount } = await this.db.query(`
      DELETE FROM library_cache WHERE expires_at <= NOW()
    `);

    return rowCount;
  }

  /**
   * Delete all entries for a library
   * @param {string} libraryId - Library ID
   * @returns {Promise<number>} Number of deleted entries
   */
  async invalidate(libraryId) {
    const { rowCount } = await this.db.query(`
      DELETE FROM library_cache WHERE library_id = $1
    `, [libraryId]);

    return rowCount;
  }

  /**
   * Get cache statistics
   * @returns {Promise<Object>} Cache statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_entries,
        COUNT(*) FILTER (WHERE expires_at > NOW()) as active_entries,
        COUNT(*) FILTER (WHERE expires_at <= NOW()) as expired_entries,
        COUNT(DISTINCT library_id) as unique_libraries,
        SUM(hit_count) as total_hits,
        AVG(hit_count) as avg_hits_per_entry,
        MIN(created_at) as oldest_entry,
        MAX(created_at) as newest_entry,
        SUM(LENGTH(content)) as total_content_bytes
      FROM library_cache
    `);

    const stats = rows[0];
    return {
      totalEntries: parseInt(stats.total_entries) || 0,
      activeEntries: parseInt(stats.active_entries) || 0,
      expiredEntries: parseInt(stats.expired_entries) || 0,
      uniqueLibraries: parseInt(stats.unique_libraries) || 0,
      totalHits: parseInt(stats.total_hits) || 0,
      avgHitsPerEntry: parseFloat(stats.avg_hits_per_entry) || 0,
      oldestEntry: stats.oldest_entry,
      newestEntry: stats.newest_entry,
      totalContentBytes: parseInt(stats.total_content_bytes) || 0,
    };
  }

  /**
   * Get top cached libraries by hit count
   * @param {number} [limit=10] - Number of libraries
   * @returns {Promise<Object[]>} Top libraries
   */
  async getTopLibraries(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT
        library_id,
        COUNT(*) as entry_count,
        SUM(hit_count) as total_hits,
        MAX(last_hit_at) as last_accessed
      FROM library_cache
      WHERE expires_at > NOW()
      GROUP BY library_id
      ORDER BY total_hits DESC
      LIMIT $1
    `, [limit]);

    return rows.map(r => ({
      libraryId: r.library_id,
      entryCount: parseInt(r.entry_count),
      totalHits: parseInt(r.total_hits),
      lastAccessed: r.last_accessed,
    }));
  }

  /**
   * Check if a specific query is cached
   * @param {string} libraryId - Library ID
   * @param {string} query - Search query
   * @returns {Promise<boolean>} True if cached and not expired
   */
  async isCached(libraryId, query) {
    const queryHash = hashQuery(query);

    const { rows } = await this.db.query(`
      SELECT 1 FROM library_cache
      WHERE library_id = $1 AND query_hash = $2 AND expires_at > NOW()
      LIMIT 1
    `, [libraryId, queryHash]);

    return rows.length > 0;
  }

  /**
   * Extend TTL for frequently accessed entries
   * @param {number} minHits - Minimum hits to extend
   * @param {number} extraHours - Hours to extend
   * @returns {Promise<number>} Number of extended entries
   */
  async extendPopular(minHits = 10, extraHours = 12) {
    const { rowCount } = await this.db.query(`
      UPDATE library_cache
      SET expires_at = expires_at + INTERVAL '${extraHours} hours'
      WHERE hit_count >= $1 AND expires_at > NOW()
    `, [minHits]);

    return rowCount;
  }
}

export default LibraryCacheRepository;
