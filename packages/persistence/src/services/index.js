/**
 * Persistence Services
 *
 * @module @cynic/persistence/services
 */

'use strict';

export {
  MemoryRetriever,
  createMemoryRetriever,
  MemoryType,
  DecisionType,
  DecisionStatus,
  LessonCategory,
  LessonSeverity,
} from './memory-retriever.js';

export {
  Embedder,
  MockEmbedder,
  OpenAIEmbedder,
  OllamaEmbedder,
  EmbedderType,
  EMBEDDING_DIMENSIONS,
  createEmbedder,
  getEmbedder,
  resetEmbedder,
} from './embedder.js';

// v1.1: Memory Consolidation
export {
  MemoryConsolidation,
  createMemoryConsolidation,
  DEFAULT_CONSOLIDATION_CONFIG,
} from './memory-consolidation.js';

// v1.1: Pattern Learning
export {
  PatternLearning,
  createPatternLearning,
  DEFAULT_PATTERN_CONFIG,
} from './pattern-learning.js';
