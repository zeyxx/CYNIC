/**
 * CYNIC Progressive Search Module
 *
 * 3-Layer retrieval pattern inspired by claude-mem:
 *   Step 1: search()          -> Index with IDs (~50 tokens/result)
 *   Step 2: timeline(anchor)  -> Context around results
 *   Step 3: get([ids])        -> Full details for filtered IDs
 *
 * Token savings: ~10x compared to returning full results immediately
 *
 * @module tools/search-progressive
 */

const SNIPPET_LENGTH = 100;

/**
 * Generate a snippet from content
 * @param {Object} item - Item to generate snippet from
 * @returns {string} Snippet text
 */
function generateSnippet(item) {
  // Try different fields for content
  const content =
    item.summary ||
    item.content ||
    item.item?.content ||
    item.reasoning ||
    JSON.stringify(item.dimensions?.slice(0, 2)) ||
    '';

  if (typeof content !== 'string') {
    return String(content).slice(0, SNIPPET_LENGTH);
  }

  return content.slice(0, SNIPPET_LENGTH) + (content.length > SNIPPET_LENGTH ? '...' : '');
}

/**
 * Calculate relevance score based on query match
 * @param {Object} item - Item to score
 * @param {string} query - Search query
 * @returns {number} Relevance score (0-1)
 */
function calculateRelevance(item, query) {
  if (!query) return 0.5;

  const q = query.toLowerCase();
  const json = JSON.stringify(item).toLowerCase();

  // Count matches
  const matches = (json.match(new RegExp(q, 'g')) || []).length;

  // Exact field matches get higher score
  let score = Math.min(matches * 0.1, 0.5);

  // Boost for matches in key fields
  if (item.verdict?.toLowerCase().includes(q)) score += 0.2;
  if (item.item_type?.toLowerCase().includes(q)) score += 0.15;
  if (item.summary?.toLowerCase().includes(q)) score += 0.15;

  return Math.min(score, 1);
}

