/**
 * CYNIC Topology Module
 *
 * Self-awareness for CYNIC — knows where it runs, what's available, what it can do.
 * Gap #6 fix: ProcessRegistry adds cross-process awareness.
 *
 * @module @cynic/core/topology
 */

'use strict';

export {
  systemTopology,
  ExecutionMode,
  ServiceStatus,
  RealityDimension,
  AnalysisDimension,
} from './system-topology.js';

export { processRegistry } from './process-registry.js';
