/**
 * Shared Enums — used across all cycle components
 *
 * These enums were duplicated identically in every Actor, Decider, etc.
 * Now they exist ONCE.
 *
 * @module @cynic/node/cycle/shared-enums
 */

'use strict';

/**
 * ActionStatus — identical in CodeActor, CosmosActor, CynicActor
 */
export const ActionStatus = {
  QUEUED: 'queued',
  DELIVERED: 'delivered',
  ACTED_ON: 'acted_on',
  DISMISSED: 'dismissed',
  EXPIRED: 'expired',
};

/**
 * SignificanceLevel — identical in all 6 Emergence files
 */
export const SignificanceLevel = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};
