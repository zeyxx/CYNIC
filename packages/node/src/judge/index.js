/**
 * CYNIC Judge Module
 *
 * The judgment engine - κυνικός (like a dog)
 *
 * @module @cynic/node/judge
 */

export * from './dimensions.js';
export * from './dimension-registry.js';
export * from './judge.js';
export * from './scorers.js';
export * from './residual.js';
export * from './graph-integration.js';
export * from './learning-service.js';
export * from './learning-manager.js';
export * from './self-skeptic.js';
export * from './engine-integration.js';
export * from './root-cause-analyzer.js';
export * from './feedback-processor.js';
export * from './external-validation.js';
export * from './rca-integration.js';

// Judgment Cards (shareable artifacts)
export * from './judgment-card.js';

// Shannon Entropy for judgment uncertainty
export * from './entropy.js';

// DPO Learning Pipeline (Week 3)
export * from './dpo-processor.js';
export * from './dpo-optimizer.js';
export * from './calibration-tracker.js';
export * from './residual-governance.js';
export * from './learning-scheduler.js';
