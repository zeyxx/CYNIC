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
