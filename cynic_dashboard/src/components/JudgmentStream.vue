<template>
  <div class="judgment-stream">
    <h3 class="stream-title">Judgment Stream</h3>
    <div class="stream-entries">
      <transition-group name="slide" tag="div">
        <div v-for="(judgment, idx) in judgments" :key="idx" class="judgment-entry">
          <span class="entry-timestamp">{{ judgment.timestamp }}</span>
          <span class="entry-dog">🐕 {{ judgment.dog }}</span>
          <span :class="['entry-verdict', judgment.verdict.toLowerCase()]">
            {{ judgment.verdict }}
          </span>
          <span class="entry-score">Q{{ judgment.qScore.toFixed(0) }}</span>
        </div>
      </transition-group>
    </div>
  </div>
</template>

<script>
export default {
  props: {
    judgments: Array
  }
}
</script>

<style scoped>
.judgment-stream {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 10px;
}

.stream-title {
  font-size: 12px;
  font-weight: bold;
  color: #e94560;
  margin: 0;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.stream-entries {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 10px;
}

.judgment-entry {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  background: rgba(22, 199, 132, 0.05);
  border-left: 2px solid #16c784;
  border-radius: 3px;
  animation: slideIn 0.3s ease-out;
}

.entry-timestamp {
  color: #999;
  font-family: monospace;
  min-width: 60px;
}

.entry-dog {
  color: #00d4ff;
  font-weight: bold;
  min-width: 80px;
}

.entry-verdict {
  padding: 2px 6px;
  border-radius: 3px;
  font-weight: bold;
  font-size: 9px;
  text-transform: uppercase;
}

.entry-verdict.howl {
  background: rgba(22, 199, 132, 0.3);
  color: #16c784;
}

.entry-verdict.wag {
  background: rgba(0, 212, 255, 0.3);
  color: #00d4ff;
}

.entry-verdict.growl {
  background: rgba(233, 69, 96, 0.3);
  color: #e94560;
}

.entry-verdict.bark {
  background: rgba(255, 107, 53, 0.3);
  color: #ff6b35;
}

.entry-score {
  margin-left: auto;
  color: #e0e0e0;
  font-family: monospace;
  font-weight: bold;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(-20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.slide-enter-active {
  transition: all 0.3s ease;
}

.slide-leave-active {
  transition: all 0.3s ease;
}

.slide-enter-from {
  opacity: 0;
  transform: translateX(-20px);
}

.slide-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #0f3460;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #00d4ff;
}
</style>
