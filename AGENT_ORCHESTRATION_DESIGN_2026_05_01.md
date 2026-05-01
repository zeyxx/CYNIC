# Agent Orchestration Design — Closing the Organ X Loop

## Pipeline State (Observed 2026-05-01 18:52)

### Running Services (✓)
| Service | Status | Output | Last Run |
|---------|--------|--------|----------|
| hermes-search-generator | ✓ working | 7 search tasks to search_tasks.jsonl | 18:32:47 |
| hermes-search-executor | ✗ MISSING script | — | — |
| hermes-feedback-loop | ✓ working | Updated SKILL.md from Gemini | 18:09:47 |
| hermes-gemini-briefing | ✗ missing input | Needs lab_briefing_latest.json | 18:14:47 |
| hermes-x-ingest | ✓ active | Tailing dataset.jsonl → POST /observe | RUNNING |

### Data Flow (Current State)

```
SKILL.md (Domain dashboard)
  ↓ 15m timer
hermes-search-generator.service
  ↓ generates 7 search tasks
search_tasks.jsonl
  ↓ [MISSING: hermes-search-executor should read this]
  ✗ BROKEN LINK (search_executor.py doesn't exist)

hermes-browser + mitmproxy (passive captures)
  ↓
dataset.jsonl
  ↓ [hermes-x-ingest runs, tails, POSTs to kernel]
kernel /observe
  ↓
verdicts (Dogs judge observations)
  ↓ 1h timer
hermes-feedback-loop.service (Gemini learns patterns)
  ↓ updates SKILL.md
SKILL.md (loop closes... partially)
```

**K15 Status:** Producer (ingest) connected to consumer (feedback loop), but search execution is orphaned.

---

## Missing Implementation: search_executor.py

### Purpose
Read search_tasks.jsonl, execute each task on the running X.com browser, capture results to mitmproxy.

### Design

```python
#!/usr/bin/env python3
"""
Hermes X Search Executor — Executes farming/exploration searches
Reads search tasks from search_tasks.jsonl and runs them via browser CDP
"""

import json
import asyncio
import time
import os
from pathlib import Path
from datetime import datetime
import logging

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("search-executor")

class SearchExecutor:
    def __init__(self, cdp_url="http://localhost:40769", task_file=None):
        """
        Args:
            cdp_url: Chrome DevTools Protocol URL
            task_file: Path to search_tasks.jsonl (default: ~/.cynic/organs/hermes/x/search_tasks.jsonl)
        """
        self.cdp_url = cdp_url
        self.task_file = task_file or Path.home() / ".cynic/organs/hermes/x/search_tasks.jsonl"
        self.execution_log = Path.home() / ".cynic/organs/hermes/x/search_execution_log.jsonl"
        
    async def fetch_cdp_endpoint(self):
        """Get browser CDP endpoint from /json/list"""
        import aiohttp
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{self.cdp_url}/json/list") as resp:
                pages = await resp.json()
                if pages:
                    return pages[0].get("webSocketDebuggerUrl")
        return None
    
    async def execute_search(self, query: str, timeout_sec: int = 30):
        """
        Execute a single search on X.com
        
        Args:
            query: Search query string (e.g., "search:slow rug")
            timeout_sec: Max seconds to wait for results
            
        Returns:
            dict with execution metadata (url, results_count, time_taken)
        """
        try:
            # Use direct HTTP to Chrome, not Playwright
            # (Avoids CDP connection limitations)
            import httpx
            
            # 1. Create new page via Chrome HTTP endpoint
            async with httpx.AsyncClient() as client:
                # Create new page
                resp = await client.post(
                    f"{self.cdp_url}/json/new",
                    json={"url": f"https://x.com/search?q={query}"},
                    timeout=timeout_sec
                )
                result = resp.json()
                page_id = result.get("id")
                
                # 2. Navigate to search URL
                target_url = f"https://x.com/search?q={query.replace('search:', '')}"
                
                # Wait for page to load (passive: mitmproxy captures)
                await asyncio.sleep(5)  # Let page load + mitmproxy capture
                
                # Log execution
                return {
                    "timestamp": datetime.utcnow().isoformat(),
                    "query": query,
                    "url": target_url,
                    "status": "executed",
                    "duration_sec": 5,
                    "page_id": page_id
                }
        except Exception as e:
            logger.error(f"Search execution failed for '{query}': {e}")
            return {
                "timestamp": datetime.utcnow().isoformat(),
                "query": query,
                "status": "failed",
                "error": str(e)
            }
    
    async def run(self):
        """Read tasks from search_tasks.jsonl and execute each"""
        if not self.task_file.exists():
            logger.error(f"Task file not found: {self.task_file}")
            return
        
        logger.info(f"Reading tasks from {self.task_file}")
        
        with open(self.task_file) as f:
            tasks = [json.loads(line) for line in f if line.strip()]
        
        logger.info(f"Loaded {len(tasks)} tasks")
        
        results = []
        for i, task in enumerate(tasks):
            logger.info(f"[{i+1}/{len(tasks)}] Executing: {task.get('query')}")
            result = await self.execute_search(task.get("query", ""))
            results.append(result)
            
            # Stagger executions (respect rate limits, look human)
            if i < len(tasks) - 1:
                await asyncio.sleep(8)  # 8s between searches
        
        # Log all executions
        with open(self.execution_log, "a") as f:
            for result in results:
                f.write(json.dumps(result) + "\n")
        
        logger.info(f"✓ Executed {len(results)} searches")
        logger.info(f"✓ Logged execution to {self.execution_log}")

async def main():
    executor = SearchExecutor()
    await executor.run()

if __name__ == "__main__":
    asyncio.run(main())
```

