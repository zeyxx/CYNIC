/**
 * CYNIC Digester Agent - The Archivist
 *
 * "I burn noise, keep signal. Long conversations contain decisions,
 *  patterns, errors - I extract the wisdom." - κυνικός Digester
 *
 * Trigger: PostConversation (end of session)
 * Behavior: Non-blocking
 * Purpose: Extract patterns, decisions, learnings from conversations
 *
 * @module @cynic/node/agents/digester
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from './base.js';

/**
 * Types of knowledge extracted
 */
export const KnowledgeType = {
  DECISION: 'decision',      // Explicit choices made
  PATTERN: 'pattern',        // Recurring behaviors
  ERROR: 'error',            // Mistakes and corrections
  INSIGHT: 'insight',        // Non-obvious observations
  QUESTION: 'question',      // Unresolved questions
  ACTION: 'action',          // Actions taken
  REFERENCE: 'reference',    // External references
};

/**
 * Digest quality levels
 */
export const DigestQuality = {
  GOLD: { min: 80, label: 'Gold' },      // High-value digest
  SILVER: { min: 50, label: 'Silver' },  // Moderate value
  BRONZE: { min: 20, label: 'Bronze' },  // Low value
  NOISE: { min: 0, label: 'Noise' },     // Mostly noise
};

/**
 * Digester Agent - Knowledge Extractor
 */
