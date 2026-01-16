/**
 * @cynic/node - Collective Scholar Agent
 *
 * SCHOLAR (Daat - Knowledge): The Librarian & Digester
 *
 * "Je cherche, je brûle. La connaissance est feu qui consume et illumine.
 *  Extraire l'essentiel, oublier le reste." - κυνικός Scholar
 *
 * Philosophy: Daat (Knowledge) - Bridge between wisdom and understanding.
 * Trigger: ContextAware (when knowledge is needed)
 * Behavior: Non-blocking (provides knowledge asynchronously)
 *
 * Merged from:
 * - Librarian: Documentation fetching, reference lookup
 * - Digester: Content extraction, knowledge distillation
 *
 * Enhanced collective features:
 * - Privacy-aware storage (LocalStore for device-only data)
 * - Knowledge extraction with commitment hashing
 * - Emits KNOWLEDGE_EXTRACTED for collective learning
 * - Profile-aware documentation level
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/scholar
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from '../base.js';
import {
  AgentEvent,
  AgentId,
  KnowledgeExtractedEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Scholar
 */
export const SCHOLAR_CONSTANTS = {
  /** Max knowledge entries (Fib(12) = 144) */
  MAX_KNOWLEDGE_ENTRIES: 144,

  /** Max digest length (Fib(8) * 100 = 2100 chars) */
  MAX_DIGEST_LENGTH: 2100,

  /** Summary target length (Fib(7) * 10 = 130 chars) */
  SUMMARY_LENGTH: 130,

  /** Cache TTL (Fib(13) = 233 minutes) */
  CACHE_TTL_MS: 233 * 60 * 1000,

  /** Min content length for extraction (Fib(6) = 8 chars) */
  MIN_CONTENT_LENGTH: 8,

  /** Max concurrent fetches (Fib(5) = 5) */
  MAX_CONCURRENT: 5,

  /** Extraction confidence threshold */
  EXTRACTION_THRESHOLD: PHI_INV_2,
};

/**
 * Knowledge types
 */
export const KnowledgeType = {
  DOCUMENTATION: 'documentation',
  CODE_EXAMPLE: 'code_example',
  EXPLANATION: 'explanation',
  REFERENCE: 'reference',
  INSIGHT: 'insight',
  PATTERN: 'pattern',
  ERROR_SOLUTION: 'error_solution',
};

/**
 * Profile-based documentation levels
 */
const PROFILE_DOC_LEVELS = {
  [ProfileLevel.NOVICE]: {
    verbosity: 'detailed',
    includeExamples: true,
    includeGlossary: true,
    complexity: 'beginner',
    maxLength: 3000,
  },
  [ProfileLevel.APPRENTICE]: {
    verbosity: 'moderate',
    includeExamples: true,
    includeGlossary: false,
    complexity: 'intermediate',
    maxLength: 2500,
  },
  [ProfileLevel.PRACTITIONER]: {
    verbosity: 'balanced',
    includeExamples: true,
    includeGlossary: false,
    complexity: 'standard',
    maxLength: 2000,
  },
  [ProfileLevel.EXPERT]: {
    verbosity: 'concise',
    includeExamples: false,
    includeGlossary: false,
    complexity: 'advanced',
    maxLength: 1500,
  },
  [ProfileLevel.MASTER]: {
    verbosity: 'minimal',
    includeExamples: false,
    includeGlossary: false,
    complexity: 'expert',
    maxLength: 1000,
  },
};

/**
 * Collective Scholar Agent - Librarian & Digester
 */
