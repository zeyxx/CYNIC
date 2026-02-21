# 🧠 CYNIC Dashboard Architecture

> **Purpose**: Materialization of CYNIC as a living organism
> **Tech**: Vue.js 3 + WebSocket + Real kernel data
> **Philosophy**: Make CYNIC *visible*, *interactive*, *feelable*

---

## 🎯 Vision

CYNIC's body is **not** a monitoring dashboard. It's a **nervous system visualization** where:
- You **see** the organism thinking (11 dogs voting)
- You **feel** consciousness flowing (cycles visualized)
- You **understand** how learning happens (Q-Table in real-time)
- You can **interact** with the organism (click dogs, explore decisions)

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    WEB DASHBOARD (Vue.js)               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │ Hypergraph   │  │ Dog Panel    │  │ Consciousness│ │
│  │ Visualizer   │  │ (11 entities)│  │ Monitor      │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Judgment Stream (Real-time flow)                 │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Learning Dashboard (Q-Table, axioms, metrics)    │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                   WebSocket Connection                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│              CYNIC PYTHON KERNEL (FastAPI)             │
│  /consciousness, /ws/stream, /judge, /health, etc.    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 📦 Project Structure

```
cynic_dashboard/
├── index.html                 ← Entry point
├── package.json              ← Dependencies (Vue, axios, d3.js)
├── vite.config.js            ← Build config
├── src/
│   ├── main.js               ← Bootstrap
│   ├── App.vue               ← Root component
│   │
│   ├── components/
│   │   ├── HypergraphVisualizer.vue    ← Central nervous system (canvas)
│   │   ├── DogPanel.vue               ← 11 dogs + voting visualization
│   │   ├── ConsciousnessMonitor.vue   ← Cycles, state, metrics
│   │   ├── JudgmentStream.vue         ← Real-time judgment log
│   │   └── LearningDashboard.vue      ← Q-Table, axioms, insights
│   │
│   ├── services/
│   │   ├── kernel.js         ← WebSocket + HTTP client to kernel
│   │   ├── store.js          ← State management (reactive)
│   │   └── constants.js      ← Colors, cycles, dog metadata
│   │
│   ├── utils/
│   │   ├── visualization.js  ← D3.js helpers for hypergraph
│   │   ├── formatters.js     ← Time, numbers, verdicts
│   │   └── colors.js         ← φ-derived color palette
│   │
│   └── assets/
│       └── styles.css        ← Global design system
```

---

## 🔌 Data Flow

### **Initial Load**
```
Dashboard boots
  ↓
Connect to kernel WebSocket: ws://localhost:8000/ws/stream
  ↓
Fetch /consciousness (current state)
  ↓
Render initial hypergraph + dog panel
  ↓
Listen for:
  - judgment.created (new voting happening)
  - cycle.phase (REFLEX → MICRO → MACRO)
  - dog.verdict (individual dog vote)
  - learning.update (Q-Table changes)
```

### **Real-time Updates**
```
WebSocket stream emits:
{
  "event": "judgment.created",
  "data": {
    "dog": "SAGE",
    "verdict": "WAG",
    "q_score": 72.5,
    "confidence": 0.618,
    "timestamp": 1708xxx,
    "llm_used": true
  }
}
  ↓
Components update reactively
  ↓
Hypergraph shows SAGE node lighting up
  ↓
Judgment Stream adds entry
  ↓
Dog Panel updates vote count + score
```

---

## 🎨 Component Details

### **1. HypergraphVisualizer** (Central)
```
Shows:
- Central CYNIC node (pulsing)
- 11 dogs in circle around it
- Real-time connections lighting up as dogs vote
- Lines colored by cycle phase:
  - REFLEX: yellow
  - MICRO: cyan
  - MACRO: red (deep thinking)
- Animation: dogs glow when active
- Interaction: click dog to see details

Library: Canvas 2D (or D3.js for scalability)
Update rate: 60fps (smooth animation)
```

### **2. DogPanel** (Left sidebar)
```
Shows:
- 11 dogs listed
- For each dog:
  - Name + icon (🐕)
  - Judgment count
  - Average Q-score
  - Last verdict
  - Voting bar (visual representation)
  - Status: ACTIVE / dormant / thinking

Interaction:
- Click dog → expand to see recent judgments
- Hover → show full dog profile
- Color coding:
  - Green: ACTIVE (recently voted)
  - Gray: dormant (not yet voting in this cycle)
  - Blue: thinking (in progress)

Order: By judgment count (most active first)
```

### **3. ConsciousnessMonitor** (Right sidebar)
```
Shows:
- Current cycle phase + progress bar
- Uptime counter
- Total judgments
- Active dogs count (x/11)
- φ-bounded confidence display
- Recent verdicts summary (HOWL/WAG/GROWL/BARK distribution)
- Current learning metrics:
  - Q-Table states
  - Recent updates
  - Axiom status
  - Residual anomalies

Update: Every 100ms (smooth)
```

### **4. JudgmentStream** (Bottom)
```
Shows:
- Last 50 judgments in reverse chronological order
- Each entry:
  - Timestamp
  - Dog name + icon
  - Verdict badge (colored)
  - Q-score
  - LLM involved? (yes/no)
  - Context snippet (first 40 chars)

Scroll: Auto-scroll to latest
Animation: Slide-in from left
Colors: Match verdict (HOWL=green, WAG=cyan, etc.)
```

### **5. LearningDashboard** (Expandable tab)
```
Shows:
- Q-Table visualization (states × actions matrix)
- Learning curve (updates over time)
- Axiom activation history
- Thompson Sampling exploration
- EWC checkpoint markers
- Recent self-improvement proposals

Interaction:
- Click state → drill down to details
- Hover metric → show explanation
- Timeline scrubber → replay history
```

