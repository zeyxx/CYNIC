/**
 * @deprecated Use `@cynic/core` circuit breaker instead.
 * This file is kept for backwards compatibility only.
 *
 * Migration:
 * - import { CircuitBreaker, getCircuitBreakerRegistry } from '@cynic/core';
 *
 * @module @cynic/node/orchestration/circuit-breaker
 */

'use strict';

// Re-export everything from @cynic/core
export {
  CircuitBreaker,
  CircuitBreakerRegistry,
  CircuitState,
  DEFAULT_CIRCUIT_CONFIG,
  withCircuitBreaker,
  getCircuitBreakerRegistry,
} from '@cynic/core';

// Log deprecation warning on first import
import { createLogger } from '@cynic/core';
const log = createLogger('CircuitBreaker');
log.warn('DEPRECATED: Import CircuitBreaker from @cynic/core instead of @cynic/node/orchestration');

export default null;
