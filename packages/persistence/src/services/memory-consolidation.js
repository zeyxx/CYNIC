/**
 * Memory Consolidation Service
 *
 * v1.1: Merges similar memories, prunes low-value ones, applies φ-weighted importance.
 *
 * Like sleep consolidation in human memory - strengthens important memories,
 * merges similar ones, and forgets the irrelevant.
 *
 * φ-derived thresholds:
 * - Similarity threshold: 0.618 (φ⁻¹) - memories must be 61.8% similar to merge
 * - Importance decay: 0.382 (φ⁻²) - importance decays by 38.2% if not accessed
 * - Prune threshold: 0.236 (φ⁻³) - memories below 23.6% importance are pruned
 *
 * "Memory is selective - keep what matters, forget the rest" - κυνικός
 *
 * @module @cynic/persistence/services/memory-consolidation
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('MemoryConsolidation');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;     // 61.8% - similarity threshold
const PHI_INV_2 = 0.381966011250105;   // 38.2% - decay rate
const PHI_INV_3 = 0.236067977499790;   // 23.6% - prune threshold

/**
 * Default consolidation configuration
 */
export const DEFAULT_CONSOLIDATION_CONFIG = Object.freeze({
  similarityThreshold: PHI_INV,        // 61.8% - minimum similarity to merge
  importanceDecay: PHI_INV_2,          // 38.2% - decay multiplier per period
  pruneThreshold: PHI_INV_3,           // 23.6% - minimum importance to keep
  maxMergePerRun: 21,                  // Fibonacci - max merges per consolidation
  maxPrunePerRun: 34,                  // Fibonacci - max prunes per consolidation
  staleAfterDays: 13,                  // Fibonacci - days before memory is considered stale
  minAccessCount: 1,                   // Minimum accesses to avoid decay
  batchSize: 55,                       // Fibonacci - batch size for processing
});

/**
 * Memory Consolidation Service
 *
 * Periodically called to optimize memory storage:
 * 1. Merge similar memories → reduce redundancy
 * 2. Decay unused memories → prioritize accessed memories
 * 3. Prune low-value memories → free storage
 */
