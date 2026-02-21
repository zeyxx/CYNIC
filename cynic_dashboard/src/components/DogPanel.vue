<template>
  <div class="dog-panel">
    <h2 class="panel-title">11 Dogs</h2>
    <div class="dogs-list">
      <div v-for="(dog, name) in sortedDogs" :key="name" class="dog-item" :class="{ active: dog.active }">
        <div class="dog-header" @click="toggleDog(name)">
          <span class="dog-icon">🐕</span>
          <span class="dog-name">{{ name }}</span>
          <span class="dog-count">{{ dog.judgments }}</span>
        </div>
        <div v-if="expandedDog === name" class="dog-details">
          <div class="detail-row">
            <span class="detail-label">Q-Score:</span>
            <span class="detail-value">{{ dog.qScore.toFixed(1) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Last Verdict:</span>
            <span :class="['verdict-badge', dog.lastVerdict?.toLowerCase()]">{{ dog.lastVerdict || 'N/A' }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Status:</span>
            <span :class="['status-badge', dog.active ? 'active' : 'dormant']">
              {{ dog.active ? 'ACTIVE' : 'Dormant' }}
            </span>
          </div>
          <div class="score-bar">
            <div class="score-fill" :style="{ width: (dog.qScore / 100) * 100 + '%' }"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, computed } from 'vue'

export default {
  props: {
    dogs: Object,
    cyclePhase: String
  },
  setup(props) {
    const expandedDog = ref(null)

    const sortedDogs = computed(() => {
      if (!props.dogs) return {}
      return Object.entries(props.dogs)
        .sort((a, b) => b[1].judgments - a[1].judgments)
        .reduce((acc, [name, dog]) => {
          acc[name] = dog
          return acc
        }, {})
    })

    function toggleDog(name) {
      expandedDog.value = expandedDog.value === name ? null : name
    }

    return { expandedDog, sortedDogs, toggleDog }
  }
}
</script>

<style scoped>
.dog-panel {
  padding: 15px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.panel-title {
  font-size: 14px;
  font-weight: bold;
  color: #e94560;
  margin: 0 0 15px 0;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.dogs-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dog-item {
  background: #0f3460;
  border: 1px solid #1a4d7a;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.2s ease;
}

.dog-item:hover {
  border-color: #00d4ff;
  box-shadow: 0 0 8px rgba(0, 212, 255, 0.3);
}

.dog-item.active {
  border-color: #16c784;
  background: #1a5d3a;
  box-shadow: 0 0 12px rgba(22, 199, 132, 0.4);
}

.dog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  background: #16213e;
  font-weight: bold;
}

.dog-icon {
  font-size: 16px;
}

.dog-name {
  flex: 1;
  font-size: 12px;
  color: #e0e0e0;
}

.dog-count {
  font-size: 11px;
  color: #00d4ff;
  background: rgba(0, 212, 255, 0.2);
  padding: 2px 6px;
  border-radius: 3px;
}

.dog-details {
  padding: 10px;
  background: #0f3460;
  border-top: 1px solid #1a4d7a;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 10px;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.detail-label {
  color: #999;
  font-weight: bold;
}

.detail-value {
  color: #e0e0e0;
}

.verdict-badge {
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 9px;
  font-weight: bold;
  text-transform: uppercase;
}

.verdict-badge.howl {
  background: rgba(22, 199, 132, 0.3);
  color: #16c784;
}

.verdict-badge.wag {
  background: rgba(0, 212, 255, 0.3);
  color: #00d4ff;
}

.verdict-badge.growl {
  background: rgba(233, 69, 96, 0.3);
  color: #e94560;
}

.verdict-badge.bark {
  background: rgba(255, 107, 53, 0.3);
  color: #ff6b35;
}

.status-badge {
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 9px;
  font-weight: bold;
}

.status-badge.active {
  background: rgba(22, 199, 132, 0.3);
  color: #16c784;
}

.status-badge.dormant {
  background: rgba(100, 100, 100, 0.3);
  color: #999;
}

.score-bar {
  height: 4px;
  background: #1a4d7a;
  border-radius: 2px;
  overflow: hidden;
  margin-top: 4px;
}

.score-fill {
  height: 100%;
  background: linear-gradient(90deg, #00d4ff, #16c784);
  transition: width 0.3s ease;
}
</style>
