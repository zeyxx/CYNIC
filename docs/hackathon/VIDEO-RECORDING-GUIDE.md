# Video Recording Guide — CYNIC Colosseum Demo

## Prerequisites
- `ffmpeg` (for local recording) or use OBS/ScreenFlow
- Terminal + curl access to kernel
- Kernel running and responding to `/judge`

## Quick Verification (before recording)

```bash
# 1. Verify kernel is live
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<TAILSCALE_CORE>:3030/health | jq .

# 2. Get latest verdict (for reference)
curl -H "Authorization: Bearer $CYNIC_API_KEY" \
  http://<TAILSCALE_CORE>:3030/verdicts?limit=1 | jq .
```

## Recording Workflow

### Option 1: OBS / ScreenFlow (Recommended)
1. Open terminal + browser side-by-side
2. **Scene 1 (15s):** Show curl command
   ```bash
   curl -X POST -H "Authorization: Bearer $CYNIC_API_KEY" \
     -H "Content-Type: application/json" \
     -d '{"content":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","domain":"token-analysis"}' \
     http://<TAILSCALE_CORE>:3030/judge | jq .
   ```
   Narrate: "We submit USDC to CYNIC for judgment across six axioms."

3. **Scene 2 (45s):** Show kernel logs in real-time
   ```bash
   journalctl --user -u cynic-kernel -f -n 50
   ```
   Watch Dogs respond. Narrate: "Five independent Dogs — qwen-7b, qwen35-gpu, gemma, deterministic, gemini — score simultaneously on Fidelity, Phi, Verify, Culture, Burn, Sovereignty."

4. **Scene 3 (40s):** Retrieve final verdict
   ```bash
   curl -H "Authorization: Bearer $CYNIC_API_KEY" \
     http://<TAILSCALE_CORE>:3030/verdicts?limit=1 | jq '.[] | {verdict, q_score, dogs_used, max_disagreement, timestamp}'
   ```
   Narrate: "The Crystal Coherence Machine crystallizes disagreement into a single verdict. This is q-score 0.39, GROWL confidence, with max disagreement 0.568 across axioms."

5. **Scene 4 (30s, optional):** Show on-chain submission
   ```bash
   curl -s https://api.devnet.solana.com -X POST -H "Content-Type: application/json" \
     -d '{
       "jsonrpc": "2.0",
       "id": 1,
       "method": "getAccountInfo",
       "params": ["8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD"]
     }' | jq .
   ```
   Narrate: "The verdict lands immutably on-chain via Pinocchio, verifiable forever."

### Option 2: FFmpeg (Minimal)
```bash
ffmpeg -f x11grab -i :0 -c:v libx264 -crf 23 -c:a aac cynic-demo.mp4
```

## Narration Timing (2-3 min total)

**Scene 1 (0:00-0:15):** "CYNIC is a multi-Dog consensus engine that judges Solana tokens on six axioms. Let's submit USDC to the kernel."

**Scene 2 (0:15-1:00):** "Five independent Dogs score in parallel on Fidelity (faithful to reality), Phi (structural harmony), Verify (testable claims), Culture (tradition), Burn (efficiency), and Sovereignty (human agency). Each Dog reasons in writing about its score."

**Scene 3 (1:00-1:40):** "The Crystal Coherence Machine crystallizes their disagreement into a single verdict. Notice the axiom scores vary — qwen-7b sees structure, qwen35-gpu catches nuance, deterministic-dog provides instant baseline. The system doesn't average; it learns consensus boundaries. This verdict is GROWL confidence, 75.5% detection on confirmed rug-pulls."

**Scene 4 (1:40-2:10, optional):** "The verdict and all axiom scores settle immutably on Solana devnet, verifiable on-chain. Sovereignty made visible."

## Colosseum Submission

Once video is recorded and uploaded:

1. Go to https://arena.colosseum.org/projects
2. Find CYNIC project (already created)
3. Upload video (~3-5 min max, MP4)
4. Add description from `COLOSSEUM-SUBMISSION-DRAFT.md`
5. Verify links:
   - GitHub: https://github.com/zeyxx/CYNIC
   - Deployed API: https://residents-administrator-temperature-boc.trycloudflare.com
   - UI: https://cynic-ui.vercel.app
6. Submit before May 10, 23:59 PDT

## Troubleshooting

**Kernel not responding?**
- Check: `systemctl --user status cynic-kernel`
- Restart: `systemctl --user restart cynic-kernel`
- Watch logs: `journalctl --user -u cynic-kernel -f`

**Dogs timing out?**
- qwen35-gpu slow? (8-9s is normal)
- gemma timing out? (OK, graceful skip)
- Deterministic + qwen7b sufficient for 2-3 Dog verdict

**Video too long?**
- Cut Scene 4 (on-chain) first
- Reduce Scene 2 (logs) to 30s
- Keep narration to essentials

---

**Target:** 2-3 min, clear audio, Dogs visible responding, q_score + verdict visible.