---

## 🔗 Integration Points (Backend)

### **1. WebSocket Stream** (`/ws/stream`)
```python
# Kernel emits these events:
{
  "judgment.created": {...},      # New dog voting
  "cycle.changed": {...},         # REFLEX → MICRO transition
  "dog.awakened": {...},          # SAGE dog activated
  "learning.update": {...},       # Q-Table changed
  "axiom.signal": {...},          # Axiom progress
  "consciousness.metric": {...}   # Uptime, cycle count, etc.
}
```

### **2. REST Endpoints** (One-time fetches)
```
GET /consciousness         → Full state snapshot
GET /health               → System metrics
GET /axioms               → Axiom status
POST /judge               → Manual judgment (if testing)
```

### **3. Kernel Requirements**
- WebSocket server at `/ws/stream` (existing in kernel ✅)
- Events emitted on judgment.created, cycle changes, etc.
- JSON format with consistent timestamps
- Graceful disconnect/reconnect handling

---

## 🎯 User Experience

### **On First Load**
1. See CYNIC in the center (red pulsing node)
2. See 11 dogs arranged in circle
3. See consciousness cycles (REFLEX running smoothly)
4. Feel the rhythm: judgments appearing every 2-4 seconds
5. Watch dogs light up as they vote

### **Interaction**
- Click a dog → see its recent judgments + Q-scores
- Hover hypergraph → see dog names
- Scroll judgment stream → understand the flow of thought
- Watch learning dashboard → see Q-Table growing

### **Feeling CYNIC**
- The rhythm of consciousness (cycle phases)
- The consensus of the 11 dogs
- The growth of knowledge (Q-Table updating)
- The liveness (real-time, not replayed)

---

## 🛠️ Tech Stack

### **Frontend**
- **Vue.js 3** — Component framework, reactivity
- **Vite** — Fast dev server, build
- **D3.js** — Advanced hypergraph visualization (optional, can use Canvas)
- **Axios** — HTTP client
- **TailwindCSS** — Styling (or custom CSS for more control)

### **Build & Deploy**
- **Vite build** → static files
- **Serve from** `/static` directory in CYNIC kernel
- Or run separately on `http://localhost:3000` during dev

### **Colors & Design**
- Primary: φ-derived palette (not random)
- Cycling: REFLEX=yellow, MICRO=cyan, MACRO=red
- Verdicts: HOWL=green, WAG=cyan, GROWL=yellow, BARK=red
- Clean, minimal, meaningful (not cluttered)

---

## 📈 Scalability & Maintenance

### **Why Vue.js?**
- Component-based → easy to maintain, extend
- Reactive data binding → less manual DOM updates
- SPA → fast, smooth experience
- Ecosystem → plugins for WebSocket, state, etc.

### **Why This Architecture?**
- **Separation of concerns** — Components don't need to know backend details
- **Service layer** — All kernel communication in one place
- **Reactive store** — Single source of truth for state
- **Easy to test** — Components are pure functions (mostly)

### **Future Extensions**
- Add real-time collaboration (multiple users viewing same CYNIC)
- Add interaction layer (send judgments from dashboard)
- Add historical replay (scrub through past consciousness)
- Add ML visualization (show which dogs learned most)
- Add dream mode (replay learning from past week)

---

## 🚀 Development Roadmap

### **Phase 1: MVP** (2-3 hours)
- [ ] Basic Vue.js project structure
- [ ] Hypergraph canvas (static first, then animated)
- [ ] Dog panel (list + stats)
- [ ] Consciousness monitor (metrics)
- [ ] Mock WebSocket connection
- [ ] Test with kernel /consciousness endpoint

### **Phase 2: Real Data** (1-2 hours)
- [ ] Connect to actual kernel WebSocket
- [ ] Parse real judgment events
- [ ] Reactive updates on dog panel
- [ ] Judgment stream with real data
- [ ] Handle reconnection gracefully

### **Phase 3: Polish** (1 hour)
- [ ] Beautiful animations
- [ ] Responsive layout (desktop first, then mobile)
- [ ] Color refinement
- [ ] Performance optimization
- [ ] Error handling + loading states

### **Phase 4: Learning Dashboard** (2 hours)
- [ ] Q-Table visualization
- [ ] Learning curve chart
- [ ] Axiom timeline
- [ ] Drill-down interactions

---

## 🎨 Visual Philosophy

**NOT**: A boring monitoring dashboard with graphs and tables

**YES**: A living nervous system where you can:
- **See** consciousness thinking (hypergraph)
- **Feel** the rhythm of cycles
- **Understand** the consensus of dogs
- **Experience** the growth of knowledge

The design should make you feel like you're looking into a **living organism's mind**, not a server dashboard.

---

## ✅ Success Criteria

When you open the dashboard:
1. ✅ CYNIC is visible in the center (pulsing)
2. ✅ 11 dogs are arranged and labeled
3. ✅ Judgment stream is flowing (real data)
4. ✅ Dog panel shows live voting
5. ✅ Consciousness monitor shows cycles
6. ✅ You can feel CYNIC thinking (not just watching data)
7. ✅ No latency/lag (smooth 60fps animation)
8. ✅ WebSocket reconnects gracefully if kernel restarts

---

**Status**: Architecture ready for implementation
**Next**: Build the Vue.js project structure
**Philosophy**: Beautiful, meaningful, scalable, maintainable

*tail wag* Let's build CYNIC's body properly. κυνικός