export class MemoryConsolidation {
  /**
   * Create a MemoryConsolidation service
   *
   * @param {Object} options - Configuration
   * @param {Object} options.pool - PostgreSQL connection pool
   * @param {Object} [options.embedder] - Embedding service for similarity
   * @param {Object} [options.config] - Override default config
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('MemoryConsolidation requires a database pool');
    }

    this._pool = options.pool;
    this._embedder = options.embedder || null;
    this._config = {
      ...DEFAULT_CONSOLIDATION_CONFIG,
      ...options.config,
    };

    // Stats
    this._lastRun = null;
    this._stats = {
      totalRuns: 0,
      totalMerged: 0,
      totalPruned: 0,
      totalDecayed: 0,
      lastRunDuration: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MAIN CONSOLIDATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Run full consolidation cycle
   *
   * @param {Object} [options] - Run options
   * @param {boolean} [options.dryRun=false] - Don't actually modify data
   * @param {string} [options.userId] - Consolidate only for specific user
   * @returns {Promise<Object>} Consolidation results
   */
  async consolidate(options = {}) {
    const { dryRun = false, userId = null } = options;
    const startTime = Date.now();

    log.info('Starting memory consolidation', { dryRun, userId });

    const results = {
      timestamp: startTime,
      dryRun,
      userId,
      merged: { count: 0, pairs: [] },
      decayed: { count: 0, memories: [] },
      pruned: { count: 0, memories: [] },
      errors: [],
    };

    try {
      // Step 1: Decay stale memories
      const decayResult = await this._decayStaleMemories(userId, dryRun);
      results.decayed = decayResult;

      // Step 2: Merge similar memories (if embedder available)
      if (this._embedder) {
        const mergeResult = await this._mergeSimilarMemories(userId, dryRun);
        results.merged = mergeResult;
      }

      // Step 3: Prune low-value memories
      const pruneResult = await this._pruneLowValueMemories(userId, dryRun);
      results.pruned = pruneResult;

    } catch (err) {
      log.error('Consolidation error', { error: err.message });
      results.errors.push(err.message);
    }

    // Update stats
    const duration = Date.now() - startTime;
    this._lastRun = startTime;
    this._stats.totalRuns++;
    this._stats.lastRunDuration = duration;
    if (!dryRun) {
      this._stats.totalMerged += results.merged.count;
      this._stats.totalPruned += results.pruned.count;
      this._stats.totalDecayed += results.decayed.count;
    }

    results.duration = duration;
    log.info('Consolidation complete', {
      duration,
      merged: results.merged.count,
      decayed: results.decayed.count,
      pruned: results.pruned.count,
    });

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DECAY STALE MEMORIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Decay importance of stale, unaccessed memories
   *
   * @param {string|null} userId - Filter by user
   * @param {boolean} dryRun - Don't modify
   * @returns {Promise<Object>} Decay results
   * @private
   */
  async _decayStaleMemories(userId, dryRun) {
    const result = { count: 0, memories: [] };

    // Find stale memories: old + low access count
    const staleDate = new Date();
    staleDate.setDate(staleDate.getDate() - this._config.staleAfterDays);

    const sql = `
      SELECT id, user_id, importance, access_count, last_accessed_at
      FROM conversation_memories
      WHERE importance > $1
        AND access_count <= $2
        AND (last_accessed_at IS NULL OR last_accessed_at < $3)
        ${userId ? 'AND user_id = $4' : ''}
      ORDER BY importance DESC
      LIMIT $${userId ? 5 : 4}
    `;

    const params = userId
      ? [this._config.pruneThreshold, this._config.minAccessCount, staleDate, userId, this._config.batchSize]
      : [this._config.pruneThreshold, this._config.minAccessCount, staleDate, this._config.batchSize];

    const { rows } = await this._pool.query(sql, params);

    for (const memory of rows) {
      const newImportance = Math.max(
        this._config.pruneThreshold,
        memory.importance * (1 - this._config.importanceDecay),
      );

      result.memories.push({
        id: memory.id,
        oldImportance: memory.importance,
        newImportance,
      });

      if (!dryRun) {
        await this._pool.query(
          'UPDATE conversation_memories SET importance = $1 WHERE id = $2',
          [newImportance, memory.id],
        );
      }
    }

    result.count = result.memories.length;
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MERGE SIMILAR MEMORIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Merge memories with high similarity
   *
   * @param {string|null} userId - Filter by user
   * @param {boolean} dryRun - Don't modify
   * @returns {Promise<Object>} Merge results
   * @private
   */
  async _mergeSimilarMemories(userId, dryRun) {
    const result = { count: 0, pairs: [] };

    // Find memories with embeddings for similarity comparison
    const sql = `
      SELECT id, user_id, memory_type, content, embedding, importance, access_count
      FROM conversation_memories
      WHERE embedding IS NOT NULL
        ${userId ? 'AND user_id = $1' : ''}
      ORDER BY created_at DESC
      LIMIT $${userId ? 2 : 1}
    `;

    const params = userId
      ? [userId, this._config.batchSize]
      : [this._config.batchSize];

    const { rows: memories } = await this._pool.query(sql, params);

    if (memories.length < 2) {
      return result;
    }

    // Compare pairs and find similar ones
    const mergedIds = new Set();
    let mergeCount = 0;

    for (let i = 0; i < memories.length && mergeCount < this._config.maxMergePerRun; i++) {
      if (mergedIds.has(memories[i].id)) continue;

      for (let j = i + 1; j < memories.length && mergeCount < this._config.maxMergePerRun; j++) {
        if (mergedIds.has(memories[j].id)) continue;

        // Same user and type required for merge
        if (memories[i].user_id !== memories[j].user_id) continue;
        if (memories[i].memory_type !== memories[j].memory_type) continue;

        // Calculate similarity
        const similarity = this._cosineSimilarity(
          memories[i].embedding,
          memories[j].embedding,
        );

        if (similarity >= this._config.similarityThreshold) {
          result.pairs.push({
            memory1: memories[i].id,
            memory2: memories[j].id,
            similarity,
          });

          if (!dryRun) {
            await this._mergeMemoryPair(memories[i], memories[j]);
          }

          mergedIds.add(memories[j].id); // Mark secondary as merged (will be deleted)
          mergeCount++;
        }
      }
    }

    result.count = result.pairs.length;
    return result;
  }

  /**
   * Merge two memories into one
   *
   * @param {Object} primary - Primary memory (kept)
   * @param {Object} secondary - Secondary memory (merged into primary, then deleted)
   * @private
   */
  async _mergeMemoryPair(primary, secondary) {
    // Combine importance (capped at 1.0)
    const combinedImportance = Math.min(1.0, primary.importance + secondary.importance * PHI_INV_2);

    // Combine access counts
    const combinedAccess = primary.access_count + secondary.access_count;

    // Combine content (append if different)
    let combinedContent = primary.content;
    if (secondary.content !== primary.content) {
      combinedContent = `${primary.content}\n---\n${secondary.content}`;
    }

    // Update primary
    await this._pool.query(
      `UPDATE conversation_memories
       SET importance = $1, access_count = $2, content = $3
       WHERE id = $4`,
      [combinedImportance, combinedAccess, combinedContent, primary.id],
    );

    // Delete secondary
    await this._pool.query(
      'DELETE FROM conversation_memories WHERE id = $1',
      [secondary.id],
    );

    log.debug('Merged memories', { primary: primary.id, secondary: secondary.id });
  }

  /**
   * Calculate cosine similarity between two embeddings
   *
   * @param {number[]} a - First embedding
   * @param {number[]} b - Second embedding
   * @returns {number} Similarity (0-1)
   * @private
   */
  _cosineSimilarity(a, b) {
    if (!a || !b || a.length !== b.length) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denominator = Math.sqrt(normA) * Math.sqrt(normB);
    return denominator === 0 ? 0 : dotProduct / denominator;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRUNE LOW-VALUE MEMORIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Delete memories below importance threshold
   *
   * @param {string|null} userId - Filter by user
   * @param {boolean} dryRun - Don't modify
   * @returns {Promise<Object>} Prune results
   * @private
   */
  async _pruneLowValueMemories(userId, dryRun) {
    const result = { count: 0, memories: [] };

    // Find low-value memories
    const sql = `
      SELECT id, user_id, importance, memory_type, created_at
      FROM conversation_memories
      WHERE importance <= $1
        ${userId ? 'AND user_id = $2' : ''}
      ORDER BY importance ASC, created_at ASC
      LIMIT $${userId ? 3 : 2}
    `;

    const params = userId
      ? [this._config.pruneThreshold, userId, this._config.maxPrunePerRun]
      : [this._config.pruneThreshold, this._config.maxPrunePerRun];

    const { rows } = await this._pool.query(sql, params);

    for (const memory of rows) {
      result.memories.push({
        id: memory.id,
        importance: memory.importance,
        type: memory.memory_type,
        age: Date.now() - new Date(memory.created_at).getTime(),
      });

      if (!dryRun) {
        await this._pool.query(
          'DELETE FROM conversation_memories WHERE id = $1',
          [memory.id],
        );
      }
    }

    result.count = result.memories.length;
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // IMPORTANCE BOOSTING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Boost importance of a memory (called when memory is useful)
   *
   * @param {string} memoryId - Memory ID
   * @param {number} [boostAmount=0.1] - Amount to boost (0-1)
   * @returns {Promise<Object>} Updated memory
   */
  async boostImportance(memoryId, boostAmount = 0.1) {
    const { rows } = await this._pool.query(
      `UPDATE conversation_memories
       SET importance = LEAST(1.0, importance + $1),
           access_count = access_count + 1,
           last_accessed_at = NOW()
       WHERE id = $2
       RETURNING id, importance, access_count`,
      [boostAmount, memoryId],
    );

    return rows[0] || null;
  }

  /**
   * Calculate φ-weighted importance score
   *
   * Combines multiple factors using golden ratio weights:
   * - Base importance (explicit)
   * - Access frequency (usage-based)
   * - Recency (time-decay)
   * - Content density (information content)
   *
   * @param {Object} memory - Memory object
   * @returns {number} Calculated importance (0-1)
   */
  calculateImportance(memory) {
    const now = Date.now();
    const createdAt = new Date(memory.created_at).getTime();
    const lastAccessed = memory.last_accessed_at
      ? new Date(memory.last_accessed_at).getTime()
      : createdAt;

    // Base importance (explicit assignment)
    const base = memory.importance || 0.5;

    // Access factor: log scale of access count
    const accessFactor = Math.min(1, Math.log10((memory.access_count || 0) + 1) / 2);

    // Recency factor: exponential decay based on last access
    const daysSinceAccess = (now - lastAccessed) / (1000 * 60 * 60 * 24);
    const recencyFactor = Math.exp(-daysSinceAccess / this._config.staleAfterDays);

    // Content density factor: longer content = potentially more valuable
    const contentLength = (memory.content || '').length;
    const densityFactor = Math.min(1, contentLength / 1000);

    // φ-weighted combination
    const weighted =
      base * PHI_INV +           // 61.8% weight on explicit importance
      accessFactor * PHI_INV_2 + // 38.2% weight on usage
      recencyFactor * PHI_INV_3 + // 23.6% weight on recency
      densityFactor * (1 - PHI_INV - PHI_INV_2 - PHI_INV_3); // Remainder on density

    return Math.min(1, Math.max(0, weighted));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get consolidation statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      lastRun: this._lastRun,
      config: { ...this._config },
      hasEmbedder: !!this._embedder,
    };
  }

  /**
   * Get memory health metrics
   *
   * @param {string} [userId] - Filter by user
   * @returns {Promise<Object>} Health metrics
   */
  async getHealthMetrics(userId = null) {
    const baseSql = `
      SELECT
        COUNT(*) as total,
        AVG(importance) as avg_importance,
        AVG(access_count) as avg_access_count,
        COUNT(*) FILTER (WHERE importance <= $1) as low_value_count,
        COUNT(*) FILTER (WHERE last_accessed_at < NOW() - INTERVAL '${this._config.staleAfterDays} days') as stale_count
      FROM conversation_memories
    `;

    const sql = userId
      ? `${baseSql} WHERE user_id = $2`
      : baseSql;

    const params = userId
      ? [this._config.pruneThreshold, userId]
      : [this._config.pruneThreshold];

    const { rows } = await this._pool.query(sql, params);
    const metrics = rows[0];

    return {
      total: parseInt(metrics.total, 10),
      avgImportance: parseFloat(metrics.avg_importance) || 0,
      avgAccessCount: parseFloat(metrics.avg_access_count) || 0,
      lowValueCount: parseInt(metrics.low_value_count, 10),
      staleCount: parseInt(metrics.stale_count, 10),
      healthScore: this._calculateHealthScore(metrics),
      thresholds: {
        prune: this._config.pruneThreshold,
        similarity: this._config.similarityThreshold,
        staleDays: this._config.staleAfterDays,
      },
    };
  }

  /**
   * Calculate overall memory health score
   * @private
   */
  _calculateHealthScore(metrics) {
    const total = parseInt(metrics.total, 10) || 1;
    const lowValueRatio = parseInt(metrics.low_value_count, 10) / total;
    const staleRatio = parseInt(metrics.stale_count, 10) / total;

    // Health decreases with low-value and stale memories
    const health = 1 - (lowValueRatio * PHI_INV + staleRatio * PHI_INV_2);
    return Math.max(0, Math.min(1, health));
  }
}

/**
 * Create a MemoryConsolidation instance
 *
 * @param {Object} options - Options
 * @returns {MemoryConsolidation}
 */
export function createMemoryConsolidation(options) {
  return new MemoryConsolidation(options);
}

export default MemoryConsolidation;