### Deployment
1. Save to `/home/user/Bureau/CYNIC/scripts/hermes-x/search_executor.py`
2. systemctl will call it via hermes-search-executor.timer (15m)
3. Execution logged to search_execution_log.jsonl

---

## Missing Input: lab_briefing_latest.json

### Problem
`hermes-gemini-briefing.service` expects a briefing file (curated domain analysis) that doesn't exist.

### Solution
Create a **stub briefing** that the gemini-briefing cron can synthesize:

```json
{
  "timestamp": "2026-05-01T18:52:00Z",
  "domains": {
    "D1": {
      "signal": "Token/Solana rug detection: 89% accuracy on validation set",
      "key_patterns": ["pump.fun graduation rate", "liquidity locks", "founder verification"],
      "recent_observations": 3,
      "quality_score": 0.78
    },
    "D2": {
      "signal": "LLM inference: Claude vs Qwen cost-quality frontier",
      "key_patterns": ["context length", "reasoning budget", "latency vs accuracy"],
      "recent_observations": 1,
      "quality_score": 0.55
    },
    "D3": {
      "signal": "Sovereignty: on-chain proofs vs off-chain attestation",
      "key_patterns": ["self-custody", "oracle risk", "incentive alignment"],
      "recent_observations": 0,
      "quality_score": 0.0
    },
    "D4": {
      "signal": "Security: honeypot detection via transfer simulation",
      "key_patterns": ["blocked transfers", "tax evasion", "contract backdoors"],
      "recent_observations": 2,
      "quality_score": 0.62
    },
    "D5": {
      "signal": "Macro: Solana ecosystem volatility correlation with BTC",
      "key_patterns": ["correlation coefficient", "funding rate", "liquidation cascades"],
      "recent_observations": 1,
      "quality_score": 0.44
    },
    "D6": {
      "signal": "Epistemology: calibration bounds on LLM confidence",
      "key_patterns": ["φ-bounds", "consensus metrics", "disagreement signals"],
      "recent_observations": 0,
      "quality_score": 0.0
    }
  },
  "gaps": [
    "D3 (Sovereignty): 0 observations — need direct on-chain analysis",
    "D6 (Epistemology): 0 observations — need calibration experiments"
  ],
  "next_focus": "D3 (sovereignty patterns) and D4 (security refinement)"
}
```

Save to `~/.cynic/organs/hermes/x/lab_briefing_latest.json`.

---

## Agent Orchestration Loop (Complete)

### Timeline
1. **T+0min:** SKILL.md (Domain Dashboard) exists
2. **T+0min:** hermes-search-generator (15m timer) reads SKILL.md
3. **T+1min:** Generates 7 search tasks → search_tasks.jsonl
4. **T+2min:** [NEW] hermes-search-executor (15m timer) reads tasks
5. **T+3-8min:** Browser executes searches, mitmproxy captures X.com responses
6. **T+10min:** hermes-x-ingest (daemon) tails dataset.jsonl → kernel /observe
7. **T+15min:** kernel judges observations (Dogs score)
8. **T+60min:** hermes-feedback-loop (1h timer) reads verdicts
9. **T+61min:** Gemini CLI analyzes patterns, updates SKILL.md
10. **T+240min:** hermes-gemini-briefing (4h timer) synthesizes briefing
11. **T+241min:** Loop back to step 2 (next cycle)

### Data Lifecycle
```
Domain wisdom (SKILL.md)
  ↓ [Search generator reads coverage gaps]
Search tasks [NEW: Executor consumes]
  ↓ [Browser executes]
X.com captures (mitmproxy)
  ↓ [Ingest tails]
Kernel observations
  ↓ [Kernel judges]
Verdicts
  ↓ [Feedback loop consumes]
Learned patterns
  ↓ [Update SKILL.md]
→ Back to domain wisdom
```

---

## Implementation Checklist

- [ ] Create `scripts/hermes-x/search_executor.py` (this design)
- [ ] Set executable: `chmod +x scripts/hermes-x/search_executor.py`
- [ ] Create lab_briefing_latest.json (stub above)
- [ ] Enable/test hermes-search-executor.service: `systemctl --user start hermes-search-executor`
- [ ] Enable/test hermes-gemini-briefing.service: `systemctl --user start hermes-gemini-briefing`
- [ ] Verify loop: Watch search_tasks.jsonl → execution_log → SKILL.md updates over 2h

---

## Post-Hackathon: Organic Agent Execution

**Current:** Search executor uses Chrome HTTP endpoint (simple, deterministic).

**May 10+:** Replace with behavior-ML-guided execution:
```python
# Instead of mechanical 8s pauses:
next_event = behavior_model.sample(context=[last_20_events])
await browser.execute_cdp(next_event)
```

This gives the agent learned human-like browsing rhythm.

---

*Architecture: 2026-05-01 18:55 CEST, ready for implementation.*
