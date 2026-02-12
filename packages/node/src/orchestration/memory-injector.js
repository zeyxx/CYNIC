/**
 * Memory Injector - Injects collective memory into Dog judgment prompts
 *
 * Connects PostgreSQL memory (patterns, judgments, learning events)
 * to active LLM sessions during Dog invocations.
 *
 * "Dogs remember. The pack learns." - ?????????
 *
 * @module @cynic/node/orchestration/memory-injector
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { PatternRepository, JudgmentRepository } from '@cynic/persistence';

const log = createLogger('MemoryInjector');

/**
 * Memory Injector
 *
 * Retrieves and formats memory context from PostgreSQL
 * for injection into Dog judgment prompts.
 */
export class MemoryInjector {
  constructor(options = {}) {
    this.patternRepo = options.patternRepo || new PatternRepository();
    this.judgmentRepo = options.judgmentRepo || new JudgmentRepository();

    // Token limits
    this.maxTokens = options.maxTokens || 2000;
    this.tokensPerPattern = options.tokensPerPattern || 150;
    this.tokensPerJudgment = options.tokensPerJudgment || 200;

    // Cache
    this._cache = new Map();
    this._cacheMaxAge = options.cacheMaxAge || 60000;

    this.stats = {
      injections: 0,
      cacheHits: 0,
      cacheMisses: 0,
      totalTokensInjected: 0,
    };
  }

  async getMemoryContext(options = {}) {
    const cacheKey = this._getCacheKey(options);

    const cached = this._cache.get(cacheKey);
    if (cached && Date.now() - cached.timestamp < this._cacheMaxAge) {
      this.stats.cacheHits++;
      log.debug('Memory cache hit', { cacheKey });
      return cached.context;
    }

    this.stats.cacheMisses++;

    const [patterns, judgments] = await Promise.all([
      this._getRelevantPatterns(options),
      this._getRelevantJudgments(options),
    ]);

    const context = this._formatMemoryContext(patterns, judgments, options);

    this._cache.set(cacheKey, {
      context,
      timestamp: Date.now(),
    });

    this.stats.injections++;
    this.stats.totalTokensInjected += context.tokenEstimate;

    log.debug('Memory context retrieved', {
      patterns: patterns.length,
      judgments: judgments.length,
      tokens: context.tokenEstimate,
    });

    return context;
  }

  injectIntoPrompt(basePrompt, memoryContext, options = {}) {
    if (!memoryContext || !memoryContext.summary) {
      return basePrompt;
    }

    const position = options.position || 'before';
    const separator = options.separator || '\n\n---\n\n';

    if (position === 'before') {
      return memoryContext.summary + separator + basePrompt;
    } else {
      return basePrompt + separator + memoryContext.summary;
    }
  }

  async _getRelevantPatterns(options) {
    const maxPatterns = options.maxPatterns || 3;

    try {
      if (options.tags && options.tags.length > 0) {
        return await this.patternRepo.findByTags(options.tags, maxPatterns);
      }

      if (options.domain || options.task) {
        const query = options.domain || options.task;
        const results = await this.patternRepo.search(query, { limit: maxPatterns });
        return results.patterns || [];
      }

      return await this.patternRepo.getTopPatterns(maxPatterns);
    } catch (error) {
      log.error('Failed to retrieve patterns', { error: error.message });
      return [];
    }
  }

  async _getRelevantJudgments(options) {
    const maxJudgments = options.maxJudgments || 2;

    try {
      if (options.context || options.task) {
        const query = options.context || options.task;
        const results = await this.judgmentRepo.search(query, { limit: maxJudgments });
        return results.judgments || [];
      }

      return await this.judgmentRepo.findRecent(maxJudgments);
    } catch (error) {
      log.error('Failed to retrieve judgments', { error: error.message });
      return [];
    }
  }

  _formatMemoryContext(patterns, judgments, _options) {
    const parts = [];
    let tokenEstimate = 0;

    parts.push('### COLLECTIVE MEMORY');
    parts.push('*The pack remembers these patterns and judgments:*\n');
    tokenEstimate += 20;

    if (patterns.length > 0) {
      parts.push('**Patterns:**');
      for (const pattern of patterns) {
        const confidence = Math.min(pattern.confidence || PHI_INV, PHI_INV);
        const occurrences = pattern.occurrences || pattern.count || 1;
        
        const line1 = '- "' + (pattern.name || pattern.pattern_name) + '" (' + occurrences + 'x, phi=' + (confidence * 100).toFixed(0) + '%)';
        parts.push(line1);
        
        if (pattern.description) {
          parts.push('  ' + pattern.description.slice(0, 150));
        }
        
        tokenEstimate += this.tokensPerPattern;
        if (tokenEstimate >= this.maxTokens) break;
      }
      parts.push('');
    }

    if (judgments.length > 0 && tokenEstimate < this.maxTokens) {
      parts.push('**Past Judgments:**');
      for (const judgment of judgments) {
        const confidence = Math.min(judgment.confidence || PHI_INV, PHI_INV);
        const verdict = judgment.verdict || 'UNKNOWN';
        
        const line1 = '- ' + verdict + ' (phi=' + (confidence * 100).toFixed(0) + '%)';
        parts.push(line1);
        
        if (judgment.reasoning) {
          parts.push('  ' + judgment.reasoning.slice(0, 120));
        }
        
        tokenEstimate += this.tokensPerJudgment;
        if (tokenEstimate >= this.maxTokens) break;
      }
      parts.push('');
    }

    parts.push('*Use this memory to inform your judgment, but remain skeptical.*');
    tokenEstimate += 15;

    return {
      patterns,
      judgments,
      summary: parts.join('\n'),
      tokenEstimate,
    };
  }

  _getCacheKey(options) {
    const parts = [
      options.domain || 'general',
      options.task || '',
      (options.tags || []).sort().join(','),
    ];
    return parts.join('|');
  }

  clearCache() {
    const size = this._cache.size;
    this._cache.clear();
    log.debug('Memory cache cleared', { entries: size });
  }

  getStats() {
    return {
      ...this.stats,
      cacheSize: this._cache.size,
      avgTokensPerInjection: this.stats.injections > 0
        ? Math.round(this.stats.totalTokensInjected / this.stats.injections)
        : 0,
    };
  }
}

export default MemoryInjector;
