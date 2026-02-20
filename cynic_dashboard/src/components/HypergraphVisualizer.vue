<template>
  <div class="visualizer">
    <canvas ref="canvas"></canvas>
  </div>
</template>

<script>
import { ref, watch, onMounted } from 'vue'

export default {
  props: {
    dogs: Object,
    cyclePhase: String,
    recentJudgment: Object
  },
  setup(props) {
    const canvas = ref(null)
    let animationId = null

    const DOGS = [
      'ANALYST', 'ARCHITECT', 'CARTOGRAPHER', 'CYNIC', 'DEPLOYER',
      'GUARDIAN', 'JANITOR', 'ORACLE', 'SAGE', 'SCHOLAR', 'SCOUT'
    ]

    const cycleColors = {
      REFLEX: '#f6a609',
      MICRO: '#00d4ff',
      MACRO: '#e94560'
    }

    function drawHypergraph() {
      if (!canvas.value) return

      const ctx = canvas.value.getContext('2d')
      const w = canvas.value.width
      const h = canvas.value.height

      ctx.clearRect(0, 0, w, h)
      ctx.fillStyle = '#16213e'
      ctx.fillRect(0, 0, w, h)

      const centerX = w / 2
      const centerY = h / 2
      const radius = 40

      // Draw central CYNIC node (pulsing)
      const pulse = Math.sin(Date.now() / 500) * 5 + 40
      ctx.fillStyle = '#e94560'
      ctx.beginPath()
      ctx.arc(centerX, centerY, pulse, 0, Math.PI * 2)
      ctx.fill()

      ctx.fillStyle = '#1a1a2e'
      ctx.font = 'bold 14px monospace'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      ctx.fillText('CYNIC', centerX, centerY)

      // Draw dogs in circle
      DOGS.forEach((dogName, i) => {
        const angle = (i / DOGS.length) * Math.PI * 2 - Math.PI / 2
        const x = centerX + Math.cos(angle) * 200
        const y = centerY + Math.sin(angle) * 200

        const dog = props.dogs[dogName]
        const isActive = dog?.active

        // Connection line
        ctx.strokeStyle = isActive ? '#16c784' : cycleColors[props.cyclePhase] || '#0f3460'
        ctx.lineWidth = isActive ? 2.5 : 1
        ctx.beginPath()
        ctx.moveTo(centerX, centerY)
        ctx.lineTo(x, y)
        ctx.stroke()

        // Dog node
        const nodeColor = isActive ? '#16c784' : '#00d4ff'
        ctx.fillStyle = nodeColor
        ctx.beginPath()
        ctx.arc(x, y, 18, 0, Math.PI * 2)
        ctx.fill()

        // Dog label
        ctx.fillStyle = '#1a1a2e'
        ctx.font = 'bold 9px monospace'
        ctx.fillText(dogName.substring(0, 3), x, y)

        // Active indicator
        if (isActive) {
          ctx.strokeStyle = '#16c784'
          ctx.lineWidth = 2
          ctx.globalAlpha = 0.6
          ctx.beginPath()
          ctx.arc(x, y, 28, 0, Math.PI * 2)
          ctx.stroke()
          ctx.globalAlpha = 1
        }
      })
    }

    function animate() {
      drawHypergraph()
      animationId = requestAnimationFrame(animate)
    }

    onMounted(() => {
      if (canvas.value) {
        canvas.value.width = canvas.value.offsetWidth
        canvas.value.height = canvas.value.offsetHeight
        animate()

        window.addEventListener('resize', () => {
          canvas.value.width = canvas.value.offsetWidth
          canvas.value.height = canvas.value.offsetHeight
        })
      }
    })

    watch(() => props.cyclePhase, () => {
      if (canvas.value) {
        canvas.value.width = canvas.value.offsetWidth
      }
    })

    return { canvas }
  }
}
</script>

<style scoped>
.visualizer {
  width: 100%;
  height: 100%;
  position: relative;
}

canvas {
  display: block;
  width: 100%;
  height: 100%;
}
</style>
