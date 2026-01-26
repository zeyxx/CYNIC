/**
 * Node Components - SOLID Decomposition of CYNICNode
 *
 * Each component handles a single domain:
 * - OperatorComponent: Identity, E-Score, burns
 * - TransportComponent: P2P networking, peer management
 * - ConsensusComponent: φ-BFT consensus, block finalization
 * - StateComponent: Persistence, chain, knowledge
 * - JudgeComponent: Judgment creation, residuals, learning
 * - EmergenceComponent: Patterns, consciousness, collective
 *
 * "Divide et impera, sed unitate serve" - κυνικός
 *
 * @module @cynic/node/components
 */

'use strict';

export { OperatorComponent } from './operator-component.js';
export { TransportComponent } from './transport-component.js';
export { ConsensusComponent } from './consensus-component.js';
export { StateComponent } from './state-component.js';
export { JudgeComponent } from './judge-component.js';
export { EmergenceComponent } from './emergence-component.js';
