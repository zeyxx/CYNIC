/**
 * LV-5: EWC Integration with Catastrophic Forgetting Detection
 *
 * Bridges EWC (prevention) with CatastrophicForgettingTracker (detection).
 *
 * Flow:
 * 1. CatastrophicForgettingTracker detects BWT < threshold
 * 2. Triggers EWC consolidation
 * 3. Protected weights prevent future forgetting
 *
 * @module @cynic/node/judge/ewc-integration
 */

'use strict';

import { EventEmitter } from 'events';

const PHI_INV = 0.618;
const FORGETTING_THRESHOLD = -0.1;

/**
 * Integrates EWC with catastrophic forgetting detection.
 */
export class EWCForgettingIntegration extends EventEmitter {
  constructor({ ewcManager, forgettingTracker, learningService, logger }) {
    super();
    this.ewcManager = ewcManager;
    this.forgettingTracker = forgettingTracker;
    this.learningService = learningService;
    this.logger = logger || console;

    this.interventions = [];
    this.stats = {
      forgettingDetected: 0,
      consolidationsTriggered: 0,
      preventionSuccess: 0,
    };

    // Listen for forgetting detection
    this._setupListeners();
  }

  _setupListeners() {
    // When catastrophic forgetting is detected, trigger EWC consolidation
    this.forgettingTracker.on('catastrophic-forgetting', async (event) => {
      await this._handleForgettingDetected(event);
    });

    // When consolidation happens, verify it reduces forgetting
    this.learningService.on('consolidated', async (event) => {
      await this._verifyConsolidationEffect(event);
    });
  }

  async _handleForgettingDetected(event) {
    this.stats.forgettingDetected++;

    this.logger.warn('[EWC-Integration] Catastrophic forgetting detected:', {
      taskType: event.taskType,
      bwt: event.bwt,
      threshold: FORGETTING_THRESHOLD,
    });

    // Trigger immediate consolidation to prevent further forgetting
    try {
      const consolidation = await this.learningService.consolidateKnowledge(event.taskType);
      
      this.stats.consolidationsTriggered++;

      this.interventions.push({
        timestamp: new Date(),
        trigger: 'catastrophic-forgetting',
        taskType: event.taskType,
        bwt: event.bwt,
        consolidationId: consolidation.consolidationId,
        fisherStats: consolidation.fisherStats,
      });

      this.emit('intervention', {
        type: 'ewc-consolidation',
        reason: 'catastrophic-forgetting-detected',
        taskType: event.taskType,
        bwt: event.bwt,
        consolidation,
      });

    } catch (error) {
      this.logger.error('[EWC-Integration] Consolidation failed:', error);
    }
  }

  async _verifyConsolidationEffect(event) {
    // Wait a bit for new judgments to come in
    setTimeout(async () => {
      const bwt = await this.forgettingTracker.calculateOverallBWT();
      
      if (bwt && bwt.average_bwt > FORGETTING_THRESHOLD) {
        this.stats.preventionSuccess++;
        
        this.logger.info('[EWC-Integration] Consolidation successful - forgetting reduced:', {
          consolidationId: event.consolidationId,
          bwt: bwt.average_bwt,
        });
      }
    }, 5000); // 5 second delay
  }

  /**
   * Check if EWC is protecting important knowledge.
   */
  async getProtectionStatus() {
    const ewcStatus = this.ewcManager.getStatus();
    const bwt = await this.forgettingTracker.calculateOverallBWT();

    return {
      ewc: ewcStatus,
      forgetting: bwt,
      interventions: this.interventions.length,
      stats: this.stats,
      isProtecting: ewcStatus.consolidated && 
                    ewcStatus.fisherStats.critical > 0 &&
                    (!bwt || bwt.average_bwt > FORGETTING_THRESHOLD),
    };
  }

  getStats() {
    return {
      ...this.stats,
      recentInterventions: this.interventions.slice(-10),
    };
  }
}

export default EWCForgettingIntegration;
