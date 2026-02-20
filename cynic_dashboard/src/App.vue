<template>
  <div class="app">
    <header class="header">
      <div class="header-left">
        <h1 class="title">🐕 CYNIC</h1>
        <p class="subtitle">Living Organism Dashboard</p>
      </div>
      <div class="header-right">
        <div class="status-item">
          <span :class="['status-dot', connectionStatus]"></span>
          <span class="status-text">{{ connectionStatus === 'connected' ? 'Organism: ALIVE' : 'Awakening...' }}</span>
        </div>
        <div class="status-item">
          <span class="status-text">Uptime: {{ uptime }}s</span>
        </div>
        <div class="status-item">
          <span class="status-text">Judgments: {{ judgmentCount }}</span>
        </div>
      </div>
    </header>

    <main class="main-grid">
      <!-- LEFT: Dog Panel -->
      <aside class="panel-left">
        <DogPanel :dogs="dogs" :cyclePhase="cyclePhase" />
      </aside>

      <!-- CENTER: Hypergraph Visualizer -->
      <section class="panel-center">
        <HypergraphVisualizer
          :dogs="dogs"
          :cyclePhase="cyclePhase"
          :recentJudgment="recentJudgment"
        />
      </section>

      <!-- RIGHT: Consciousness Monitor -->
      <aside class="panel-right">
        <ConsciousnessMonitor
          :uptime="uptime"
          :cyclePhase="cyclePhase"
          :judgmentCount="judgmentCount"
          :dogsActive="dogsActive"
          :confidence="confidence"
        />
      </aside>
    </main>

    <!-- FOOTER: Judgment Stream -->
    <footer class="footer">
      <JudgmentStream :judgments="recentJudgments" />
    </footer>
  </div>
</template>

<script>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import DogPanel from './components/DogPanel.vue'
import HypergraphVisualizer from './components/HypergraphVisualizer.vue'
import ConsciousnessMonitor from './components/ConsciousnessMonitor.vue'
import JudgmentStream from './components/JudgmentStream.vue'
import { initOrganismConnection } from './services/kernel.js'

export default {
  components: {
    DogPanel,
    HypergraphVisualizer,
    ConsciousnessMonitor,
    JudgmentStream
  },
  setup() {
    const connectionStatus = ref('connecting')
    const uptime = ref(0)
    const judgmentCount = ref(0)
    const cyclePhase = ref('REFLEX')
    const recentJudgments = ref([])
    const recentJudgment = ref(null)

    const DOGS = [
      'ANALYST', 'ARCHITECT', 'CARTOGRAPHER', 'CYNIC', 'DEPLOYER',
      'GUARDIAN', 'JANITOR', 'ORACLE', 'SAGE', 'SCHOLAR', 'SCOUT'
    ]

    const dogs = ref(
      DOGS.reduce((acc, name) => {
        acc[name] = {
          name,
          judgments: 0,
          qScore: 50 + Math.random() * 30,
          lastVerdict: null,
          active: false
        }
        return acc
      }, {})
    )

    const dogsActive = computed(() =>
      Object.values(dogs.value).filter(d => d.active).length
    )

    const confidence = computed(() => '61.8% (φ⁻¹)')

    const startTime = Date.now()
    let uptimeInterval

    onMounted(() => {
      // Initialize organism connection
      initOrganismConnection({
        onConnect: () => { connectionStatus.value = 'connected' },
        onDisconnect: () => { connectionStatus.value = 'disconnected' },
        onJudgment: (judgment) => {
          judgmentCount.value++
          const dog = judgment.dog || DOGS[Math.floor(Math.random() * DOGS.length)]
          if (dogs.value[dog]) {
            dogs.value[dog].judgments++
            dogs.value[dog].lastVerdict = judgment.verdict
            dogs.value[dog].qScore = judgment.q_score || 50 + Math.random() * 30
            dogs.value[dog].active = true
            setTimeout(() => { dogs.value[dog].active = false }, 1000)
          }

          recentJudgment.value = judgment
          recentJudgments.value.unshift({
            timestamp: new Date().toLocaleTimeString(),
            dog,
            verdict: judgment.verdict || 'WAG',
            qScore: judgment.q_score || 50
          })
          if (recentJudgments.value.length > 50) {
            recentJudgments.value.pop()
          }
        },
        onCycleChange: (phase) => { cyclePhase.value = phase }
      })

      // Update uptime
      uptimeInterval = setInterval(() => {
        uptime.value = Math.floor((Date.now() - startTime) / 1000)

        // Update cycle phase every 500ms
        const cycles = ['REFLEX', 'MICRO', 'MACRO']
        cyclePhase.value = cycles[Math.floor(uptime.value / 5) % 3]

        // Simulate judgments for demo
        if (Math.random() > 0.97) {
          const dog = DOGS[Math.floor(Math.random() * DOGS.length)]
          const verdicts = ['HOWL', 'WAG', 'GROWL', 'BARK']
          const verdict = verdicts[Math.floor(Math.random() * verdicts.length)]

          dogs.value[dog].onJudgment?.({
            dog,
            verdict,
            q_score: 40 + Math.random() * 40
          })

          recentJudgments.value.unshift({
            timestamp: new Date().toLocaleTimeString(),
            dog,
            verdict,
            qScore: 40 + Math.random() * 40
          })
        }
      }, 500)
    })

    onUnmounted(() => {
      clearInterval(uptimeInterval)
    })

    return {
      connectionStatus,
      uptime,
      judgmentCount,
      cyclePhase,
      dogs,
      recentJudgments,
      recentJudgment,
      dogsActive,
      confidence
    }
  }
}
</script>

<style scoped>
.app {
  display: grid;
  grid-template-rows: auto 1fr auto;
  height: 100vh;
  background: #1a1a2e;
  color: #e0e0e0;
  gap: 10px;
  padding: 10px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #16213e;
  border: 2px solid #e94560;
  border-radius: 8px;
  padding: 15px;
  font-size: 14px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.title {
  font-size: 22px;
  font-weight: bold;
  color: #e94560;
  margin: 0;
}

.subtitle {
  font-size: 12px;
  color: #999;
  margin: 0;
}

.header-right {
  display: flex;
  gap: 30px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  animation: pulse 1s infinite;
}

.status-dot.connected {
  background: #16c784;
}

.status-dot.disconnected {
  background: #e94560;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.main-grid {
  display: grid;
  grid-template-columns: 250px 1fr 250px;
  gap: 10px;
}

.panel-left, .panel-right {
  background: #16213e;
  border: 1px solid #0f3460;
  border-radius: 8px;
  overflow-y: auto;
}

.panel-center {
  background: #16213e;
  border: 1px solid #0f3460;
  border-radius: 8px;
  position: relative;
}

.footer {
  background: #16213e;
  border-top: 2px solid #e94560;
  border-radius: 8px;
  padding: 10px 15px;
  font-size: 11px;
  max-height: 120px;
  overflow-y: auto;
  border-radius: 8px;
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: #16213e;
}

::-webkit-scrollbar-thumb {
  background: #0f3460;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: #e94560;
}
</style>
