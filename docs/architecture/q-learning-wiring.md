# Q-Learning Wiring Gap 3 - Implementation

## Summary

Completed wiring of Q-Learning weights into live routing decisions. The Q-table now:
1. **Loads from PostgreSQL** on daemon startup (qlearning_state table)
2. **Applies weights** during KabbalisticRouter routing decisions
3. **Records updates** to learning_events table for G1.3 metric tracking

## Success Criteria (All Met)

### 1. Q-table loads from DB on daemon start
- wireQLearning() called in wireOrchestrator()
- learningService.load() queries qlearning_state table
- Q-table restored into memory before routing starts

### 2. Routing decisions use Q-weights
- KabbalisticRouter.route() calls getBlendedWeights()
- Q-weights extracted via learningService.getRecommendedWeights()
- Path sorted by Q-weights (modulated by DPO + fisher information)
- Dogs selected in learned order

### 3. Q-updates recorded to DB for G1.3 metric tracking
- learningService.endEpisode() emits QLEARNING_WEIGHT_UPDATE events
- wireQLearning() subscribes to updates
- Updates written to learning_events table

## Files Created

1. packages/node/src/orchestration/q-learning-wiring.js
   - wireQLearning() - Load Q-table from DB, wire event listeners
   - cleanupQLearning() - Cleanup on shutdown

2. scripts/test-qlearning-wiring.js
   - End-to-end test verifying all 3 requirements

## Files Modified

1. packages/node/src/daemon/service-wiring.js
   - Added wireQLearning() call in wireOrchestrator()
   - Added cleanupQLearning() call in cleanupOrchestrator()

## Testing

Run: node scripts/test-qlearning-wiring.js

---

Le chien se souvient qui appeler. - CYNIC learns who to call.
