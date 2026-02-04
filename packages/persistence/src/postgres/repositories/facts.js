/**
 * Facts Repository (M2)
 *
 * Auto-extracted facts from tool outputs and conversations.
 * Cross-session fact retrieval for memory augmentation.
 *
 * Inspired by MoltBrain's facts table with auto-extraction.
 *
 * @module @cynic/persistence/repositories/facts
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

// φ constants
const PHI_INV = 0.618033988749895;

/**
 * Fact types for classification
 */
export const FactType = Object.freeze({
  CODE_PATTERN: 'code_pattern',
  API_DISCOVERY: 'api_discovery',
  ERROR_RESOLUTION: 'error_resolution',
  FILE_STRUCTURE: 'file_structure',
  USER_PREFERENCE: 'user_preference',
  TOOL_RESULT: 'tool_result',
  DECISION: 'decision',
  LEARNING: 'learning',
  REFLECTION: 'reflection',
  SELF_OBSERVATION: 'self_observation',
});

/**
 * Generate short fact ID
 */
function generateFactId() {
  return 'fact_' + crypto.randomBytes(8).toString('hex');
}

/**
 * Facts Repository
 *
 * Stores auto-extracted facts from tool outputs.
 * Supports FTS for semantic retrieval.
 *
 * @extends BaseRepository
 */
