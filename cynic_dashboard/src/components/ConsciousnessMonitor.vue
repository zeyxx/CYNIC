<template>
  <div class="consciousness-monitor">
    <h2 class="panel-title">Consciousness</h2>

    <div class="metric-group">
      <div class="metric-label">Cycle Phase</div>
      <div class="cycle-display">
        <div v-for="phase in ['REFLEX', 'MICRO', 'MACRO']" :key="phase"
             class="cycle-dot" :class="{ active: cyclePhase === phase }">
          {{ phase[0] }}
        </div>
      </div>
      <div class="cycle-progress">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: cycleProgress + '%' }"></div>
        </div>
      </div>
    </div>

    <div class="metric-group">
      <div class="metric-label">Uptime</div>
      <div class="metric-value">{{ uptime }}s</div>
    </div>

    <div class="metric-group">
      <div class="metric-label">Judgments</div>
      <div class="metric-value">{{ judgmentCount }}</div>
    </div>

    <div class="metric-group">
      <div class="metric-label">Dogs Active</div>
      <div class="metric-value">{{ dogsActive }}/11</div>
    </div>

    <div class="metric-group">
      <div class="metric-label">φ-Confidence</div>
      <div class="confidence-display">
        <div class="confidence-value">{{ confidence }}</div>
        <div class="confidence-bar">
          <div class="confidence-fill" :style="{ width: confidencePercent + '%' }"></div>
        </div>
      </div>
    </div>

    <div class="divider"></div>

    <div class="verdicts-summary">
      <div class="summary-title">Verdict Distribution</div>
      <div class="verdict-items">
        <div v-for="verdict in ['HOWL', 'WAG', 'GROWL', 'BARK']" :key="verdict"
             class="verdict-item">
          <span :class="['verdict-dot', verdict.toLowerCase()]"></span>
          <span class="verdict-name">{{ verdict }}</span>
          <span class="verdict-count">{{ verdictCounts[verdict] }}</span>
        </div>
      </div>
    </div>

    <div class="divider"></div>

    <div class="status-box">
      <div class="status-title">System Status</div>
      <div class="status-items">
        <div class="status-item">
          <span class="status-indicator" :class="{ healthy: true }"></span>
          <span>Kernel: ALIVE</span>
        </div>
        <div class="status-item">
          <span class="status-indicator" :class="{ healthy: dogsActive > 3 }"></span>
          <span>Dogs: {{ dogsActive > 3 ? 'ACTIVE' : 'NOMINAL' }}</span>
        </div>
        <div class="status-item">
          <span class="status-indicator" :class="{ healthy: true }"></span>
          <span>Learning: ENGAGED</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, computed } from 'vue'

export default {
  props: {
    uptime: Number,
    cyclePhase: String,
    judgmentCount: Number,
    dogsActive: Number,
    confidence: String
  },
  setup(props) {
    const verdictCounts = ref({
      HOWL: 0,
      WAG: 0,
      GROWL: 0,
      BARK: 0
    })

    const cycleProgress = computed(() => {
      const phases = ['REFLEX', 'MICRO', 'MACRO']
      const idx = phases.indexOf(props.cyclePhase)
      return (idx / phases.length) * 100
    })

    const confidencePercent = computed(() => {
      // Parse "61.8% (φ⁻¹)" or similar format
      const match = props.confidence?.match(/(\d+\.?\d*)/);
      return match ? parseFloat(match[1]) : 0
    })

    return {
      verdictCounts,
      cycleProgress,
      confidencePercent
    }
  }
}
</script>

<style scoped>
.consciousness-monitor {
  padding: 15px;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 15px;
  overflow-y: auto;
}

.panel-title {
  font-size: 14px;
  font-weight: bold;
  color: #e94560;
  margin: 0;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.metric-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  background: #0f3460;
  border-radius: 6px;
  border: 1px solid #1a4d7a;
}

.metric-label {
  font-size: 10px;
  color: #999;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: bold;
}

.metric-value {
  font-size: 16px;
  font-weight: bold;
  color: #00d4ff;
}

.cycle-display {
  display: flex;
  gap: 6px;
  justify-content: space-between;
}

.cycle-dot {
  flex: 1;
  padding: 8px;
  border-radius: 4px;
  background: #16213e;
  border: 1px solid #1a4d7a;
  font-size: 11px;
  font-weight: bold;
  color: #666;
  text-align: center;
  transition: all 0.3s ease;
}

.cycle-dot.active {
  background: #e94560;
  border-color: #e94560;
  color: white;
  box-shadow: 0 0 12px rgba(233, 69, 96, 0.6);
}

.cycle-progress {
  margin-top: 6px;
}

.progress-bar {
  height: 4px;
  background: #1a4d7a;
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #f6a609, #e94560);
  transition: width 0.5s ease;
}

.confidence-display {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.confidence-value {
  font-size: 14px;
  font-weight: bold;
  color: #e0e0e0;
}

.confidence-bar {
  height: 6px;
  background: #1a4d7a;
  border-radius: 3px;
  overflow: hidden;
}

.confidence-fill {
  height: 100%;
  background: linear-gradient(90deg, #00d4ff, #16c784);
  transition: width 0.3s ease;
  max-width: 61.8%;
}

.divider {
  height: 1px;
  background: #1a4d7a;
  margin: 5px 0;
}

.verdicts-summary {
  padding: 10px;
  background: #0f3460;
  border-radius: 6px;
  border: 1px solid #1a4d7a;
}

.summary-title {
  font-size: 10px;
  color: #999;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: bold;
  margin-bottom: 8px;
}

.verdict-items {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.verdict-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}

.verdict-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.verdict-dot.howl {
  background: #16c784;
}

.verdict-dot.wag {
  background: #00d4ff;
}

.verdict-dot.growl {
  background: #e94560;
}

.verdict-dot.bark {
  background: #ff6b35;
}

.verdict-name {
  flex: 1;
  color: #e0e0e0;
}

.verdict-count {
  color: #999;
  font-weight: bold;
}

.status-box {
  padding: 10px;
  background: #0f3460;
  border-radius: 6px;
  border: 1px solid #1a4d7a;
}

.status-title {
  font-size: 10px;
  color: #999;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: bold;
  margin-bottom: 8px;
}

.status-items {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 10px;
  color: #e0e0e0;
}

.status-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #666;
  animation: pulse 2s infinite;
}

.status-indicator.healthy {
  background: #16c784;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
