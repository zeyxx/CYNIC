/**
 * Node Services
 *
 * Background services for CYNIC automation and event handling.
 *
 * @module @cynic/node/services
 */

'use strict';

// Autonomous Daemon - Background task processing
export {
  AutonomousDaemon,
  createAutonomousDaemon,
  registerTaskHandler,
} from './autonomous-daemon.js';

// Event Bus - Centralized pub-sub for automation
export {
  EventBus,
  EventType,
  createEventBus,
  getEventBus,
  publish,
  subscribe,
} from './event-bus.js';

// Automation Executor - Scheduled automation tasks
export {
  AutomationExecutor,
  createAutomationExecutor,
} from './automation-executor.js';

// Skeptic Service - Kabbalistic verification (TZIMTZUM → BERUR → TIKKUN)
export {
  SkepticService,
  createSkepticService,
  ClaimType,
  VerificationStatus,
  SkepticVerdict,
} from './skeptic-service.js';

// Orchestration Visibility - Real-time orchestration display
export {
  OrchestrationVisibility,
  VisibilityLevel,
  createOrchestrationVisibility,
  getOrchestrationVisibility,
} from './orchestration-visibility.js';

// Heartbeat Service - Continuous health monitoring (AXE 5: OBSERVE)
export {
  HeartbeatService,
  HealthStatus,
  createHeartbeatService,
  createDefaultChecks,
  getHeartbeatService,
} from './heartbeat-service.js';

// SLA Tracker - 99.9% uptime compliance (AXE 5: OBSERVE)
export {
  SLATracker,
  SLAStatus,
  SLA_TARGETS,
  createSLATracker,
  getSLATracker,
} from './sla-tracker.js';

// Consciousness Bridge - System health to awareness (AXE 5: OBSERVE)
export {
  ConsciousnessBridge,
  ObservationType,
  createConsciousnessBridge,
  getConsciousnessBridge,
  wireConsciousness,
} from './consciousness-bridge.js';

// Emergence Detector - Cross-session pattern analysis (AXE 6: EMERGE)
export {
  EmergenceDetector,
  PatternCategory,
  SignificanceLevel,
  createEmergenceDetector,
  getEmergenceDetector,
} from './emergence-detector.js';

// Brain Service - Fully configured Brain with orchestrators (CONSCIOUSNESS)
export {
  BrainService,
  BRAIN_CONFIG,
  getBrainService,
  getConfiguredBrain,
  thinkWithBrain,
  _resetBrainServiceForTesting,
} from './brain-service.js';

// Error Handler - Unified error management with consciousness (WEEK 4)
export {
  ErrorHandler,
  ErrorSeverity,
  ErrorCategory,
  getErrorHandler,
  resetErrorHandler,
} from './error-handler.js';
