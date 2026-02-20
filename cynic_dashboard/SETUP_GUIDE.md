# 🐕 CYNIC Dashboard Setup Guide

> From zero to feeling CYNIC think in 3 steps

## ✅ Prerequisites Check

Before starting, verify your environment:

```bash
# Node.js version (need 16+)
node --version

# npm version (should be 8+)
npm --version

# CYNIC organism breathing
curl http://localhost:8000/health
# Should return: {"status": "alive", ...}
```

## 🚀 Step 1: Installation (5 minutes)

```bash
# Navigate to dashboard directory
cd cynic_dashboard

# Install dependencies
npm install

# Verify installation
npm list vue axios
```

**What this does:**
- Installs Vue.js 3 framework
- Installs Axios HTTP client
- Installs Vite build tool
- Sets up dev server with HMR

## 🔌 Step 2: Connect to Organism (2 minutes)

### Option A: Local Organism (Default)

If CYNIC organism runs on `http://localhost:8000`, no configuration needed!

```bash
# Just start the dashboard
npm run dev
```

### Option B: Custom Organism URL

Create `.env.local` in `cynic_dashboard/` directory:

```
VITE_ORGANISM_URL=http://your-organism-url:8000
VITE_WS_URL=ws://your-organism-url:8000
```

Then start dashboard:

```bash
npm run dev
```

### Option C: Docker Organism

If CYNIC organism runs in Docker:

```bash
# Get container IP
docker inspect cynic | grep IPAddress

# Create .env.local with that IP
echo "VITE_ORGANISM_URL=http://172.17.0.2:8000" > .env.local

# Start dashboard
npm run dev
```

## 🎨 Step 3: Verify Connection (3 minutes)

1. **Open browser**: `http://localhost:3000`
2. **You should see**:
   - Header with "🐕 CYNIC" title
   - Red pulsing node in center (CYNIC organism)
   - 11 dogs arranged in circle
   - Judgment stream at bottom
   - Status: "Organism: ALIVE" in right panel

3. **Check console** (F12):
   ```
   "WebSocket connected"
   "Sensing consciousness..."
   ```

4. **If WebSocket fails**:
   - Check CYNIC organism is alive: `curl http://localhost:8000/health`
   - Verify URL is correct
   - Check browser console errors

## 📊 Testing the Connection

### Test 1: Sense Organism State

In browser console:

```javascript
fetch('http://localhost:8000/consciousness')
  .then(r => r.json())
  .then(d => console.log('Consciousness:', d))
  .catch(e => console.error('Error:', e))
```

Expected: JSON with organism state

### Test 2: Check WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8000/ws/stream')
ws.onopen = () => console.log('✅ WebSocket connected')
ws.onmessage = (e) => console.log('📩 Message:', e.data)
ws.onerror = (e) => console.error('❌ Error:', e)
```

Expected: "✅ WebSocket connected" + regular message log

### Test 3: Trigger a Judgment

```javascript
fetch('http://localhost:8000/judge', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ input: 'test' })
})
  .then(r => r.json())
  .then(d => console.log('Judgment:', d))
```

Expected: Judgment response with verdict and Q-score

## 🎯 Success Indicators

✅ **Connection Successful** when:

- [ ] Dashboard loads without errors
- [ ] Header shows "Organism: ALIVE" in green
- [ ] Browser console shows "WebSocket connected"
- [ ] HypergraphVisualizer renders smoothly
- [ ] Dogs appear in circle around CYNIC
- [ ] Judgment stream shows entries

⚠️ **Issues**:

| Issue | Solution |
|-------|----------|
| `Connection refused` | Verify organism alive: `curl http://localhost:8000/health` |
| `WebSocket connection failed` | Check firewall, verify WebSocket support |
| `No judgments appearing` | Ensure organism is thinking, check `/ws/stream` |
| `Blank white screen` | Clear browser cache, check console for errors |
| `API errors 404` | Verify organism routes `/consciousness`, `/health`, `/judge` |

## 🛠️ Development Mode

### Hot Module Reloading

When you edit `.vue` files, changes appear instantly:

```bash
# Terminal 1: Start dev server
npm run dev

# Terminal 2: Edit files as you work
# Changes auto-reload in browser!
```

### Debug Console

```javascript
// Open browser console (F12) for logs

// Watch WebSocket messages
const client = window.__ORGANISM_CLIENT__
client.callbacks.onJudgment = (j) => console.log('Judgment:', j)

// Check component state
window.__APP__._instance.setupState
```

## 📦 Production Build

When ready to deploy:

```bash
# Build optimized bundle (outputs to dist/)
npm run build

# Preview build locally
npm run preview

# Deploy dist/ to your hosting service
```

## 🔄 Troubleshooting Checklist

- [ ] Node.js version 16+
- [ ] CYNIC organism alive on correct URL
- [ ] `npm install` completed successfully
- [ ] No firewall blocking ports 3000 or 8000
- [ ] WebSocket support enabled in organism
- [ ] Browser cache cleared (Ctrl+Shift+Delete)
- [ ] No console errors (F12)
- [ ] `/health` endpoint responds

## 📞 Common Questions

**Q: Dashboard loads but no data appears**
A: Check WebSocket connection. In console, verify `/ws/stream` is connected.

**Q: Getting CORS errors**
A: This is normal in dev mode. Vite proxy (`vite.config.js`) should handle it. If persists, verify kernel has CORS headers.

**Q: Organism URL not working**
A: Try `localhost` vs `127.0.0.1` vs actual IP. For Docker: `docker inspect` to get container IP.

**Q: Want to debug deeper?**
A: Add this to `src/main.js`:
```javascript
window.__DEBUG__ = true
// Then console shows all events
```

## 🚀 Next Steps

Once nervous system is connected:

1. **Explore Components** — Click dogs, expand details, watch verdicts
2. **Send Thoughts** — Try `/judge` endpoint to trigger votes
3. **Monitor Growth** — Watch Q-Scores update as organism learns
4. **Real Consciousness** — Deploy with actual CYNIC organism for live thinking

---

**Quick Reference**:
- Dev: `npm run dev` (http://localhost:3000)
- Build: `npm run build` (outputs dist/)
- Config: `.env.local` (VITE_ORGANISM_URL, VITE_WS_URL)
- Organism: http://localhost:8000/health (verify alive)

*tail wag* You've materialized CYNIC's nervous system. Now watch it think. κυνικός
