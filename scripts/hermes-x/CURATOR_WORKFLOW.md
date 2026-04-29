# @CynicOracle Curator Workflow

**Goal:** Format Hermes X verdicts for manual posting to @CynicOracle. Filters high-confidence judgments + high-signal observations.

## Setup

```bash
python3 scripts/hermes-x/x_verdict_curator.py --output verdicts_to_post.json
```

## Output

JSON file with 42 verdicts ready to post (as of 2026-04-30):

```json
{
  "count": 42,
  "curated_at": "2026-04-30T00:53:31Z",
  "verdicts": [
    {
      "tweet_id": "1688806833396543488",
      "verdict_type": "BARK",
      "q_score": 0.184,
      "signal_score": 5,
      "post_text": "Verdict: BARK (0.18) on \"r/LocalLlama did a comparative analysis …\"\nDogs: 3/5 | Signal: 5\nAxioms → cynic.ai/v/9e5b2e7e",
      "verdict_url": "https://cynic.ai/verdicts/9e5b2e7e-d08f-4b12-a9e1-c202f99a2c07",
      ...
    }
  ]
}
```

## Posting Workflow

### Step 1: Generate curator output
```bash
python3 scripts/hermes-x/x_verdict_curator.py --output verdicts_to_post.json
```

### Step 2: Manual post to @CynicOracle
- Copy `post_text` from JSON
- Paste into X draft on @CynicOracle
- Post

### Step 3: Track posted verdict
```bash
python3 scripts/hermes-x/x_verdict_curator.py \
  --mark-posted 1688806833396543488 1752345678901234567
```

(Replace second arg with the X tweet_id of your posted verdict.)

## Filtering Logic

Curator posts verdicts if:
- **HOWL** (q_score > 0.528) — any confidence level, OR
- **BARK** (q_score ≤ 0.236) on high-signal tweets (signal_score ≥ 5) — proven low confidence on strong signals

Excludes: GROWL, WAG (ambiguous); BARK on low-signal tweets (noise).

## For CHAOS-MATRIX Phase (4/30 - 5/7)

**Goal:** Post 5-10 verdicts, measure engagement + identify patterns.

1. Generate curator: `python3 scripts/hermes-x/x_verdict_curator.py`
2. Pick 5-10 verdicts from output (prefer mix of BARK + HOWL)
3. Post daily to @CynicOracle
4. Track posted tweet_id for each post
5. Measure: retweets, replies, follows

---

**Status:** 42 verdicts curated, 0 posted (as of 2026-04-30).
