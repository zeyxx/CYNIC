/**
 * CYNIC Communication Bus
 *
 * Unified inter-layer communication system following:
 * - N-tier architecture (Presentation → Application → Domain → Infrastructure)
 * - SOLID principles (especially Dependency Inversion)
 *
 * Components:
 * - EventBus: Publish/Subscribe messaging between layers
 * - ServiceRegistry: Service discovery via interfaces
 * - Interfaces: Abstract contracts for each layer
 *
 * "The pack speaks with one voice" - κυνικός
 *
 * @module @cynic/core/bus
 */

'use strict';

// Event Bus (inter-layer messaging)
export {
  CYNICEventBus,
  CYNICEvent,
  EventType,
  globalEventBus,
  publish,
  subscribe,
} from './event-bus.js';

// Parallel Event Bus (non-blocking dispatch)
export {
  ParallelEventBus,
  createParallelEventBus,
} from './parallel-event-bus.js';

// Unified Event Bus (Phase 1: single taxonomy replacing 3 buses)
export {
  UnifiedEventBus,
  UnifiedEvent,
  EventCategory,
  EventPriority,
  getUnifiedEventBus,
  createUnifiedEventBus,
  publish as publishUnified,
  subscribe as subscribeUnified,
} from './unified-event-bus.js';

// Event Adapter (Phase 1, Day 2: migration layer)
export {
  EventAdapter,
  createEventAdapter,
  getTranslationMaps,
} from './event-adapter.js';

// Service Registry (dependency injection)
export {
  ServiceRegistry,
  globalServiceRegistry,
  registerService,
  getService,
} from './service-registry.js';

// Bus Connector (unify EventEmitters)
export {
  connectToBus,
  withBusConnectivity,
  createBusEmitter,
  subscribeToComponent,
  subscribeToAllFromComponent,
  EVENT_NAMESPACES,
} from './connector.js';

// Interfaces (contracts)
export {
  // Infrastructure layer
  IRepository,
  ICache,
  IEventBus,

  // Domain layer
  IEngine,
  IScorer,
  IPattern,

  // Application layer
  IJudge,
  IOrchestrator,
  ISession,

  // Presentation layer
  IHook,
  ITool,

  // Layer utilities
  Layer,
  isLayerCallAllowed,
  implements_,
  assertImplements,
  createLayerProxy,
} from './interfaces.js';
