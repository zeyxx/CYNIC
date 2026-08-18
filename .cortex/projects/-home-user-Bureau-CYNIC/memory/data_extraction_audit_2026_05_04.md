---
name: Data Extraction Audit & x_proxy Enrichment Gap
description: Resolved architectural issue where x_proxy computed enrichments weren't being used downstream
type: project
---

# Data Extraction Audit — 2026-05-04

## Problem Discovered

User's challenge: "from the audit, but from reality, are we sure using 100% of what we can extract?"

**Answer: No.** x_proxy.py was computing sophisticated enrichments but downstream tools weren't using them.

## Architecture Issue

### x_proxy.py (mitmproxy addon)
- **Computes:** signal_score (-5 to +7), narratives, author_tier, coordination_count
- **Saves to:** dataset.jsonl with all 49 fields
- **Lines:** 68-168 (signal logic), 269-338 (_enrich function)
- Status: ✓ WORKING

### kill-chain tracer
- **Read from:** captures/*.json directly (lines 178-235)
- **Recomputed:** signal_score with naive heuristics (lines 279-299)
- **Ignored:** All x_proxy enrichments in dataset.jsonl
- Status: ✗ WASTED COMPUTATION

### organic agent
- **Read from:** learned_weights.json only
- **Ignores:** dataset.jsonl entirely (but agent is live, so this is acceptable)

## Fix Implemented

**Commit:** `feat(hermes-x): use pre-computed enrichments in kill-chain`

### Changes
1. Added `load_enrichments()` method (lines 157-192)
   - Loads 8586 rows from dataset.jsonl
   - Dedupes to 8490 unique tweets
   - Extracts: signal_score, narratives, author_tier, coordination_count

2. Modified `_parse_tweet()` (lines 295-299)
   - Check `self.tweet_enrichments[tweet_id]` first
   - Fall back to naive heuristics if not in enrichment cache

3. Wired in main() (line 531)
   - Non-blocking: load_enrichments() is optional (graceful degradation)

### Verification
- Kill-chain ran successfully: 29456 clicks processed
- Enrichment loaded: "✓ Loaded 8586 enrichments from dataset.jsonl (deduped to 8490 unique tweets)"
- Sample output: signal_score=4 (from x_proxy) vs naive heuristic baseline
- Coverage: 4.5% clicks matched to captures, avg signal 0.14

## Data Extraction Status

### All 49 Fields in dataset.jsonl

**NOW BEING USED (after fix):**
- signal_score (x_proxy computed)
- narratives (x_proxy computed)
- author_tier (x_proxy computed)
- coordination_count (x_proxy computed)

**COMPUTED BUT NOT CONSUMED:**
- engagement_rate (lines 319, x_proxy: `round(engagement, 6)`)
- is_coordinated (lines 335, x_proxy: `coord_count >= 3`)

**CAPTURED BUT UNDERUTILIZED:**
- viewer_favorited, viewer_retweeted, viewer_bookmarked (indicate actual user engagement)
- author_bio, author_account_created_at, author_default_profile* (credibility signals)
- cashtags, hashtags, mentions (domain-specific signals)
- media, urls, has_media (content structure)
- quoted_tweet, retweeted_tweet (conversation context)

**METADATA:**
- dedupe_key, sampling_bias, capture_ts, operation, search_query, interaction_type, lang, possibly_sensitive

## Secondary Extraction Gaps (Post-Phase 2)

1. **Engagement rate** — Could replace naive engagement calculation with pre-computed value
2. **Viewer fields** — Indicate whether T. actually engaged with the tweet (stronger signal than metadata)
3. **Domain signals** — cashtags/hashtags/mentions could drive routing logic
4. **Author credibility** — Bio, account age, default profile status unused

## Root Cause

Pattern: **Intermediate storage (dataset.jsonl) becomes invisible when downstream tools don't read from it.** 

Kill-chain re-parsed captures/*.json (original format) instead of querying dataset (processed format). This is safe but wasteful: duplicates the enrichment computation and loses sophistication.

**Prevention:** Establish K15-style consumer contract — "What system reads enriched tweets from dataset.jsonl?" Answer: kill-chain (now), future: domain router, behavioral analyzer.

## Design Principle

Storage without consumption is dead data. This was a latent K15 violation: x_proxy was publishing enrichments to a file no one read.
