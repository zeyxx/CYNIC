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

// v1.1: Burnout Detection
export {
  BurnoutDetection,
  createBurnoutDetection,
  DEFAULT_BURNOUT_CONFIG,
  WarningSeverity,
  WarningType,
} from './burnout-detection.js';

// v1.2: EWC++ Knowledge Retention
export {
  EWCConsolidationService,
  createEWCService,
  EWC_CONFIG,
} from './ewc-consolidation.js';

// M2: Auto Fact Extraction (V2 Gap Analysis)
export {
  FactExtractor,
  createFactExtractor,
} from './fact-extractor.js';

// M3: ReasoningBank (V2 Gap Analysis)
export {
  ReasoningBank,
  createReasoningBank,
  TrajectoryOutcome,
} from './reasoning-bank.js';

// Self-Knowledge: Codebase Indexer
export {
  CodebaseIndexer,
  createCodebaseIndexer,
} from './codebase-indexer.js';

// Telemetry: Usage stats, frictions, benchmarking
export {
  TelemetryCollector,
  createTelemetryCollector,
  getTelemetry,
  MetricType,
  FrictionSeverity,
  Category as TelemetryCategory,
  // Task #62: Thresholds and alerts
  AlertLevel,
  DEFAULT_THRESHOLDS,
} from './telemetry-collector.js';

// Distributed Tracing: Span persistence with buffered batch inserts
export {
  TraceStorage,
  createTraceStorage,
} from './trace-storage.js';