/**
 * Create search index tool (Step 1)
 * Returns lightweight results with IDs and snippets
 *
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createSearchIndexTool(persistence = null) {
  return {
    name: 'brain_search_index',
    description: `Step 1 of progressive search: Returns lightweight index with IDs (~50 tokens/result).
Use this first to find relevant items, then use brain_timeline or brain_get_observations for details.
This approach saves ~10x tokens compared to fetching full results.`,
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Search query',
        },
        type: {
          type: 'string',
          enum: ['judgment', 'pattern', 'knowledge', 'all'],
          description: 'Type of content to search (default: all)',
        },
        limit: {
          type: 'number',
          description: 'Maximum results (default: 20)',
        },
        offset: {
          type: 'number',
          description: 'Skip first N results for pagination (default: 0)',
        },
      },
      required: ['query'],
    },
    handler: async (params) => {
      const { query, type = 'all', limit = 20, offset = 0 } = params;

      if (!query) {
        throw new Error('Missing required parameter: query');
      }

      const results = [];

      // Search judgments
      if (persistence && (type === 'all' || type === 'judgment')) {
        try {
          const judgments = await persistence.searchJudgments(query, { limit: limit * 2 });
          for (const j of judgments) {
            results.push({
              id: j.judgment_id,
              type: 'judgment',
              snippet: generateSnippet(j),
              score: calculateRelevance(j, query),
              verdict: j.verdict,
              qScore: j.q_score,
              timestamp: j.created_at,
            });
          }
        } catch (e) {
          console.error('[search-index] Error searching judgments:', e.message);
        }
      }

      // Search patterns
      if (persistence && (type === 'all' || type === 'pattern')) {
        try {
          const patterns = await persistence.getPatterns({ limit: limit * 2 });
          const q = query.toLowerCase();
          for (const p of patterns) {
            if (JSON.stringify(p).toLowerCase().includes(q)) {
              results.push({
                id: p.pattern_id,
                type: 'pattern',
                snippet: generateSnippet(p),
                score: calculateRelevance(p, query),
                category: p.category,
                timestamp: p.created_at,
              });
            }
          }
        } catch (e) {
          console.error('[search-index] Error searching patterns:', e.message);
        }
      }

      // Search knowledge
      if (persistence && (type === 'all' || type === 'knowledge')) {
        try {
          const knowledge = await persistence.searchKnowledge(query, { limit: limit * 2 });
          for (const k of knowledge) {
            results.push({
              id: k.knowledge_id,
              type: 'knowledge',
              snippet: generateSnippet(k),
              score: calculateRelevance(k, query),
              category: k.category,
              timestamp: k.created_at,
            });
          }
        } catch (e) {
          console.error('[search-index] Error searching knowledge:', e.message);
        }
      }

      // Sort by relevance score
      results.sort((a, b) => b.score - a.score);

      // Apply pagination
      const paginated = results.slice(offset, offset + limit);

      return {
        query,
        type,
        results: paginated,
        total: results.length,
        hasMore: offset + limit < results.length,
        pagination: {
          offset,
          limit,
          returned: paginated.length,
        },
        hint: 'Use brain_timeline with anchor=<id> for context, or brain_get_observations with ids=[...] for full details',
      };
    },
  };
}

/**
 * Create timeline tool (Step 2)
 * Returns context around an anchor observation
 *
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createTimelineTool(persistence = null) {
  return {
    name: 'brain_timeline',
    description: `Step 2 of progressive search: Get temporal context around an observation.
Returns items before and after the anchor point for understanding the sequence of events.`,
    inputSchema: {
      type: 'object',
      properties: {
        anchor: {
          type: 'string',
          description: 'Observation ID to center the timeline on (e.g., jdg_abc123)',
        },
        query: {
          type: 'string',
          description: 'Optional: If anchor not known, finds best match for query and uses as anchor',
        },
        depthBefore: {
          type: 'number',
          description: 'Number of items to show before anchor (default: 3)',
        },
        depthAfter: {
          type: 'number',
          description: 'Number of items to show after anchor (default: 3)',
        },
      },
    },
    handler: async (params) => {
      const { anchor, query, depthBefore = 3, depthAfter = 3 } = params;

      if (!anchor && !query) {
        throw new Error('Either anchor or query must be provided');
      }

      // Collect all items with timestamps
      const allItems = [];

      if (persistence) {
        try {
          // Get all judgments
          const judgments = await persistence.findRecentJudgments
            ? await persistence.findRecentJudgments(1000)
            : await persistence.getRecentJudgments(1000);

          for (const j of judgments || []) {
            allItems.push({
              id: j.judgment_id,
              type: 'judgment',
              timestamp: new Date(j.created_at).getTime(),
              data: {
                verdict: j.verdict,
                qScore: j.q_score,
                itemType: j.item_type,
                snippet: generateSnippet(j),
              },
            });
          }

          // Get all patterns
          const patterns = await persistence.getPatterns({ limit: 1000 });
          for (const p of patterns || []) {
            allItems.push({
              id: p.pattern_id,
              type: 'pattern',
              timestamp: new Date(p.created_at).getTime(),
              data: {
                category: p.category,
                name: p.name,
                snippet: generateSnippet(p),
              },
            });
          }

          // Get all knowledge
          const knowledge = await persistence.searchKnowledge('', { limit: 1000 });
          for (const k of knowledge || []) {
            allItems.push({
              id: k.knowledge_id,
              type: 'knowledge',
              timestamp: new Date(k.created_at).getTime(),
              data: {
                category: k.category,
                snippet: generateSnippet(k),
              },
            });
          }
        } catch (e) {
          console.error('[timeline] Error collecting items:', e.message);
        }
      }

      // Sort by timestamp
      allItems.sort((a, b) => a.timestamp - b.timestamp);

      // Find anchor
      let anchorIndex = -1;
      let anchorItem = null;

      if (anchor) {
        // Direct ID lookup
        anchorIndex = allItems.findIndex((item) => item.id === anchor);
      } else if (query) {
        // Find best match
        let bestScore = -1;
        allItems.forEach((item, idx) => {
          const score = calculateRelevance(item, query);
          if (score > bestScore) {
            bestScore = score;
            anchorIndex = idx;
          }
        });
      }

      if (anchorIndex === -1) {
        return {
          error: 'Anchor not found',
          query,
          anchor,
          suggestion: 'Use brain_search_index to find valid IDs first',
        };
      }

      anchorItem = allItems[anchorIndex];

      // Get context window
      const startIdx = Math.max(0, anchorIndex - depthBefore);
      const endIdx = Math.min(allItems.length - 1, anchorIndex + depthAfter);

      const before = allItems.slice(startIdx, anchorIndex);
      const after = allItems.slice(anchorIndex + 1, endIdx + 1);

      return {
        before,
        anchor: anchorItem,
        after,
        context: {
          totalItems: allItems.length,
          anchorPosition: anchorIndex,
          windowStart: startIdx,
          windowEnd: endIdx,
        },
        hint: 'Use brain_get_observations with ids=[...] to get full details for specific items',
      };
    },
  };
}

/**
 * Create get observations tool (Step 3)
 * Returns full details for specific IDs
 *
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createGetObservationsTool(persistence = null) {
  return {
    name: 'brain_get_observations',
    description: `Step 3 of progressive search: Fetch full details for specific observation IDs.
Only use this for items you've already identified via brain_search_index or brain_timeline.`,
    inputSchema: {
      type: 'object',
      properties: {
        ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Array of observation IDs to fetch (e.g., ["jdg_abc123", "pat_def456"])',
        },
      },
      required: ['ids'],
    },
    handler: async (params) => {
      const { ids } = params;

      if (!ids || !Array.isArray(ids) || ids.length === 0) {
        throw new Error('Missing required parameter: ids (array of observation IDs)');
      }

      const observations = [];
      const notFound = [];

      for (const id of ids) {
        let found = false;

        // Determine type from ID prefix
        const type = id.startsWith('jdg_')
          ? 'judgment'
          : id.startsWith('pat_')
            ? 'pattern'
            : id.startsWith('kn_')
              ? 'knowledge'
              : 'unknown';

        if (persistence && type === 'judgment') {
          try {
            // Search for specific judgment
            const judgments = await persistence.searchJudgments(id, { limit: 100 });
            const judgment = judgments.find((j) => j.judgment_id === id);
            if (judgment) {
              observations.push({
                id: judgment.judgment_id,
                type: 'judgment',
                full: true,
                data: judgment,
              });
              found = true;
            }
          } catch (e) {
            console.error('[get_observations] Error fetching judgment:', e.message);
          }
        }

        if (persistence && type === 'pattern') {
          try {
            const patterns = await persistence.getPatterns({ limit: 1000 });
            const pattern = patterns.find((p) => p.pattern_id === id);
            if (pattern) {
              observations.push({
                id: pattern.pattern_id,
                type: 'pattern',
                full: true,
                data: pattern,
              });
              found = true;
            }
          } catch (e) {
            console.error('[get_observations] Error fetching pattern:', e.message);
          }
        }

        if (persistence && type === 'knowledge') {
          try {
            const knowledge = await persistence.searchKnowledge(id, { limit: 1000 });
            const item = knowledge.find((k) => k.knowledge_id === id);
            if (item) {
              observations.push({
                id: item.knowledge_id,
                type: 'knowledge',
                full: true,
                data: item,
              });
              found = true;
            }
          } catch (e) {
            console.error('[get_observations] Error fetching knowledge:', e.message);
          }
        }

        if (!found) {
          notFound.push(id);
        }
      }

      return {
        observations,
        notFound,
        stats: {
          requested: ids.length,
          found: observations.length,
          missing: notFound.length,
        },
      };
    },
  };
}

/**
 * Create all progressive search tools
 *
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object[]} Array of tool definitions
 */
export function createProgressiveSearchTools(persistence = null) {
  return [
    createSearchIndexTool(persistence),
    createTimelineTool(persistence),
    createGetObservationsTool(persistence),
  ];
}

export default {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
};
