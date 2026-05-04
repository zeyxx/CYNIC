# Organ X Feedback Loop — Semi-Manual Learning

**Status**: WIRED (2026-04-28). Ready for human decision-making.

## What Is This?

Hermes (Claude Code/Gemini) can now:
1. See what Dogs think about its observations (verdicts)
2. Get recommendations on which domain to explore next
3. Decide to follow or override the recommendation
4. Log the decision for analysis

## How It Works

### 1. Each Session (Hermes runs every 4 hours via cron)

**At START:**
```
Claude Code session starts
  ↓
read ~/.cynic/organs/hermes/x/recommendation.json (if exists)
  ↓
Hermes sees: "D4 (Security/Scams) suggested — deficit 97%, HIGH priority"
  ↓
Hermes decides: "Accept. This domain needs coverage."
  ↓
python feedback_decision_log.py \
  --decision D4 \
  --reason "Highest deficit (97%), HIGH priority, low recency" \
  --action "Browse @gcrtrd + @FabianoSolana, analyze dataset for scam patterns"
  ↓
Explore D4 for the session...
```

**At END:**
```
Session complete. Hermes found 5 observations, 2 high-signal.
  ↓
python generate_domain_recommendation.py
  ↓
Recommendation written: "D1 next (deficit 87%), previous Dogs scored D4 highest"
  ↓
NEXT CRON (4h later): Claude sees this, decides whether to follow
```

### 2. Files

| File | What | Who Updates | When |
|------|------|-------------|------|
| `recommendation.json` | "What should I explore next?" | `generate_domain_recommendation.py` | End of session |
| `feedback_decisions.jsonl` | "What did Hermes choose?" | `feedback_decision_log.py` | Start of session |
| `domain_coverage.json` | "Curated count per domain" | Claude Code | End of session |
| `feedback_log.jsonl` (future) | "What did Dogs think?" | kernel (via /state-history) | Continuously |

### 3. Decision Points (Where Claude Code Chooses)

Hermes **must** read `recommendation.json` at session start and decide:

**Option A: Accept Recommendation**
```
"D1 is recommended"
→ Hermes agrees: "D1 has 87% deficit, makes sense"
→ Logs: feedback_decision_log.py --decision D1 --reason "Accepted recommendation"
→ Explores D1
```

**Option B: Override**
```
"D1 is recommended"
→ Hermes disagrees: "Just explored D1 yesterday, skip it"
→ Logs: feedback_decision_log.py --decision D4 --reject --reason "Rejected recommendation; D1 too recent"
→ Explores D4 instead
```

**Option C: Combine**
```
"D1 is recommended"
→ Hermes: "D1 + D4 both need work, I'll split my session"
→ Logs BOTH decisions, explores both
```

## Files to Know

- **`generate_domain_recommendation.py`** — generates the next suggestion (call at session END)
- **`feedback_decision_log.py`** — records what Claude decided (call at session START)
- **`~/.hermes/skills/cynic/SKILL.md`** — tells Hermes the protocol (updated 2026-04-28)

## Next Steps

1. **Next x-explorer cron (20:55 CEST)**:
   - Runs naturally as usual
   - At END, generates recommendation.json

2. **Session after that (00:55 CEST 2026-04-29)**:
   - Claude Code reads recommendation.json
   - Makes decision
   - Logs it
   - Explores chosen domain
   - At END, generates next recommendation

3. **Loop Stabilizes**:
   - Each 4h: Hermes reads Dogs' feedback (via recommendation)
   - Each 4h: Hermes decides next domain
   - Coverage naturally balances with Dogs' signal quality

## Measurement Points

Watch for:
1. **Decision acceptance rate**: How often does Hermes follow the recommendation? (target: >60%)
2. **Coverage growth**: Do low-coverage domains fill faster? (measure D1, D3, D4)
3. **Signal-to-Dogs latency**: From observation → verdict → next decision (target: <24h)
4. **Feedback quality**: Do recommended domains have higher verdicts than ignored ones?

## Known Limitations

- **Slow feedback**: Verdicts take time to accumulate (need 10+ per domain to measure signal)
- **Cold start**: First recommendation may be based on old verdict data
- **Hermes autonomy**: Claude Code must decide each time (not automatic) — requires session involvement

## Timeline

- **2026-04-28 20:55**: First recommendation generated
- **2026-04-29 00:55**: First manual decision made
- **2026-04-29 to 2026-05-05**: Feedback loop matures, patterns emerge
- **2026-05-05**: First falsifiable signal (Can we measure if recommendations improve learning?)