export class Digester extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'Digester',
      trigger: AgentTrigger.POST_CONVERSATION,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Digestion history
    this.digests = [];
    this.maxDigests = options.maxDigests || 100;

    // Pattern extractors
    this.extractors = {
      decisions: this._extractDecisions.bind(this),
      patterns: this._extractPatterns.bind(this),
      errors: this._extractErrors.bind(this),
      insights: this._extractInsights.bind(this),
      questions: this._extractQuestions.bind(this),
      actions: this._extractActions.bind(this),
      references: this._extractReferences.bind(this),
    };

    // Keywords for detection
    this.keywords = {
      decision: ['decided', 'chose', 'selected', 'will use', 'going with', 'opted for', 'prefer'],
      error: ['error', 'failed', 'mistake', 'wrong', 'bug', 'fix', 'broken', 'issue'],
      insight: ['realized', 'noticed', 'discovered', 'found that', 'interesting', 'actually'],
      question: ['why', 'how', 'what if', 'should we', 'could we', '?'],
      action: ['created', 'updated', 'deleted', 'modified', 'added', 'removed', 'implemented'],
    };
  }

  /**
   * Trigger on conversation end or explicit digest request
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.POST_CONVERSATION ||
           event.type === 'conversation_end' ||
           event.type === 'digest_request';
  }

  /**
   * Analyze conversation for digestable content
   */
  async analyze(event, context) {
    const content = event.content || event.conversation || '';
    const messages = event.messages || [];

    // Extract all types of knowledge
    const extracted = {};
    for (const [type, extractor] of Object.entries(this.extractors)) {
      extracted[type] = await extractor(content, messages, context);
    }

    // Calculate digest quality
    const totalItems = Object.values(extracted).reduce((sum, arr) => sum + arr.length, 0);
    const quality = this._assessQuality(extracted, content);

    return {
      extracted,
      totalItems,
      quality,
      contentLength: content.length,
      messageCount: messages.length,
      confidence: Math.min(PHI_INV, quality.score / 100),
    };
  }

  /**
   * Create digest from analysis
   */
  async decide(analysis, context) {
    const { extracted, quality, totalItems } = analysis;

    // Skip if nothing valuable extracted
    if (totalItems === 0) {
      return {
        response: AgentResponse.LOG,
        action: false,
        reason: 'No significant content to digest',
      };
    }

    // Create digest
    const digest = this._createDigest(extracted, quality, context);

    // Store digest
    this._storeDigest(digest);

    // Record patterns for learning
    for (const pattern of extracted.patterns) {
      this.recordPattern({
        type: 'digested_pattern',
        content: pattern,
      });
    }

    return {
      response: AgentResponse.SUGGEST,
      action: true,
      digest,
      summary: this._generateSummary(digest),
      message: `Digested ${totalItems} items (${quality.label} quality)`,
    };
  }

  /**
   * Extract decisions from content
   * @private
   */
  async _extractDecisions(content, messages, _context) {
    const decisions = [];
    const lines = content.split(/[.!?\n]+/);

    for (const line of lines) {
      const lower = line.toLowerCase();
      for (const keyword of this.keywords.decision) {
        if (lower.includes(keyword)) {
          decisions.push({
            type: KnowledgeType.DECISION,
            content: line.trim(),
            keyword,
            confidence: PHI_INV_2,
          });
          break;
        }
      }
    }

    // Also check structured messages
    for (const msg of messages) {
      if (msg.role === 'assistant' && msg.decision) {
        decisions.push({
          type: KnowledgeType.DECISION,
          content: msg.decision,
          source: 'structured',
          confidence: PHI_INV,
        });
      }
    }

    return this._deduplicate(decisions);
  }

  /**
   * Extract patterns from content
   * @private
   */
  async _extractPatterns(content, messages, _context) {
    const patterns = [];

    // Look for repeated structures
    const codeBlocks = content.match(/```[\s\S]*?```/g) || [];
    if (codeBlocks.length >= 2) {
      // Analyze code patterns
      const languages = codeBlocks.map(block => {
        const match = block.match(/```(\w+)/);
        return match ? match[1] : 'unknown';
      });

      const langCounts = {};
      for (const lang of languages) {
        langCounts[lang] = (langCounts[lang] || 0) + 1;
      }

      for (const [lang, count] of Object.entries(langCounts)) {
        if (count >= 2) {
          patterns.push({
            type: KnowledgeType.PATTERN,
            subtype: 'code_language',
            content: `Frequently used: ${lang} (${count} times)`,
            count,
            confidence: Math.min(PHI_INV, count * 0.15),
          });
        }
      }
    }

    // Look for repeated tool usage
    const toolMentions = content.match(/(?:using|used|called|ran|executed)\s+(\w+)/gi) || [];
    const toolCounts = {};
    for (const mention of toolMentions) {
      const tool = mention.split(/\s+/).pop().toLowerCase();
      toolCounts[tool] = (toolCounts[tool] || 0) + 1;
    }

    for (const [tool, count] of Object.entries(toolCounts)) {
      if (count >= 3) {
        patterns.push({
          type: KnowledgeType.PATTERN,
          subtype: 'tool_usage',
          content: `Frequently used tool: ${tool} (${count} times)`,
          count,
          confidence: Math.min(PHI_INV, count * 0.1),
        });
      }
    }

    return patterns;
  }

  /**
   * Extract errors and corrections
   * @private
   */
  async _extractErrors(content, _messages, _context) {
    const errors = [];
    const lines = content.split(/[.!?\n]+/);

    for (const line of lines) {
      const lower = line.toLowerCase();
      for (const keyword of this.keywords.error) {
        if (lower.includes(keyword)) {
          // Check if it's a correction (error followed by fix)
          const isCorrection = lower.includes('fix') || lower.includes('correct') || lower.includes('solved');

          errors.push({
            type: KnowledgeType.ERROR,
            content: line.trim(),
            keyword,
            corrected: isCorrection,
            confidence: PHI_INV_2,
          });
          break;
        }
      }
    }

    return this._deduplicate(errors);
  }

  /**
   * Extract insights
   * @private
   */
  async _extractInsights(content, _messages, _context) {
    const insights = [];
    const lines = content.split(/[.!?\n]+/);

    for (const line of lines) {
      const lower = line.toLowerCase();
      for (const keyword of this.keywords.insight) {
        if (lower.includes(keyword)) {
          insights.push({
            type: KnowledgeType.INSIGHT,
            content: line.trim(),
            keyword,
            confidence: PHI_INV_3,
          });
          break;
        }
      }
    }

    return this._deduplicate(insights);
  }

  /**
   * Extract unresolved questions
   * @private
   */
  async _extractQuestions(content, _messages, _context) {
    const questions = [];
    const sentences = content.split(/[.!]+/);

    for (const sentence of sentences) {
      if (sentence.includes('?')) {
        // Check if question seems unresolved
        const trimmed = sentence.trim();
        if (trimmed.length > 10 && trimmed.length < 200) {
          questions.push({
            type: KnowledgeType.QUESTION,
            content: trimmed,
            confidence: PHI_INV_3,
          });
        }
      }
    }

    return this._deduplicate(questions).slice(0, 5); // Max 5 questions
  }

  /**
   * Extract actions taken
   * @private
   */
  async _extractActions(content, _messages, _context) {
    const actions = [];
    const lines = content.split(/[.!?\n]+/);

    for (const line of lines) {
      const lower = line.toLowerCase();
      for (const keyword of this.keywords.action) {
        if (lower.includes(keyword)) {
          actions.push({
            type: KnowledgeType.ACTION,
            content: line.trim(),
            keyword,
            confidence: PHI_INV_2,
          });
          break;
        }
      }
    }

    return this._deduplicate(actions);
  }

  /**
   * Extract external references
   * @private
   */
  async _extractReferences(content, _messages, _context) {
    const references = [];

    // URLs
    const urls = content.match(/https?:\/\/[^\s<>"{}|\\^`[\]]+/g) || [];
    for (const url of [...new Set(urls)]) {
      references.push({
        type: KnowledgeType.REFERENCE,
        subtype: 'url',
        content: url,
        confidence: PHI_INV,
      });
    }

    // File paths
    const paths = content.match(/(?:\/[\w.-]+){2,}/g) || [];
    for (const path of [...new Set(paths)].slice(0, 10)) {
      references.push({
        type: KnowledgeType.REFERENCE,
        subtype: 'path',
        content: path,
        confidence: PHI_INV_2,
      });
    }

    return references;
  }

  /**
   * Assess digest quality
   * @private
   */
  _assessQuality(extracted, content) {
    let score = 0;

    // Weight different types
    score += extracted.decisions.length * 15;
    score += extracted.patterns.length * 10;
    score += extracted.errors.length * 8;
    score += extracted.insights.length * 12;
    score += extracted.actions.length * 5;
    score += extracted.references.length * 3;

    // Penalize questions (unresolved = less valuable)
    score -= extracted.questions.length * 2;

    // Normalize by content length (longer = harder to maintain quality)
    const lengthPenalty = Math.max(0, (content.length - 5000) / 1000);
    score -= lengthPenalty;

    // Cap at 100
    score = Math.min(100, Math.max(0, score));

    // Determine label
    for (const [key, level] of Object.entries(DigestQuality)) {
      if (score >= level.min) {
        return { score, label: level.label, key };
      }
    }

    return { score, label: 'Noise', key: 'NOISE' };
  }

  /**
   * Create digest object
   * @private
   */
  _createDigest(extracted, quality, context) {
    return {
      id: `dig_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      timestamp: Date.now(),
      quality,
      content: {
        decisions: extracted.decisions.slice(0, 10),
        patterns: extracted.patterns.slice(0, 10),
        errors: extracted.errors.slice(0, 10),
        insights: extracted.insights.slice(0, 10),
        actions: extracted.actions.slice(0, 15),
        questions: extracted.questions.slice(0, 5),
        references: extracted.references.slice(0, 10),
      },
      stats: {
        totalDecisions: extracted.decisions.length,
        totalPatterns: extracted.patterns.length,
        totalErrors: extracted.errors.length,
        totalInsights: extracted.insights.length,
        totalActions: extracted.actions.length,
        totalQuestions: extracted.questions.length,
        totalReferences: extracted.references.length,
      },
      context: {
        sessionId: context.sessionId,
        project: context.project,
      },
    };
  }

  /**
   * Generate human-readable summary
   * @private
   */
  _generateSummary(digest) {
    const { stats, quality } = digest;
    const parts = [];

    if (stats.totalDecisions > 0) {
      parts.push(`${stats.totalDecisions} decision(s)`);
    }
    if (stats.totalPatterns > 0) {
      parts.push(`${stats.totalPatterns} pattern(s)`);
    }
    if (stats.totalErrors > 0) {
      parts.push(`${stats.totalErrors} error(s)`);
    }
    if (stats.totalInsights > 0) {
      parts.push(`${stats.totalInsights} insight(s)`);
    }

    return `[${quality.label}] Extracted: ${parts.join(', ') || 'minimal content'}`;
  }

  /**
   * Store digest
   * @private
   */
  _storeDigest(digest) {
    this.digests.push(digest);

    if (this.digests.length > this.maxDigests) {
      this.digests = this.digests.slice(-this.maxDigests);
    }
  }

  /**
   * Remove duplicates from array
   * @private
   */
  _deduplicate(items) {
    const seen = new Set();
    return items.filter(item => {
      const key = item.content.toLowerCase().slice(0, 50);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  /**
   * Digest content manually
   * @param {string} content - Content to digest
   * @param {Object} [context] - Context
   * @returns {Promise<Object>} Digest result
   */
  async digest(content, context = {}) {
    return this.process({ type: 'digest_request', content }, context);
  }

  /**
   * Get all digests
   * @param {Object} [options] - Filter options
   * @returns {Object[]} Digests
   */
  getDigests(options = {}) {
    let digests = [...this.digests];

    if (options.minQuality) {
      digests = digests.filter(d => d.quality.score >= options.minQuality);
    }

    if (options.limit) {
      digests = digests.slice(-options.limit);
    }

    return digests;
  }

  /**
   * Get digester summary
   * @returns {Object} Summary
   */
  getSummary() {
    const qualityCounts = { Gold: 0, Silver: 0, Bronze: 0, Noise: 0 };
    for (const digest of this.digests) {
      qualityCounts[digest.quality.label]++;
    }

    return {
      ...this.getStats(),
      totalDigests: this.digests.length,
      qualityDistribution: qualityCounts,
      recentDigests: this.digests.slice(-3).map(d => ({
        id: d.id,
        quality: d.quality.label,
        timestamp: d.timestamp,
      })),
    };
  }
}

export default Digester;
