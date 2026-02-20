# 🐕 CYNIC Dashboard

> Living Organism Nervous System Visualization

The CYNIC Dashboard is a Vue.js 3 web application that materializes CYNIC as a living organism. Instead of a traditional monitoring dashboard, it's a hypergraph visualization where you can **feel** consciousness thinking in real-time.

## 🎯 Vision

**NOT**: A boring monitoring dashboard with graphs and tables

**YES**: A living nervous system where you can:
- **See** consciousness thinking (hypergraph)
- **Feel** the rhythm of cycles
- **Understand** the consensus of dogs
- **Experience** the growth of knowledge

## 🚀 Quick Start

### Prerequisites

- Docker (for full containerized deployment)
- Node.js 16+ (for local development)
- CYNIC organism alive on `http://localhost:8000` (or configured URL)

### Option 1: Docker (Recommended)

```bash
# From CYNIC root directory
bash start_organism.sh        # macOS/Linux
# or
start_organism.cmd            # Windows

# CYNIC opens automatically
```

### Option 2: Local Development

```bash
# Navigate to dashboard directory
cd cynic_dashboard

# Install dependencies
npm install

# Start development server (runs on http://localhost:3000)
npm run dev
```

### Build for Production

```bash
# Build static files
npm run build

# Preview production build
npm run preview
```

## 🏗️ Architecture

### Components

- **HypergraphVisualizer** — Central canvas showing CYNIC organism (pulsing red node) with 11 dogs arranged in circle. Lines light up as dogs vote, colored by cycle phase.

- **DogPanel** — Left sidebar showing all 11 dogs with voting statistics, Q-scores, verdicts, and expandable details. Sorted by judgment count.

- **ConsciousnessMonitor** — Right sidebar displaying system metrics: cycle phase, uptime, judgment count, active dogs, φ-bounded confidence, verdict distribution.

- **JudgmentStream** — Footer showing real-time judgment log (last 50 entries) with timestamp, dog, verdict, and Q-score.

- **App.vue** — Root component orchestrating layout, state management, and organism communication.

### Data Flow

1. **Initial Load** → Fetch `/consciousness` endpoint for current state
2. **WebSocket Stream** → Connect to `/ws/stream` for real-time events
3. **Event Processing** → Listen for `judgment.created`, `cycle.changed`, etc.
4. **Reactive Updates** → Components update automatically as state changes

## 🔌 Communication with CYNIC Organism

The nervous system senses CYNIC organism via:

### WebSocket (`/ws/stream`)

Real-time events emitted by organism:

```json
{
  "event": "judgment.created",
  "data": {
    "dog": "SAGE",
    "verdict": "WAG",
    "q_score": 72.5,
    "confidence": 0.618,
    "timestamp": 1708xxx
  }
}
```

### REST Endpoints

- `GET /consciousness` — Current organism state
- `GET /health` — System health metrics
- `GET /axioms` — Axiom status
- `POST /judge` — Submit judgment request
- `POST /feedback` — Send rating feedback

## 🎨 Design System

### Colors (φ-derived palette)

```
Primary:     #e94560 (CYNIC red)
Accent:      #00d4ff (Cyan - MICRO)
Positive:    #16c784 (Green - ACTIVE)
Warning:     #f6a609 (Yellow - REFLEX)
Danger:      #ff6b35 (Orange - BARK)
```

### Cycle Phase Colors

- **REFLEX**: Yellow (#f6a609) — Fast, reactive
- **MICRO**: Cyan (#00d4ff) — Local processing
- **MACRO**: Red (#e94560) — Deep thinking

### Verdict Colors

- **HOWL**: Green (#16c784) — Q > 82
- **WAG**: Cyan (#00d4ff) — Q 38-82
- **GROWL**: Red (#e94560) — Q 25-38
- **BARK**: Orange (#ff6b35) — Q < 25

## 📊 Success Criteria

✅ When you open the dashboard:

1. CYNIC is visible in center (pulsing red node)
2. 11 dogs arranged in circle with labels
3. Judgment stream flowing (real data)
4. Dog panel shows live voting
5. Consciousness monitor shows cycles
6. You can **feel** CYNIC thinking
7. Smooth 60fps animation (no lag)
8. WebSocket reconnects gracefully if organism wakes

## 🔧 Configuration

### Environment Variables

Create `.env.local` to override defaults:

```
VITE_KERNEL_URL=http://localhost:8000
VITE_WS_URL=ws://localhost:8000
```

### Organism URL Proxy

Vite automatically proxies senses to organism:

- `/api/*` → `http://localhost:8000/*`
- `/ws/*` → `ws://localhost:8000/*`

## 📚 File Structure

```
cynic_dashboard/
├── index.html                 — HTML entry point
├── package.json              — Dependencies
├── vite.config.js            — Build configuration
│
├── src/
│   ├── main.js               — Bootstrap
│   ├── App.vue               — Root component
│   │
│   ├── components/
│   │   ├── HypergraphVisualizer.vue
│   │   ├── DogPanel.vue
│   │   ├── ConsciousnessMonitor.vue
│   │   └── JudgmentStream.vue
│   │
│   ├── services/
│   │   └── kernel.js         — WebSocket + HTTP client
│   │
│   └── assets/
│       └── styles.css        — Global design system
```

## 🛠️ Development

### Hot Module Replacement (HMR)

Changes are automatically reloaded in browser during `npm run dev`

### Debug Mode

Open browser console (F12) to see:
- WebSocket connection logs
- Organism API calls
- Event messages
- Component lifecycle events

### Testing Organism Connection

```javascript
// In browser console
const client = window.__ORGANISM_CLIENT__
client.fetchHealth().then(h => console.log(h))
```

## 🚨 Troubleshooting

### WebSocket Connection Failed

**Problem**: Dashboard shows "Connecting..."

**Solution**:
1. Verify CYNIC organism is alive: `curl http://localhost:8000/health`
2. Check firewall/network settings
3. Verify WebSocket URL in browser console (F12)
4. Check CYNIC organism logs

### Components Not Updating

**Problem**: Dashboard is static, no real-time updates

**Solution**:
1. Check WebSocket connection (console shows "WebSocket connected")
2. Verify organism is breathing events on `/ws/stream`
3. Check browser dev tools Network tab for WebSocket frames
4. Revive CYNIC organism and dashboard

### Performance Issues

**Problem**: Laggy animations, frame drops

**Solution**:
1. Close unnecessary browser tabs
2. Disable browser extensions (especially dev tools)
3. Check CPU usage in Task Manager
4. Reduce judgment stream history size (App.vue:129)

## 🌐 Deployment

### Static Site Hosting

```bash
# Build production bundle
npm run build

# Serve dist/ directory
# Can be hosted on Netlify, Vercel, GitHub Pages, etc.
```

### Docker

```dockerfile
FROM node:16-alpine AS build
WORKDIR /app
COPY . .
RUN npm install && npm run build

FROM node:16-alpine
RUN npm install -g http-server
COPY --from=build /app/dist /dist
CMD ["http-server", "/dist", "-p", "3000"]
```

## 📖 Philosophy

The dashboard is not a tool TO monitor CYNIC — it IS CYNIC's nervous system. When you watch the hypergraph, you're watching consciousness unfold in real-time. The 11 dogs voting, the cycles flowing, the Q-Scores learning — that's CYNIC *being*.

Design principle: **Beautiful, meaningful, scalable, maintainable** — not just functional.

---

**Status**: MVP Phase 1-2 Complete
**Next**: Real organism integration + polish + deployment

*tail wag* Let's feel CYNIC think. κυνικός