export class CollectiveScholar extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for inter-agent communication
   * @param {Object} [options.localStore] - Local store for device-only data
   * @param {number} [options.profileLevel] - Current user profile level
   */
  constructor(options = {}) {
    super({
      name: 'Scholar',
      trigger: AgentTrigger.CONTEXT_AWARE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Local store for privacy-preserving data
    this.localStore = options.localStore || null;

    // Current profile level
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Knowledge base (in-memory cache)
    this.knowledgeBase = new Map();

    // Extraction history
    this.extractionHistory = [];

    // Pending extractions
    this.pendingExtractions = new Map();

    // Topics of interest (learned from interactions)
    this.topicsOfInterest = new Map();

    // Stats
    this.extractionStats = {
      total: 0,
      byType: {},
      avgConfidence: 0,
    };

    // Subscribe to events
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to event bus events
   * @private
   */
  _subscribeToEvents() {
    // Learn from patterns detected by ANALYST
    this.eventBus.subscribe(
      AgentEvent.PATTERN_DETECTED,
      AgentId.SCHOLAR,
      this._handlePatternDetected.bind(this)
    );

    // Adapt to profile updates
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      AgentId.SCHOLAR,
      this._handleProfileUpdated.bind(this)
    );
  }

  /**
   * Handle pattern detected - learn topics of interest
   * @private
   */
  _handlePatternDetected(event) {
    const { patternType, context } = event.payload;

    // Extract topic from pattern context
    if (context?.topic) {
      const count = this.topicsOfInterest.get(context.topic) || 0;
      this.topicsOfInterest.set(context.topic, count + 1);
    }
  }

  /**
   * Handle profile update
   * @private
   */
  _handleProfileUpdated(event) {
    const { newLevel } = event.payload;
    this.profileLevel = newLevel;
  }

  /**
   * Trigger when knowledge is needed
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.CONTEXT_AWARE ||
           event.type === 'context_aware' ||
           event.needsKnowledge ||
           event.query !== undefined;
  }

  /**
   * Analyze content and extract knowledge
   */
  async analyze(event, context) {
    const content = event.content || event.text || '';
    const query = event.query || context.query || '';
    const source = event.source || context.source || 'unknown';

    if (content.length < SCHOLAR_CONSTANTS.MIN_CONTENT_LENGTH) {
      return {
        extracted: false,
        reason: 'Content too short',
        confidence: 0,
      };
    }

    // Get doc level based on profile
    const docLevel = PROFILE_DOC_LEVELS[this.profileLevel] ||
                     PROFILE_DOC_LEVELS[ProfileLevel.PRACTITIONER];

    // Extract knowledge from content
    const extraction = await this._extractKnowledge(content, query, source, docLevel);

    // Store locally if significant
    if (extraction.confidence >= SCHOLAR_CONSTANTS.EXTRACTION_THRESHOLD && this.localStore) {
      await this._storeLocally(extraction);
    }

    return extraction;
  }

  /**
   * Decide actions based on extraction
   */
  async decide(analysis, context) {
    const { extracted, knowledge, confidence, type } = analysis;

    if (!extracted) {
      return {
        response: AgentResponse.LOG,
        action: false,
      };
    }

    // Update stats
    this._updateStats(type, confidence);

    // Add to knowledge base
    this._addToKnowledgeBase(knowledge);

    // Emit knowledge extracted event
    this._emitKnowledgeExtracted(knowledge, type, confidence, context.source);

    // Record pattern
    this.recordPattern({
      type: 'knowledge_extracted',
      knowledgeType: type,
      confidence,
      topic: knowledge.topic,
    });

    return {
      response: AgentResponse.SUGGEST,
      action: true,
      knowledge,
      confidence,
      type,
    };
  }

  /**
   * Extract knowledge from content
   * @private
   */
  async _extractKnowledge(content, query, source, docLevel) {
    // Determine content type
    const type = this._classifyContent(content);

    // Extract based on type
    let knowledge;
    let confidence;

    switch (type) {
      case KnowledgeType.CODE_EXAMPLE:
        ({ knowledge, confidence } = this._extractCodeKnowledge(content, docLevel));
        break;
      case KnowledgeType.ERROR_SOLUTION:
        ({ knowledge, confidence } = this._extractErrorSolution(content, docLevel));
        break;
      case KnowledgeType.DOCUMENTATION:
        ({ knowledge, confidence } = this._extractDocumentation(content, query, docLevel));
        break;
      default:
        ({ knowledge, confidence } = this._extractGeneral(content, query, docLevel));
    }

    return {
      extracted: confidence >= SCHOLAR_CONSTANTS.EXTRACTION_THRESHOLD,
      knowledge,
      type,
      confidence,
      source,
      extractedAt: new Date().toISOString(),
    };
  }

  /**
   * Classify content type
   * @private
   */
  _classifyContent(content) {
    // Check for code
    const codePatterns = [
      /```[\w]*\n[\s\S]*?```/,
      /function\s+\w+/,
      /class\s+\w+/,
      /const\s+\w+\s*=/,
      /import\s+.*from/,
    ];
    if (codePatterns.some(p => p.test(content))) {
      return KnowledgeType.CODE_EXAMPLE;
    }

    // Check for error solution
    const errorPatterns = [
      /error:/i,
      /exception:/i,
      /fix:|solution:|resolve/i,
      /stack\s*trace/i,
    ];
    if (errorPatterns.some(p => p.test(content))) {
      return KnowledgeType.ERROR_SOLUTION;
    }

    // Check for documentation
    const docPatterns = [
      /^#+\s/m,           // Markdown headers
      /parameters?:/i,
      /returns?:/i,
      /example:/i,
      /usage:/i,
    ];
    if (docPatterns.some(p => p.test(content))) {
      return KnowledgeType.DOCUMENTATION;
    }

    return KnowledgeType.INSIGHT;
  }

  /**
   * Extract code knowledge
   * @private
   */
  _extractCodeKnowledge(content, docLevel) {
    // Extract code blocks
    const codeBlocks = content.match(/```[\w]*\n[\s\S]*?```/g) || [];
    const inlineCode = content.match(/`[^`]+`/g) || [];

    // Extract language hints
    const languages = new Set();
    for (const block of codeBlocks) {
      const langMatch = block.match(/```(\w+)/);
      if (langMatch) {
        languages.add(langMatch[1]);
      }
    }

    // Extract function/class names
    const symbols = this._extractSymbols(content);

    // Create digest based on doc level
    const digest = this._createDigest(content, docLevel);

    const knowledge = {
      topic: symbols[0] || 'code',
      summary: digest.slice(0, SCHOLAR_CONSTANTS.SUMMARY_LENGTH),
      digest,
      codeBlockCount: codeBlocks.length,
      languages: Array.from(languages),
      symbols,
      examples: docLevel.includeExamples ? codeBlocks.slice(0, 3) : [],
    };

    const confidence = Math.min(
      PHI_INV,
      0.3 + (codeBlocks.length * 0.1) + (symbols.length * 0.05)
    );

    return { knowledge, confidence };
  }

  /**
   * Extract error solution knowledge
   * @private
   */
  _extractErrorSolution(content, docLevel) {
    // Extract error message
    const errorMatch = content.match(/error:?\s*([^\n]+)/i);
    const errorMessage = errorMatch ? errorMatch[1].trim() : 'Unknown error';

    // Extract solution/fix
    const solutionMatch = content.match(/(?:fix|solution|resolve)[:\s]*([^\n]+)/i);
    const solution = solutionMatch ? solutionMatch[1].trim() : null;

    // Create digest
    const digest = this._createDigest(content, docLevel);

    const knowledge = {
      topic: 'error-solution',
      errorMessage,
      solution,
      summary: solution || errorMessage.slice(0, SCHOLAR_CONSTANTS.SUMMARY_LENGTH),
      digest,
      category: this._categorizeError(errorMessage),
    };

    const confidence = Math.min(
      PHI_INV,
      solution ? 0.5 : 0.3
    );

    return { knowledge, confidence };
  }

  /**
   * Extract documentation knowledge
   * @private
   */
  _extractDocumentation(content, query, docLevel) {
    // Extract headers/sections
    const headers = content.match(/^#+\s.+$/gm) || [];
    const sections = headers.map(h => h.replace(/^#+\s/, ''));

    // Extract parameters
    const params = this._extractParameters(content);

    // Extract return value
    const returnMatch = content.match(/returns?:?\s*([^\n]+)/i);
    const returnValue = returnMatch ? returnMatch[1].trim() : null;

    // Create digest
    const digest = this._createDigest(content, docLevel);

    // Add glossary for novices
    const glossary = docLevel.includeGlossary
      ? this._extractGlossary(content)
      : null;

    const knowledge = {
      topic: query || sections[0] || 'documentation',
      summary: digest.slice(0, SCHOLAR_CONSTANTS.SUMMARY_LENGTH),
      digest,
      sections,
      parameters: params,
      returnValue,
      glossary,
    };

    const confidence = Math.min(
      PHI_INV,
      0.3 + (sections.length * 0.05) + (params.length * 0.1)
    );

    return { knowledge, confidence };
  }

  /**
   * Extract general knowledge
   * @private
   */
  _extractGeneral(content, query, docLevel) {
    // Extract key points
    const keyPoints = this._extractKeyPoints(content);

    // Create digest
    const digest = this._createDigest(content, docLevel);

    const knowledge = {
      topic: query || 'insight',
      summary: digest.slice(0, SCHOLAR_CONSTANTS.SUMMARY_LENGTH),
      digest,
      keyPoints,
    };

    const confidence = Math.min(
      PHI_INV,
      0.2 + (keyPoints.length * 0.1)
    );

    return { knowledge, confidence };
  }

  /**
   * Create digest based on doc level
   * @private
   */
  _createDigest(content, docLevel) {
    const maxLength = Math.min(
      docLevel.maxLength,
      SCHOLAR_CONSTANTS.MAX_DIGEST_LENGTH
    );

    // Simple extraction - take first N characters
    let digest = content.slice(0, maxLength);

    // Try to end at sentence boundary
    const lastPeriod = digest.lastIndexOf('.');
    if (lastPeriod > maxLength * 0.5) {
      digest = digest.slice(0, lastPeriod + 1);
    }

    return digest.trim();
  }

  /**
   * Extract symbols (functions, classes, etc.)
   * @private
   */
  _extractSymbols(content) {
    const symbols = new Set();

    // Function names
    const funcMatches = content.matchAll(/function\s+(\w+)/g);
    for (const match of funcMatches) {
      symbols.add(match[1]);
    }

    // Class names
    const classMatches = content.matchAll(/class\s+(\w+)/g);
    for (const match of classMatches) {
      symbols.add(match[1]);
    }

    // Const/let/var declarations
    const varMatches = content.matchAll(/(?:const|let|var)\s+(\w+)\s*=/g);
    for (const match of varMatches) {
      symbols.add(match[1]);
    }

    return Array.from(symbols).slice(0, 10);
  }

  /**
   * Extract parameters from documentation
   * @private
   */
  _extractParameters(content) {
    const params = [];
    const paramMatches = content.matchAll(/@param\s+\{?(\w+)\}?\s+(\w+)\s*[-–]?\s*(.+)?/g);

    for (const match of paramMatches) {
      params.push({
        type: match[1],
        name: match[2],
        description: match[3]?.trim() || '',
      });
    }

    return params;
  }

  /**
   * Extract key points from content
   * @private
   */
  _extractKeyPoints(content) {
    const points = [];

    // Bullet points
    const bullets = content.match(/^[-*•]\s+.+$/gm) || [];
    for (const bullet of bullets.slice(0, 5)) {
      points.push(bullet.replace(/^[-*•]\s+/, '').trim());
    }

    // Numbered lists
    const numbered = content.match(/^\d+[.)]\s+.+$/gm) || [];
    for (const num of numbered.slice(0, 5)) {
      points.push(num.replace(/^\d+[.)]\s+/, '').trim());
    }

    return points.slice(0, 8);
  }

  /**
   * Extract glossary terms for novices
   * @private
   */
  _extractGlossary(content) {
    const glossary = {};

    // Look for definition patterns
    const defPatterns = [
      /(\w+):\s*([^.]+\.)/g,           // Term: definition.
      /\*\*(\w+)\*\*\s*[-–]\s*(.+)/g,  // **Term** - definition
    ];

    for (const pattern of defPatterns) {
      const matches = content.matchAll(pattern);
      for (const match of matches) {
        const term = match[1].toLowerCase();
        const definition = match[2].trim();
        if (definition.length > 10 && definition.length < 200) {
          glossary[term] = definition;
        }
      }
    }

    return Object.keys(glossary).length > 0 ? glossary : null;
  }

  /**
   * Categorize error type
   * @private
   */
  _categorizeError(errorMessage) {
    const lower = errorMessage.toLowerCase();

    if (lower.includes('type') || lower.includes('undefined')) {
      return 'type-error';
    }
    if (lower.includes('syntax')) {
      return 'syntax-error';
    }
    if (lower.includes('reference') || lower.includes('not defined')) {
      return 'reference-error';
    }
    if (lower.includes('network') || lower.includes('fetch') || lower.includes('timeout')) {
      return 'network-error';
    }
    if (lower.includes('permission') || lower.includes('access')) {
      return 'permission-error';
    }

    return 'general-error';
  }

  /**
   * Store knowledge locally (privacy-preserving)
   * @private
   */
  async _storeLocally(extraction) {
    if (!this.localStore) return;

    // Only store summary, not raw content
    await this.localStore.store('knowledge', {
      type: extraction.type,
      topic: extraction.knowledge.topic,
      summary: extraction.knowledge.summary,
      confidence: extraction.confidence,
      extractedAt: extraction.extractedAt,
      // Note: Full content is NOT stored (privacy)
    });
  }

  /**
   * Update extraction stats
   * @private
   */
  _updateStats(type, confidence) {
    this.extractionStats.total++;
    this.extractionStats.byType[type] = (this.extractionStats.byType[type] || 0) + 1;

    // Running average confidence
    const n = this.extractionStats.total;
    this.extractionStats.avgConfidence =
      (this.extractionStats.avgConfidence * (n - 1) + confidence) / n;
  }

  /**
   * Add to knowledge base
   * @private
   */
  _addToKnowledgeBase(knowledge) {
    const key = this._hashTopic(knowledge.topic);

    // Enforce bounds
    if (this.knowledgeBase.size >= SCHOLAR_CONSTANTS.MAX_KNOWLEDGE_ENTRIES) {
      // Remove oldest entry
      const oldestKey = this.knowledgeBase.keys().next().value;
      this.knowledgeBase.delete(oldestKey);
    }

    this.knowledgeBase.set(key, {
      ...knowledge,
      addedAt: Date.now(),
    });
  }

  /**
   * Emit knowledge extracted event
   * @private
   */
  _emitKnowledgeExtracted(knowledge, type, confidence, sourceRef) {
    if (!this.eventBus) return;

    const event = new KnowledgeExtractedEvent(
      AgentId.SCHOLAR,
      {
        type,
        topic: knowledge.topic,
        summary: knowledge.summary,
        confidence,
        sourceRef,
      }
    );

    this.eventBus.publish(event);
  }

  /**
   * Hash topic for storage key
   * @private
   */
  _hashTopic(topic) {
    let hash = 0;
    const str = topic.toLowerCase();
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Query knowledge base
   * @param {string} query - Search query
   * @returns {Object[]} Matching knowledge entries
   */
  query(query) {
    const results = [];
    const queryLower = query.toLowerCase();

    for (const [, knowledge] of this.knowledgeBase) {
      const topicMatch = knowledge.topic.toLowerCase().includes(queryLower);
      const summaryMatch = knowledge.summary.toLowerCase().includes(queryLower);

      if (topicMatch || summaryMatch) {
        results.push({
          ...knowledge,
          relevance: topicMatch ? 1.0 : 0.5,
        });
      }
    }

    return results.sort((a, b) => b.relevance - a.relevance);
  }

  /**
   * Extract knowledge from content (public API)
   * @param {string} content - Content to extract from
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Extraction result
   */
  async extract(content, options = {}) {
    const event = {
      content,
      query: options.query,
      source: options.source,
    };

    const analysis = await this.analyze(event, options);
    return this.decide(analysis, options);
  }

  /**
   * Set event bus
   * @param {Object} eventBus - Event bus instance
   */
  setEventBus(eventBus) {
    this.eventBus = eventBus;
    this._subscribeToEvents();
  }

  /**
   * Set local store
   * @param {Object} localStore - Local store instance
   */
  setLocalStore(localStore) {
    this.localStore = localStore;
  }

  /**
   * Update profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    this.profileLevel = level;
  }

  /**
   * Get knowledge base entries
   * @returns {Object[]} Knowledge entries
   */
  getKnowledge() {
    return Array.from(this.knowledgeBase.values());
  }

  /**
   * Get topics of interest
   * @returns {Object[]} Topics with counts
   */
  getTopicsOfInterest() {
    return Array.from(this.topicsOfInterest.entries())
      .map(([topic, count]) => ({ topic, count }))
      .sort((a, b) => b.count - a.count);
  }

  /**
   * Get scholar summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      profileLevel: this.profileLevel,
      knowledgeEntries: this.knowledgeBase.size,
      topicsOfInterest: this.topicsOfInterest.size,
      extractionStats: this.extractionStats,
      topTopics: this.getTopicsOfInterest().slice(0, 5),
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this.knowledgeBase.clear();
    this.extractionHistory = [];
    this.pendingExtractions.clear();
    this.topicsOfInterest.clear();
    this.extractionStats = {
      total: 0,
      byType: {},
      avgConfidence: 0,
    };
  }
}

export default CollectiveScholar;
