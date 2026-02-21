# CYNIC Dashboard Setup & Running Guide

## Prerequisites

- **Node.js**: v16+ (includes npm)
- **CYNIC Kernel**: Running on `localhost:8000` (see `../cynic/`)
- **Port 5173**: Available for Vite dev server

## 1. Install Dependencies

```bash
cd cynic_dashboard_react
npm install
```

This installs:
- **react** 18.2.0 — UI framework
- **vite** 5.0.0 — build tool & dev server
- **tailwindcss** 3.3.6 — styling
- **recharts** 2.10.0 — charting library
- **axios** 1.6.0 — HTTP client
- **ws** 8.14.0 — WebSocket support

## 2. Configure Backend URL

Create `.env` file from template:

```bash
cp .env.example .env
```

Edit `.env` if CYNIC kernel is not on localhost:8000:

```env
VITE_CYNIC_URL=http://localhost:8000
VITE_CYNIC_WS_URL=ws://localhost:8000
```

## 3. Start Development Server

```bash
npm run dev
```

Output:
```
  VITE v5.0.0  ready in 245 ms

  ➜  Local:   http://localhost:5173/
  ➜  press h to show help
```

Navigate to `http://localhost:5173/` in browser.

## 4. Verify Connection

Look for:
- **🟢 Connected** indicator in header (top-right)
- WebSocket messages in browser console (DevTools → Console)
- Real-time dog voting updates (should update as kernel sends JUDGMENT_CREATED events)

If **🔴 Disconnected**:
1. Verify CYNIC kernel running: `curl http://localhost:8000/health`
2. Check browser console for connection errors
3. Ensure no other service on port 5173

## 5. Build for Production

```bash
npm run build
```

Outputs to `dist/` directory — ready for deployment.

```bash
npm run preview
```

Preview production build locally.

## 🎯 What to Expect

### Initial State
- Empty dog bars (no judgments yet)
- "Awaiting first judgment..." in Judgment tab
- "No proposed actions" in Decision Theater

### After Kernel Emits JUDGMENT_CREATED
- **Dogs Voting**: 11 bars appear with Q-Scores, sorted highest → lowest
- **Judgment Display**: Shows verdict (HOWL/WAG/GROWL/BARK), Q-Score, confidence, reasoning
- **Decision Theater**: Shows any proposed actions for human approval
- **Learning Analytics**: Convergence graphs (mock data until endpoints wired)
- **Deep Dive**: Individual Dog profiles with scores

### Real-Time Updates
- WebSocket receives events every time kernel judges something
- Dogs update in real-time (no page refresh needed)
- Ping/pong keepalive every 30 seconds (proves connection alive)

## 🔌 Backend Integration Status

### ✅ Implemented (Working)
- `ws://localhost:8000/ws/stream` — Real-time event streaming (JUDGMENT_CREATED, LEARNING_EVENT, etc.)
- `GET /consciousness` — System metrics (fallback mock if 404)

### 🟡 Stubbed (Mock Data)
- `GET /actions` — List proposed actions (mock data if 404)
- `POST /actions/{id}/accept` — Approve action (UI-only fallback)
- `POST /actions/{id}/reject` — Reject action (UI-only fallback)

### 🔴 Not Yet Wired
- Individual dog endpoint (using mock Dogs list)
- Learning progression data (using generated mock curves)
- Action execution feedback

## 📝 Environment Variables

```env
# Backend URLs
VITE_CYNIC_URL=http://localhost:8000
VITE_CYNIC_WS_URL=ws://localhost:8000

# API timeout (milliseconds)
VITE_API_TIMEOUT=30000

# Development
VITE_DEV_PORT=5173
VITE_DEBUG=false
```

## 🛠️ Development Tips

### Hot Module Replacement (HMR)
Saves to file → browser auto-updates. Works for:
- Component changes
- CSS changes
- Import updates

### Console Debugging
```javascript
// In browser DevTools Console
// Watch WebSocket messages
ws.onmessage = (e) => console.log('WS:', JSON.parse(e.data))

// Test backend connectivity
fetch('http://localhost:8000/health')
  .then(r => r.json())
  .then(console.log)
```

### Tailwind Styling
- Use `@apply` directive in CSS for component styles
- Color system supports dog names (e.g., `text-sage`, `bg-guardian`)
- Custom colors defined in `tailwind.config.js`

## 📦 Project Structure

```
cynic_dashboard_react/
├── index.html              # Entry point (contains #root)
├── package.json            # Dependencies + scripts
├── vite.config.js          # Build config (React plugin, proxy)
├── tailwind.config.js      # Tailwind colors + theme
├── postcss.config.js       # CSS processing
├── .gitignore              # Ignore node_modules, dist, .env
├── README.md               # Feature overview
├── SETUP.md                # This file
├── .env.example            # Example environment
├── src/
│   ├── main.jsx            # React entry
│   ├── App.jsx             # Main app + routing
│   ├── index.css           # Global styles + components
│   ├── api/
│   │   └── client.js       # Axios client with interceptors
│   └── components/
│       ├── DogVoting.jsx   # 11 Dogs consensus bars
│       ├── JudgmentDisplay.jsx  # Verdict + score
│       ├── DecisionTheater.jsx  # Action approval
│       ├── LearningAnalytics.jsx # Graphs + metrics
│       └── DogsDeepDive.jsx     # Dog profiles
└── dist/                   # Production build (after `npm run build`)
```

## 🚀 Deployment

### Local Staging
```bash
npm run build
npm run preview
# Open http://localhost:4173
```

### Render Cloud (example)
```bash
# 1. Push to GitHub
git push origin main

# 2. Create static site on Render with:
# - Build command: npm install && npm run build
# - Publish directory: dist
# - Environment: VITE_CYNIC_URL=<your-cynic-url>
```

## ❓ Troubleshooting

### "Cannot find module 'recharts'"
```bash
npm install recharts
```

### WebSocket fails to connect
- Check CYNIC kernel running: `curl http://localhost:8000/health`
- Check browser console for CORS errors
- Verify VITE_CYNIC_WS_URL is correct in .env

### Port 5173 already in use
```bash
# Find & kill process
lsof -ti:5173 | xargs kill -9

# Or use different port
npm run dev -- --port 5174
```

### Build fails with Tailwind errors
```bash
# Clear cache
rm -rf node_modules/.cache
npm run build
```

## 📞 Support

See main README.md for CYNIC architecture questions.
For React/Vite issues, check:
- [Vite Docs](https://vitejs.dev/)
- [React Docs](https://react.dev/)
- [Tailwind Docs](https://tailwindcss.com/)

---

*sniff* Dashboard ready for real-time judgment visualization. Let's go.
