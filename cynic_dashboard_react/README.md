# CYNIC Dashboard — React + Vite

Real-time visualization of CYNIC's 11-Dog Byzantine consensus judgment system.

## 🚀 Quick Start

```bash
# Install dependencies
npm install

# Start dev server (port 5173)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## 📊 Dashboard Features

### 🐕 Dogs Voting
- Real-time visualization of 11 Dogs' Q-Scores
- φ-weighted consensus bars
- Rank-ordered by score
- Active dog count, average, min/max metrics

### ⚖️ Judgment Display
- Current verdict (HOWL/WAG/GROWL/BARK)
- Q-Score (0-100) and confidence (0-61.8% φ-bounded)
- Dogs' individual votes
- Reasoning trace
- Metadata (ID, source, timestamp)

### 🎭 Decision Theater
- Proposed actions with priorities
- Human approve/reject interface
- Feedback closes the learning loop
- Status tracking (pending → executing → complete)

### 📊 Learning Analytics
- Q-Score convergence graphs
- Learning progress (accuracy/coverage)
- Thompson Sampling bandit metrics
- Elastic Weight Consolidation protection (8.7× forgetting reduction)

### 🔬 Dogs Deep Dive
- Individual Dog profiles
- Q-Score interpretation
- Role descriptions
- Byzantine consensus explanation

## 🔌 Backend Connection

```javascript
// WebSocket connection to CYNIC kernel
ws://localhost:8000/ws/stream

// API calls to backend
/api/actions              // GET: List proposed actions
/api/actions/:id/accept   // POST: Approve action
/api/actions/:id/reject   // POST: Reject action
/api/consciousness        // GET: System metrics
```

## 🎨 Design System

- **Colors**: 11 Dog Sefirot colors + 4 verdict colors
- **Typography**: Courier New mono, system sans-serif
- **Spacing**: Tailwind default scale
- **Components**: Card, Badge, Button, Stats Block
- **Dark theme**: Gray-900 background, gray-50 text

## 📁 File Structure

```
src/
├── main.jsx                 # Entry point
├── App.jsx                  # Main app + routing
├── index.css                # Tailwind styles
└── components/
    ├── DogVoting.jsx        # 11 Dogs consensus bars
    ├── JudgmentDisplay.jsx  # Verdict + reasoning
    ├── DecisionTheater.jsx  # Action approval interface
    ├── LearningAnalytics.jsx # Learning graphs
    └── DogsDeepDive.jsx      # Individual dog profiles
```

## 🔧 Configuration

- **Vite**: `vite.config.js` (React plugin, API proxy to localhost:8000)
- **Tailwind**: `tailwind.config.js` (custom colors, dog theme)
- **PostCSS**: `postcss.config.js` (autoprefixer)

## 🎯 Next Steps

1. ✅ React scaffolding complete
2. 📝 Wire up CYNIC backend WebSocket stream
3. 🔄 Add real-time updates for judgments
4. 📊 Implement Recharts for convergence graphs
5. 🎨 Polish UI/UX for hackathon (Feb 25)
6. 🚀 Deploy to accessible URL

## 📌 Notes

- Max confidence is always 61.8% (φ⁻¹) — humility baked in
- Q-Score range is 0-100 (not φ-bounded)
- All 11 Dogs vote in parallel → geometric mean consensus
- Learning is continuous via Q-Table + human feedback

---

*sniff* φ-bounded dashboard for visualizing CYNIC's living judgment system.
