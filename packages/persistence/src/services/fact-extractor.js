/**
 * Fact Extractor Service (M2)
 *
 * Auto-extracts facts from tool outputs.
 * Inspired by MoltBrain's fact extraction pipeline.
 *
 * Extraction patterns:
 * - File discoveries (Read, Glob results)
 * - API patterns (WebFetch results)
 * - Error resolutions (what fixed what)
 * - Code patterns (Edit, Write results)
 * - User preferences (repeated patterns)
 *
 * @module @cynic/persistence/services/fact-extractor
 */

'use strict';

import { createLogger } from '@cynic/core';
import { FactsRepository, FactType } from '../postgres/repositories/facts.js';

const log = createLogger('FactExtractor');

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Extraction patterns for different tool types
 */
const EXTRACTION_PATTERNS = {
  // Read tool - extract file structure facts
  Read: {
    type: FactType.FILE_STRUCTURE,
    extract: (input, output) => {
      if (!output || output.length < 100) return null;

      const facts = [];
      const filePath = input.file_path || input.filePath;

      // Extract export patterns
      const exports = output.match(/export\s+(?:default\s+)?(?:class|function|const|let|var)\s+(\w+)/g);
      if (exports?.length) {
        facts.push({
          subject: `Exports in ${filePath}`,
          content: `File ${filePath} exports: ${exports.slice(0, 10).join(', ')}`,
          tags: ['exports', getFileExtension(filePath)],
          confidence: PHI_INV_2,
        });
      }

      // Extract import patterns
      const imports = output.match(/(?:import|require)\s*\(['"]([@\w\-/]+)['"]\)|from\s+['"]([@\w\-/]+)['"]/g);
      if (imports?.length > 3) {
        facts.push({
          subject: `Dependencies of ${filePath}`,
          content: `File ${filePath} depends on: ${[...new Set(imports)].slice(0, 10).join(', ')}`,
          tags: ['dependencies', 'imports'],
          confidence: PHI_INV_2,
        });
      }

      return facts.length ? facts : null;
    },
  },

  // Glob tool - extract file structure facts
  Glob: {
    type: FactType.FILE_STRUCTURE,
    extract: (input, output) => {
      if (!output || typeof output !== 'string') return null;

      const files = output.split('\n').filter(f => f.trim());
      if (files.length < 2) return null;

      const pattern = input.pattern || 'unknown pattern';
      const directories = [...new Set(files.map(f => f.split('/').slice(0, -1).join('/')))];

      return [{
        subject: `Files matching ${pattern}`,
        content: `Found ${files.length} files in ${directories.length} directories matching pattern "${pattern}"`,
        tags: ['file_structure', 'glob', pattern.replace(/[*?]/g, '')],
        confidence: PHI_INV_2,
        context: {
          pattern,
          count: files.length,
          sampleFiles: files.slice(0, 5),
          directories: directories.slice(0, 5),
        },
      }];
    },
  },

  // Grep tool - extract code patterns
  Grep: {
    type: FactType.CODE_PATTERN,
    extract: (input, output) => {
      if (!output || typeof output !== 'string') return null;

      const pattern = input.pattern;
      const matches = output.split('\n').filter(l => l.trim());

      if (matches.length < 2) return null;

      return [{
        subject: `Pattern "${pattern}" locations`,
        content: `Found ${matches.length} occurrences of pattern "${pattern}"`,
        tags: ['code_pattern', 'search'],
        confidence: PHI_INV_2,
        context: {
          pattern,
          matchCount: matches.length,
          sampleMatches: matches.slice(0, 5),
        },
      }];
    },
  },

  // Edit tool - extract code change patterns
  Edit: {
    type: FactType.CODE_PATTERN,
    extract: (input, output, context) => {
      const filePath = input.file_path || input.filePath;
      const oldString = input.old_string || input.oldString || '';
      const newString = input.new_string || input.newString || '';

      if (!filePath || oldString === newString) return null;

      // Detect what kind of change
      const changeType = detectChangeType(oldString, newString);

      return [{
        subject: `${changeType} in ${filePath}`,
        content: `Applied ${changeType.toLowerCase()} to ${filePath}: replaced ${oldString.length} chars with ${newString.length} chars`,
        tags: ['edit', changeType.toLowerCase(), getFileExtension(filePath)],
        confidence: PHI_INV_2,
        sourceFile: filePath,
        context: {
          changeType,
          oldLength: oldString.length,
          newLength: newString.length,
          replaceAll: input.replace_all || input.replaceAll,
        },
      }];
    },
  },

  // Bash tool - extract command patterns
  Bash: {
    type: FactType.TOOL_RESULT,
    extract: (input, output, context) => {
      const command = input.command;
      if (!command) return null;

      const facts = [];

      // npm/pnpm install - package discovery
      if (/^(?:npm|pnpm|yarn)\s+install/.test(command)) {
        facts.push({
          subject: 'Package installation',
          content: `Installed packages with: ${command.slice(0, 100)}`,
          tags: ['npm', 'dependencies', 'install'],
          confidence: 0.4,
        });
      }

      // git commands - repository info
      if (/^git\s+(status|log|diff|branch)/.test(command) && output) {
        const gitCmd = command.match(/^git\s+(\w+)/)?.[1];
        facts.push({
          subject: `Git ${gitCmd} result`,
          content: output.slice(0, 500),
          tags: ['git', gitCmd],
          confidence: 0.3,
        });
      }

      // test commands - test results
      if (/(?:npm|pnpm|node)\s+(?:test|--test)/.test(command) && output) {
        const passMatch = output.match(/(\d+)\s+pass/i);
        const failMatch = output.match(/(\d+)\s+fail/i);

        if (passMatch || failMatch) {
          facts.push({
            subject: 'Test results',
            content: `Tests: ${passMatch?.[1] || 0} passed, ${failMatch?.[1] || 0} failed`,
            tags: ['tests', 'quality'],
            confidence: PHI_INV_2,
            context: {
              passed: parseInt(passMatch?.[1] || 0),
              failed: parseInt(failMatch?.[1] || 0),
            },
          });
        }
      }

      return facts.length ? facts : null;
    },
  },

  // WebFetch - API discovery
  WebFetch: {
    type: FactType.API_DISCOVERY,
    extract: (input, output) => {
      const url = input.url;
      if (!url || !output) return null;

      const domain = new URL(url).hostname;

      return [{
        subject: `API: ${domain}`,
        content: `Fetched from ${domain}: ${output.slice(0, 300)}...`,
        tags: ['api', 'web', domain],
        confidence: 0.4,
        context: { url, domain },
      }];
    },
  },
};

/**
 * Detect type of code change
 */
function detectChangeType(oldStr, newStr) {
  if (!oldStr && newStr) return 'Addition';
  if (oldStr && !newStr) return 'Deletion';
  if (newStr.length > oldStr.length * 1.5) return 'Expansion';
  if (newStr.length < oldStr.length * 0.5) return 'Reduction';
  if (/fix|bug|error/i.test(newStr)) return 'Bugfix';
  if (/refactor/i.test(newStr)) return 'Refactor';
  return 'Modification';
}

/**
 * Get file extension
 */
function getFileExtension(filePath) {
  if (!filePath) return 'unknown';
  const ext = filePath.split('.').pop()?.toLowerCase();
  return ext || 'unknown';
}

/**
 * Fact Extractor Service
 */
export class FactExtractor {
  /**
   * Create FactExtractor
   *
   * @param {Object} options
   * @param {Object} options.pool - PostgreSQL pool
   * @param {Object} [options.vectorStore] - Optional VectorStore for semantic enrichment
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('FactExtractor requires database pool');
    }

    this._pool = options.pool;
    this._vectorStore = options.vectorStore || null;
    this._factsRepo = new FactsRepository(options.pool);
    this._initialized = false;

    this._stats = {
      processed: 0,
      extracted: 0,
      stored: 0,
      errors: 0,
    };
  }

  /**
   * Initialize (ensure tables exist)
   */
  async initialize() {
    if (this._initialized) return;

    try {
      await this._factsRepo.ensureTable();
      this._initialized = true;
      log.info('FactExtractor initialized');
    } catch (err) {
      log.error('Failed to initialize FactExtractor', { error: err.message });
      throw err;
    }
  }

  /**
   * Extract facts from a tool result
   *
   * @param {Object} toolResult - Tool execution result
   * @param {string} toolResult.tool - Tool name
   * @param {Object} toolResult.input - Tool input
   * @param {*} toolResult.output - Tool output
   * @param {Object} [context] - Additional context
   * @returns {Promise<Object[]>} Extracted facts
   */
  async extract(toolResult, context = {}) {
    await this.initialize();

    this._stats.processed++;
    const { tool, input, output } = toolResult;

    // Get extraction pattern for this tool
    const pattern = EXTRACTION_PATTERNS[tool];
    if (!pattern) return [];

    try {
      // Run extraction
      const extracted = pattern.extract(input, output, context);
      if (!extracted || extracted.length === 0) return [];

      const facts = [];

      for (const factData of extracted) {
        const fact = await this._factsRepo.create({
          userId: context.userId,
          sessionId: context.sessionId,
          factType: pattern.type,
          sourceTool: tool,
          sourceFile: factData.sourceFile || input.file_path || input.filePath,
          ...factData,
        });

        facts.push(fact);
        this._stats.extracted++;
        this._stats.stored++;

        // Store in VectorStore if available for semantic search
        if (this._vectorStore) {
          try {
            await this._vectorStore.store(
              fact.factId,
              `${fact.subject} ${fact.content}`,
              { factType: fact.factType, confidence: fact.confidence }
            );
          } catch (e) {
            // Non-critical
          }
        }
      }

      log.debug('Extracted facts', { tool, count: facts.length });
      return facts;

    } catch (err) {
      this._stats.errors++;
      log.error('Fact extraction error', { tool, error: err.message });
      return [];
    }
  }

  /**
   * Extract facts from error resolution
   *
   * @param {Object} resolution
   * @param {string} resolution.error - Original error
   * @param {string} resolution.solution - What fixed it
   * @param {Object} [context]
   */
  async extractErrorResolution(resolution, context = {}) {
    await this.initialize();

    const { error, solution, tool, file } = resolution;

    const fact = await this._factsRepo.create({
      userId: context.userId,
      sessionId: context.sessionId,
      factType: FactType.ERROR_RESOLUTION,
      subject: `Fix for: ${error.slice(0, 100)}`,
      content: `Error: ${error}\n\nSolution: ${solution}`,
      sourceTool: tool,
      sourceFile: file,
      tags: ['error', 'resolution', 'fix'],
      confidence: PHI_INV, // High confidence for verified fixes
      relevance: PHI_INV,
      context: { errorType: error.split(':')[0] },
    });

    this._stats.extracted++;
    this._stats.stored++;

    return fact;
  }

  /**
   * Extract user preference from repeated behavior
   *
   * @param {Object} preference
   * @param {string} preference.type - Preference type
   * @param {string} preference.value - Preference value
   * @param {number} preference.occurrences - How many times seen
   */
  async extractUserPreference(preference, context = {}) {
    await this.initialize();

    const { type, value, occurrences } = preference;

    // Only store if seen multiple times
    if (occurrences < 3) return null;

    const fact = await this._factsRepo.create({
      userId: context.userId,
      sessionId: context.sessionId,
      factType: FactType.USER_PREFERENCE,
      subject: `Preference: ${type}`,
      content: `User prefers ${type}: ${value} (seen ${occurrences} times)`,
      tags: ['preference', type],
      confidence: Math.min(PHI_INV, occurrences * 0.1),
      relevance: PHI_INV,
      context: { type, value, occurrences },
    });

    this._stats.extracted++;
    this._stats.stored++;

    return fact;
  }

  /**
   * Search for relevant facts
   */
  async search(query, options = {}) {
    await this.initialize();
    return this._factsRepo.search(query, options);
  }

  /**
   * Get facts for current session
   */
  async getSessionFacts(sessionId) {
    await this.initialize();
    return this._factsRepo.findBySession(sessionId);
  }

  /**
   * Get facts for user
   */
  async getUserFacts(userId, options = {}) {
    await this.initialize();
    return this._factsRepo.findByUser(userId, options);
  }

  /**
   * Record fact access (for relevance boosting)
   */
  async recordAccess(factId) {
    await this.initialize();
    return this._factsRepo.recordAccess(factId);
  }

  /**
   * Run maintenance (decay stale, prune old)
   */
  async runMaintenance() {
    await this.initialize();

    const decayed = await this._factsRepo.decayStale(30, 0.05);
    const pruned = await this._factsRepo.prune(0.1, 90);

    log.info('Fact maintenance complete', { decayed, pruned });
    return { decayed, pruned };
  }

  /**
   * Get statistics
   */
  async getStats(userId = null) {
    await this.initialize();

    const repoStats = await this._factsRepo.getStats(userId);
    return {
      ...repoStats,
      processing: { ...this._stats },
    };
  }
}

/**
 * Create FactExtractor instance
 */
export function createFactExtractor(options) {
  return new FactExtractor(options);
}

export default FactExtractor;