export class FactsRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Ensure facts table exists
   */
  async ensureTable() {
    await this.db.query(`
      CREATE TABLE IF NOT EXISTS facts (
        id SERIAL PRIMARY KEY,
        fact_id TEXT UNIQUE NOT NULL,
        user_id TEXT,
        session_id TEXT,
        fact_type TEXT NOT NULL,
        subject TEXT NOT NULL,
        content TEXT NOT NULL,
        context JSONB DEFAULT '{}',
        source_tool TEXT,
        source_file TEXT,
        confidence REAL DEFAULT 0.5,
        relevance REAL DEFAULT 0.5,
        access_count INTEGER DEFAULT 0,
        last_accessed TIMESTAMPTZ,
        tags TEXT[] DEFAULT '{}',
        search_vector TSVECTOR,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW()
      );

      CREATE INDEX IF NOT EXISTS idx_facts_user_id ON facts(user_id);
      CREATE INDEX IF NOT EXISTS idx_facts_session_id ON facts(session_id);
      CREATE INDEX IF NOT EXISTS idx_facts_fact_type ON facts(fact_type);
      CREATE INDEX IF NOT EXISTS idx_facts_source_tool ON facts(source_tool);
      CREATE INDEX IF NOT EXISTS idx_facts_search_vector ON facts USING GIN(search_vector);
      CREATE INDEX IF NOT EXISTS idx_facts_tags ON facts USING GIN(tags);
    `);

    // Create trigger for auto-updating search_vector
    await this.db.query(`
      CREATE OR REPLACE FUNCTION facts_search_trigger() RETURNS trigger AS $$
      BEGIN
        NEW.search_vector :=
          setweight(to_tsvector('english', COALESCE(NEW.subject, '')), 'A') ||
          setweight(to_tsvector('english', COALESCE(NEW.content, '')), 'B') ||
          setweight(to_tsvector('english', COALESCE(array_to_string(NEW.tags, ' '), '')), 'C');
        RETURN NEW;
      END
      $$ LANGUAGE plpgsql;

      DROP TRIGGER IF EXISTS facts_search_update ON facts;
      CREATE TRIGGER facts_search_update BEFORE INSERT OR UPDATE
        ON facts FOR EACH ROW EXECUTE FUNCTION facts_search_trigger();
    `);
  }

  /**
   * Create a fact
   * @param {Object} fact - Fact data
   * @param {number[]} [fact.embedding] - Optional vector embedding
   * @param {string} [fact.embeddingModel] - Model used for embedding
   */
  async create(fact) {
    const factId = generateFactId();

    // Handle embedding - store as JSONB array
    const embedding = fact.embedding ? JSON.stringify(fact.embedding) : null;
    const embeddingModel = fact.embeddingModel || null;
    const embeddingDim = fact.embedding?.length || null;

    const { rows } = await this.db.query(`
      INSERT INTO facts (
        fact_id, user_id, session_id, fact_type, subject, content,
        context, source_tool, source_file, confidence, relevance, tags,
        embedding, embedding_model, embedding_dim
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13::jsonb, $14, $15)
      RETURNING *
    `, [
      factId,
      fact.userId || null,
      fact.sessionId || null,
      fact.factType || FactType.TOOL_RESULT,
      fact.subject,
      fact.content,
      JSON.stringify(fact.context || {}),
      fact.sourceTool || null,
      fact.sourceFile || null,
      Math.min(fact.confidence || 0.5, PHI_INV), // Cap at φ⁻¹
      fact.relevance || 0.5,
      fact.tags || [],
      embedding,
      embeddingModel,
      embeddingDim,
    ]);

    return this._mapRow(rows[0]);
  }

  /**
   * Create multiple facts in a single batch insert
   * Dramatically faster than individual inserts for remote DBs
   *
   * @param {Array<Object>} facts - Array of fact objects
   * @returns {Promise<Array>} Created facts
   */
  async createBatch(facts) {
    if (!facts || facts.length === 0) return [];

    // Build VALUES clause with all facts
    const values = [];
    const params = [];
    let paramIndex = 1;

    for (const fact of facts) {
      const factId = generateFactId();
      const embedding = fact.embedding ? JSON.stringify(fact.embedding) : null;
      const embeddingModel = fact.embeddingModel || null;
      const embeddingDim = fact.embedding?.length || null;

      values.push(`($${paramIndex}, $${paramIndex + 1}, $${paramIndex + 2}, $${paramIndex + 3}, $${paramIndex + 4}, $${paramIndex + 5}, $${paramIndex + 6}, $${paramIndex + 7}, $${paramIndex + 8}, $${paramIndex + 9}, $${paramIndex + 10}, $${paramIndex + 11}, $${paramIndex + 12}::jsonb, $${paramIndex + 13}, $${paramIndex + 14})`);

      params.push(
        factId,
        fact.userId || null,
        fact.sessionId || null,
        fact.factType || FactType.TOOL_RESULT,
        fact.subject,
        fact.content,
        JSON.stringify(fact.context || {}),
        fact.sourceTool || null,
        fact.sourceFile || null,
        Math.min(fact.confidence || 0.5, PHI_INV),
        fact.relevance || 0.5,
        fact.tags || [],
        embedding,
        embeddingModel,
        embeddingDim
      );

      paramIndex += 15;
    }

    const { rows } = await this.db.query(`
      INSERT INTO facts (
        fact_id, user_id, session_id, fact_type, subject, content,
        context, source_tool, source_file, confidence, relevance, tags,
        embedding, embedding_model, embedding_dim
      ) VALUES ${values.join(', ')}
      RETURNING *
    `, params);

    return rows.map(row => this._mapRow(row));
  }

  /**
   * Find fact by ID
   */
  async findById(factId) {
    const { rows } = await this.db.query(
      'SELECT * FROM facts WHERE fact_id = $1',
      [factId]
    );
    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Update a fact
   */
  async update(factId, updates) {
    const fields = [];
    const values = [factId];
    let paramIndex = 2;

    if (updates.content !== undefined) {
      fields.push(`content = $${paramIndex++}`);
      values.push(updates.content);
    }
    if (updates.confidence !== undefined) {
      fields.push(`confidence = $${paramIndex++}`);
      values.push(Math.min(updates.confidence, PHI_INV));
    }
    if (updates.relevance !== undefined) {
      fields.push(`relevance = $${paramIndex++}`);
      values.push(updates.relevance);
    }
    if (updates.context !== undefined) {
      fields.push(`context = context || $${paramIndex++}::jsonb`);
      values.push(JSON.stringify(updates.context));
    }
    if (updates.tags !== undefined) {
      fields.push(`tags = $${paramIndex++}`);
      values.push(updates.tags);
    }

    if (fields.length === 0) return null;

    fields.push('updated_at = NOW()');

    const { rows } = await this.db.query(`
      UPDATE facts SET ${fields.join(', ')}
      WHERE fact_id = $1
      RETURNING *
    `, values);

    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Search facts using full-text search
   */
  async search(query, options = {}) {
    const {
      userId,
      factType,
      sourceTool,
      tags,
      limit = 10,
      minConfidence = 0,
    } = options;

    let sql = `
      SELECT *,
        ts_rank(search_vector, websearch_to_tsquery('english', $1)) as rank
      FROM facts
      WHERE search_vector @@ websearch_to_tsquery('english', $1)
        AND confidence >= $2
    `;
    const params = [query, minConfidence];
    let paramIndex = 3;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (factType) {
      sql += ` AND fact_type = $${paramIndex++}`;
      params.push(factType);
    }

    if (sourceTool) {
      sql += ` AND source_tool = $${paramIndex++}`;
      params.push(sourceTool);
    }

    if (tags && tags.length > 0) {
      sql += ` AND tags && $${paramIndex++}`;
      params.push(tags);
    }

    sql += ` ORDER BY rank DESC, confidence DESC, relevance DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Semantic search combining FTS + vector similarity
   * Uses φ-weighted scoring: 0.382 FTS + 0.618 vector
   *
   * @param {string} query - Search query
   * @param {Object} options - Search options
   * @param {number[]} options.embedding - Query embedding vector
   * @param {number} options.limit - Max results
   * @returns {Promise<Array>} Facts with combined scores
   */
  async semanticSearch(query, options = {}) {
    const {
      embedding,
      userId,
      factType,
      limit = 10,
      minConfidence = 0,
    } = options;

    // If no embedding, fall back to FTS-only
    if (!embedding || embedding.length === 0) {
      return this.search(query, options);
    }

    // φ-weighted hybrid search
    // 1. Get FTS results
    // 2. Compute vector similarity in JS (PostgreSQL JSONB doesn't have native vector ops)
    // 3. Combine with φ weights

    const ftsWeight = 0.382;  // φ⁻²
    const vectorWeight = 0.618;  // φ⁻¹

    // Query facts with embeddings
    let sql = `
      SELECT *,
        ts_rank(search_vector, websearch_to_tsquery('english', $1)) as fts_score
      FROM facts
      WHERE (
        search_vector @@ websearch_to_tsquery('english', $1)
        OR embedding IS NOT NULL
      )
      AND confidence >= $2
    `;
    const params = [query, minConfidence];
    let paramIndex = 3;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (factType) {
      sql += ` AND fact_type = $${paramIndex++}`;
      params.push(factType);
    }

    sql += ` ORDER BY fts_score DESC NULLS LAST, confidence DESC LIMIT $${paramIndex}`;
    params.push(limit * 3); // Over-fetch for re-ranking

    const { rows } = await this.db.query(sql, params);

    // Compute vector similarity and combine scores
    const results = rows.map(row => {
      const fact = this._mapRow(row);
      fact.ftsScore = parseFloat(row.fts_score) || 0;

      // Compute cosine similarity if fact has embedding
      if (row.embedding && Array.isArray(row.embedding)) {
        fact.vectorScore = this._cosineSimilarity(embedding, row.embedding);
      } else {
        fact.vectorScore = 0;
      }

      // Combined φ-weighted score
      fact.combinedScore = (ftsWeight * fact.ftsScore) + (vectorWeight * fact.vectorScore);
      return fact;
    });

    // Sort by combined score and limit
    return results
      .sort((a, b) => b.combinedScore - a.combinedScore)
      .slice(0, limit);
  }

  /**
   * Cosine similarity between two vectors
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

    const magnitude = Math.sqrt(normA) * Math.sqrt(normB);
    if (magnitude === 0) return 0;

    return dotProduct / magnitude;
  }

  /**
   * Find facts by user
   */
  async findByUser(userId, options = {}) {
    const { limit = 20, factType } = options;

    let sql = 'SELECT * FROM facts WHERE user_id = $1';
    const params = [userId];
    let paramIndex = 2;

    if (factType) {
      sql += ` AND fact_type = $${paramIndex++}`;
      params.push(factType);
    }

    sql += ` ORDER BY relevance DESC, created_at DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Find facts by session
   */
  async findBySession(sessionId, limit = 50) {
    const { rows } = await this.db.query(`
      SELECT * FROM facts
      WHERE session_id = $1
      ORDER BY created_at DESC
      LIMIT $2
    `, [sessionId, limit]);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Find related facts (same source file or similar tags)
   */
  async findRelated(factId, limit = 5) {
    const fact = await this.findById(factId);
    if (!fact) return [];

    const { rows } = await this.db.query(`
      SELECT DISTINCT f.* FROM facts f
      WHERE f.fact_id != $1
        AND (
          f.source_file = $2
          OR f.tags && $3
          OR f.fact_type = $4
        )
      ORDER BY f.relevance DESC, f.confidence DESC
      LIMIT $5
    `, [factId, fact.sourceFile, fact.tags, fact.factType, limit]);

    return rows.map(r => this._mapRow(r));
  }

  /**
   * Increment access count and update last_accessed
   */
  async recordAccess(factId) {
    await this.db.query(`
      UPDATE facts
      SET access_count = access_count + 1,
          last_accessed = NOW(),
          relevance = LEAST(1.0, relevance + 0.01)
      WHERE fact_id = $1
    `, [factId]);
  }

  /**
   * Update fact relevance (boost or decay)
   */
  async updateRelevance(factId, delta) {
    const { rows } = await this.db.query(`
      UPDATE facts
      SET relevance = GREATEST(0, LEAST(1.0, relevance + $1)),
          updated_at = NOW()
      WHERE fact_id = $2
      RETURNING *
    `, [delta, factId]);
    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Decay stale facts
   */
  async decayStale(daysOld = 30, decayRate = 0.05) {
    const { rowCount } = await this.db.query(`
      UPDATE facts
      SET relevance = GREATEST(0, relevance - $1)
      WHERE last_accessed < NOW() - INTERVAL '1 day' * $2
        OR (last_accessed IS NULL AND created_at < NOW() - INTERVAL '1 day' * $2)
    `, [decayRate, daysOld]);
    return rowCount;
  }

  /**
   * Get fact statistics
   */
  async getStats(userId = null) {
    let sql = `
      SELECT
        COUNT(*) as total,
        COUNT(DISTINCT fact_type) as types,
        COUNT(DISTINCT source_tool) as tools,
        AVG(confidence) as avg_confidence,
        AVG(relevance) as avg_relevance,
        SUM(access_count) as total_accesses
      FROM facts
    `;
    const params = [];

    if (userId) {
      sql += ' WHERE user_id = $1';
      params.push(userId);
    }

    const { rows } = await this.db.query(sql, params);
    const stats = rows[0];

    return {
      total: parseInt(stats.total),
      types: parseInt(stats.types),
      tools: parseInt(stats.tools),
      avgConfidence: parseFloat(stats.avg_confidence) || 0,
      avgRelevance: parseFloat(stats.avg_relevance) || 0,
      totalAccesses: parseInt(stats.total_accesses) || 0,
    };
  }

  /**
   * Query dependency graph using recursive CTE (SUPERMEMORY enhancement)
   *
   * Traverses file dependencies stored in context.dependencies.
   *
   * @param {string} startPath - File path or symbol to start from
   * @param {Object} options
   * @param {number} options.maxDepth - Maximum traversal depth (default: 5)
   * @param {string} options.direction - 'imports', 'exports', or 'both'
   * @returns {Promise<Object>} Graph with nodes and edges
   */
  async queryDependencyGraph(startPath, options = {}) {
    const { maxDepth = 5, direction = 'both' } = options;

    // First find the starting node(s)
    const startQuery = `
      SELECT fact_id, subject, context->>'path' as file_path,
             context->'dependencies' as deps
      FROM facts
      WHERE fact_type = 'file_structure'
        AND (subject ILIKE $1 OR context->>'path' ILIKE $1)
      LIMIT 10
    `;

    const startNodes = await this.db.query(startQuery, [`%${startPath}%`]);

    if (startNodes.rows.length === 0) {
      return { nodes: [], edges: [], error: 'No matching files found' };
    }

    // Build graph using recursive CTE
    const graphQuery = `
      WITH RECURSIVE dep_tree AS (
        -- Base: starting nodes
        SELECT
          f.fact_id,
          f.subject,
          f.context->>'path' as file_path,
          f.context->'dependencies'->'imports' as imports,
          f.context->'dependencies'->'exports' as exports,
          1 as depth,
          ARRAY[f.fact_id] as visited
        FROM facts f
        WHERE f.fact_id = ANY($1::text[])

        UNION ALL

        -- Recursive: follow imports
        SELECT
          f.fact_id,
          f.subject,
          f.context->>'path' as file_path,
          f.context->'dependencies'->'imports' as imports,
          f.context->'dependencies'->'exports' as exports,
          dt.depth + 1,
          dt.visited || f.fact_id
        FROM facts f
        CROSS JOIN dep_tree dt
        CROSS JOIN LATERAL jsonb_array_elements(dt.imports) as imp
        WHERE f.fact_type = 'file_structure'
          AND dt.depth < $2
          AND NOT f.fact_id = ANY(dt.visited)
          AND (
            f.context->>'path' LIKE '%' || REPLACE(REPLACE(imp->>'source', './', ''), '../', '') || '%'
            OR f.subject LIKE '%' || REPLACE(REPLACE(imp->>'source', './', ''), '../', '') || '%'
          )
      )
      SELECT DISTINCT ON (fact_id)
        fact_id, subject, file_path, imports, exports, depth
      FROM dep_tree
      ORDER BY fact_id, depth
      LIMIT 200
    `;

    try {
      const startIds = startNodes.rows.map(r => r.fact_id);
      const { rows } = await this.db.query(graphQuery, [startIds, maxDepth]);

      // Build nodes and edges
      const nodes = rows.map(r => ({
        id: r.fact_id,
        label: r.file_path || r.subject,
        path: r.file_path,
        depth: r.depth,
        importCount: r.imports?.length || 0,
        exportCount: r.exports?.length || 0,
      }));

      const edges = [];
      const pathToId = new Map(rows.map(r => [r.file_path, r.fact_id]));

      for (const row of rows) {
        if (row.imports) {
          for (const imp of row.imports) {
            const source = imp.source || imp;
            // Find matching target
            for (const [targetPath, targetId] of pathToId) {
              if (targetPath && source &&
                  (targetPath.includes(source.replace('./', '').replace('../', '')) ||
                   source.includes(targetPath.split('/').pop()?.replace('.js', '')))) {
                if (row.fact_id !== targetId) {
                  edges.push({
                    source: row.fact_id,
                    target: targetId,
                    type: 'imports',
                  });
                }
              }
            }
          }
        }
      }

      return {
        startPath,
        direction,
        maxDepth,
        nodes,
        edges,
        stats: {
          nodeCount: nodes.length,
          edgeCount: edges.length,
          maxDepthReached: Math.max(...rows.map(r => r.depth), 0),
        },
      };
    } catch (e) {
      return { error: e.message, nodes: [], edges: [] };
    }
  }

  /**
   * Find all files that import a given file (reverse dependencies)
   *
   * @param {string} filePath - File path to search for
   * @returns {Promise<Array>} List of importing files
   */
  async findReverseDependencies(filePath) {
    const query = `
      SELECT fact_id, subject, context->>'path' as file_path,
             context->'dependencies'->'imports' as imports
      FROM facts
      WHERE fact_type = 'file_structure'
        AND context->'dependencies'->'imports' @> $1::jsonb
    `;

    // Can't use @> directly for partial match, use simpler approach
    const simpleQuery = `
      SELECT fact_id, subject, context->>'path' as file_path
      FROM facts
      WHERE fact_type = 'file_structure'
        AND context::text ILIKE $1
    `;

    const searchTerm = `%${filePath.replace(/\\/g, '/')}%`;

    try {
      const { rows } = await this.db.query(simpleQuery, [searchTerm]);
      return rows.map(r => ({
        factId: r.fact_id,
        subject: r.subject,
        path: r.file_path,
      }));
    } catch (e) {
      return [];
    }
  }

  /**
   * Delete old, low-relevance facts
   */
  async prune(minRelevance = 0.1, daysOld = 90) {
    const { rowCount } = await this.db.query(`
      DELETE FROM facts
      WHERE relevance < $1
        AND created_at < NOW() - INTERVAL '1 day' * $2
    `, [minRelevance, daysOld]);
    return rowCount;
  }

  /**
   * Map database row to fact object
   */
  _mapRow(row) {
    if (!row) return null;
    return {
      factId: row.fact_id,
      userId: row.user_id,
      sessionId: row.session_id,
      factType: row.fact_type,
      subject: row.subject,
      content: row.content,
      context: row.context,
      sourceTool: row.source_tool,
      sourceFile: row.source_file,
      confidence: parseFloat(row.confidence),
      relevance: parseFloat(row.relevance),
      accessCount: row.access_count,
      lastAccessed: row.last_accessed,
      tags: row.tags,
      embedding: row.embedding, // JSONB array
      embeddingModel: row.embedding_model,
      embeddingDim: row.embedding_dim,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      rank: row.rank ? parseFloat(row.rank) : undefined,
    };
  }
}

export default FactsRepository;
