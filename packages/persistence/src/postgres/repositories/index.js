/**
 * Repository Exports
 *
 * @module @cynic/persistence/repositories
 */

'use strict';

export { JudgmentRepository } from './judgments.js';
export { PatternRepository } from './patterns.js';
export { UserRepository } from './users.js';
export { SessionRepository } from './sessions.js';
export { FeedbackRepository } from './feedback.js';
export { KnowledgeRepository } from './knowledge.js';
export { PoJBlockRepository } from './poj-blocks.js';
export { LibraryCacheRepository } from './library-cache.js';
export { EcosystemDocsRepository } from './ecosystem-docs.js';

// Phase 11: Learning System
export { EScoreHistoryRepository } from './escore-history.js';
export { LearningCyclesRepository } from './learning-cycles.js';
export { PatternEvolutionRepository } from './pattern-evolution.js';
export { UserLearningProfilesRepository } from './user-learning-profiles.js';

// Phase 12: Triggers System
export { TriggerRepository } from './triggers.js';

// Phase 13: Discovery System
export { DiscoveryRepository } from './discovery.js';

// Phase 14: Consciousness Persistence
export { ConsciousnessRepository } from './consciousness.js';

// Phase 15: Psychology System
export { PsychologyRepository } from './psychology.js';

// Phase 16: Total Memory + Full Autonomy
export { ConversationMemoriesRepository, MemoryType } from './conversation-memories.js';
export { ArchitecturalDecisionsRepository, DecisionType, DecisionStatus } from './architectural-decisions.js';
export { LessonsLearnedRepository, LessonCategory, LessonSeverity } from './lessons-learned.js';
export { AutonomousGoalsRepository, GoalType, GoalStatus } from './autonomous-goals.js';
export { AutonomousTasksRepository, TaskStatus, TaskType } from './autonomous-tasks.js';
export { ProactiveNotificationsRepository, NotificationType } from './proactive-notifications.js';
