"""
CYNIC Scout Dog — Malkuth (Kingdom)

Malkuth = the lowest sefirot, closest to the physical world.
"Kingdom" — where spirit meets matter. Scout touches GROUND TRUTH.

Where other dogs reason from code (Architect), memory (Scholar), or
topology (Cartographer) — SCOUT goes out and CHECKS. It fetches URLs,
tests reachability, measures response time, extracts content.

Technology: urllib (stdlib, async via run_in_executor)
Heuristic path (Phase 1): URL reachability + content extraction scoring.
LLM path (Phase 2): temporal_judgment() on extracted web content.

Responsibilities:
  - Extract URLs from cell content (regex)
  - Check HTTP reachability (status code, response time, content length)
  - Score: 200 OK + rich content → high Q; 404/timeout → penalized
  - Feed extracted content to temporal MCTS when LLM available

Why Scout?
  Malkuth = physical manifestation. The Kingdom where ideas become real.
  CYNIC's other dogs reason. Scout ACTS. It reaches out to the internet,
  tests links, fetches documentation, verifies claims.
  "Don't trust — verify. Scout is VERIFY made flesh."

φ-integration:
  MAX_URLS_PER_CELL = F(4)=3   — check at most 3 URLs (speed vs coverage)
  TIMEOUT_SEC       = F(3)=2   — 2s max per URL (fast or dead)
  MIN_CONTENT_LEN   = F(8)=21  — empty pages penalized
  MAX_LATENCY_MS    = F(12)×10 = 1440ms — slow pages flagged

  Penalties (additive, capped at BASE_Q):
    MISS_PENALTY      = 8.0   — URL not reachable (4xx/5xx)
    TIMEOUT_PENALTY   = 4.0   — URL timed out (likely dead)
    EMPTY_PENALTY     = 5.0   — URL reachable but empty content
    ERROR_PENALTY     = 6.0   — URL fetch error (DNS fail, SSL, etc.)

  Confidence: PHI_INV_2 (0.382) — Scout sees surface, not semantics.
  VETO: impossible — Scout discovers, never blocks.
"""
from __future__ import annotations

import asyncio
import logging
import re
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import (
    PHI_INV, PHI_INV_2, MAX_Q_SCORE, MAX_CONFIDENCE,
    phi_bound_score, fibonacci,
)
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.scout")

# ── Thresholds (Fibonacci-aligned) ────────────────────────────────────────
MAX_URLS_PER_CELL: int = fibonacci(4)   # F(4)=3  — max URLs checked per cell
TIMEOUT_SEC: float     = fibonacci(3)   # F(3)=2  — seconds per URL
MIN_CONTENT_LEN: int   = fibonacci(8)   # F(8)=21 — minimum meaningful content

BASE_Q: float = MAX_Q_SCORE

# ── Penalties ─────────────────────────────────────────────────────────────
MISS_PENALTY: float    = 8.0   # 4xx/5xx response
TIMEOUT_PENALTY: float = 4.0   # timed out
EMPTY_PENALTY: float   = 5.0   # reachable but empty
ERROR_PENALTY: float   = 6.0   # DNS/SSL/connection failure

SCOUT_CONFIDENCE: float = PHI_INV_2  # 0.382 — surface-level knowledge only

# ── URL extraction regex ────────────────────────────────────────────────────
_URL_PATTERN = re.compile(
    r'https?://[^\s<>"\'{}|\\^`\[\]]{4,}',
    re.IGNORECASE,
)


# ── Fetch result ───────────────────────────────────────────────────────────
@dataclass
class FetchResult:
    url: str
    status: int          # HTTP status (0 = timeout, -1 = error)
    content_len: int     # Bytes of content read (first 4KB)
    latency_ms: float

    @property
    def is_ok(self) -> bool:
        return 200 <= self.status < 300

    @property
    def is_miss(self) -> bool:
        return 400 <= self.status < 600

    @property
    def is_timeout(self) -> bool:
        return self.status == 0

    @property
    def is_error(self) -> bool:
        return self.status == -1


class ScoutDog(LLMDog):
    """
    Scout (Malkuth) — web/URL discovery and reachability scorer.

    Heuristic path: Async URL reachability checks via urllib (stdlib).
    Temporal MCTS path (when LLM available): 7-perspective temporal judgment.

    Malkuth = the physical world. Scout is where theory meets reality.
    If code links to a URL that 404s — Scout catches it. No LLM required.
    """

    def __init__(self) -> None:
        super().__init__(DogId.SCOUT, task_type="web_discovery")
        self._urls_checked: int = 0
        self._hits: int = 0       # 2xx responses
        self._misses: int = 0     # 4xx/5xx
        self._timeouts: int = 0
        self._errors: int = 0     # DNS/SSL/other

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.SCOUT,
            sefirot="Malkuth — Kingdom (ground truth, web discovery)",
            consciousness_min=ConsciousnessLevel.MACRO,  # Network I/O → too slow for REFLEX
            uses_llm=True,
            supported_realities={"SOCIAL", "COSMOS", "MARKET", "CODE"},
            supported_analyses={"PERCEIVE", "JUDGE"},
            technology="urllib (stdlib async via run_in_executor)",
            max_concurrent=3,   # F(4)=3 — limited by network concurrency
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Route to temporal MCTS path (LLM available) or URL reachability path.
        """
        start = time.perf_counter()
        adapter = await self.get_llm()
        if adapter is not None:
            return await self._temporal_path(cell, adapter, start)
        return await self._heuristic_path(cell, start)

    # ── Temporal Path (LLM) ───────────────────────────────────────────────

    async def _temporal_path(
        self,
        cell: Cell,
        adapter: Any,
        start: float,
    ) -> DogJudgment:
        """7-perspective temporal MCTS judgment on extracted web content."""
        from cynic.llm.temporal import temporal_judgment

        content = self._extract_content(cell)
        tj = await temporal_judgment(adapter, content)

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=(
                f"*sniff* Scout temporal MCTS: Q={tj.phi_aggregate:.1f} "
                f"from 7 discovery perspectives"
            ),
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── Heuristic Path (URL reachability) ─────────────────────────────────

    async def _heuristic_path(self, cell: Cell, start: float) -> DogJudgment:
        """Async URL reachability check — stdlib only, no LLM."""
        urls = self._extract_urls(cell)

        if not urls:
            return self._neutral_judgment(cell, start, reason="no-urls-found")

        checked = urls[:MAX_URLS_PER_CELL]
        results = await self._check_urls(checked)
        violations, penalty, evidence = self._score_results(results)

        self._urls_checked += len(results)
        for r in results:
            if r.is_ok:
                self._hits += 1
            elif r.is_miss:
                self._misses += 1
            elif r.is_timeout:
                self._timeouts += 1
            elif r.is_error:
                self._errors += 1

        q_raw = BASE_Q - penalty
        q_score = phi_bound_score(max(0.0, q_raw))

        if violations:
            reasoning = f"*sniff* Scout: {len(violations)} URL issue(s): {', '.join(violations[:3])}"
        else:
            reasoning = "*tail wag* Scout: all URLs reachable — Malkuth confirms ground truth"

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=SCOUT_CONFIDENCE,
            reasoning=reasoning,
            evidence={**evidence, "violations": violations[:8]},
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── URL Extraction ─────────────────────────────────────────────────────

    def _extract_urls(self, cell: Cell) -> List[str]:
        """Extract URLs from cell content. Deduplicated, order preserved."""
        content = cell.content
        if isinstance(content, str):
            text = content
        elif isinstance(content, dict):
            parts = []
            for key in ("url", "urls", "links", "content", "text", "source"):
                val = content.get(key)
                if val:
                    parts.append(str(val) if not isinstance(val, list) else " ".join(val))
            text = " ".join(parts)
        else:
            text = str(content) if content else ""

        found = _URL_PATTERN.findall(text)
        # Deduplicate while preserving order
        seen: set = set()
        unique = []
        for url in found:
            url = url.rstrip(".,;)")  # Strip trailing punctuation
            if url not in seen:
                seen.add(url)
                unique.append(url)
        return unique

    def _extract_content(self, cell: Cell) -> str:
        """Extract text content for temporal judgment."""
        content = cell.content
        if isinstance(content, str):
            return content[:2000]
        if isinstance(content, dict):
            parts = [str(v) for k, v in content.items() if v and k != "budget_usd"]
            return " ".join(parts)[:2000]
        return (
            f"reality:{cell.reality} analysis:{cell.analysis} "
            f"complexity:{cell.complexity:.2f} novelty:{cell.novelty:.2f}"
        )

    # ── Async URL Checking ─────────────────────────────────────────────────

    async def _check_urls(self, urls: List[str]) -> List[FetchResult]:
        """Check all URLs concurrently via asyncio.gather."""
        tasks = [self._fetch_url(url) for url in urls]
        return list(await asyncio.gather(*tasks))

    async def _fetch_url(self, url: str) -> FetchResult:
        """Async URL fetch — runs _sync_fetch in thread executor."""
        loop = asyncio.get_running_loop()
        start = time.perf_counter()
        try:
            result = await asyncio.wait_for(
                loop.run_in_executor(None, self._sync_fetch, url),
                timeout=TIMEOUT_SEC + 0.5,  # Slightly more than sync timeout
            )
            return result
        except asyncio.TimeoutError:
            latency = (time.perf_counter() - start) * 1000
            logger.debug("Scout timeout: %s", url)
            return FetchResult(url=url, status=0, content_len=0, latency_ms=latency)
        except Exception as e:
            latency = (time.perf_counter() - start) * 1000
            logger.debug("Scout error on %s: %s", url, e)
            return FetchResult(url=url, status=-1, content_len=0, latency_ms=latency)

    @staticmethod
    def _sync_fetch(url: str) -> FetchResult:
        """
        Synchronous URL fetch (runs in executor, stdlib only).

        Reads first 4KB only — we need status + content presence, not full page.
        """
        import urllib.error
        import urllib.request

        start = time.perf_counter()
        try:
            req = urllib.request.Request(
                url,
                headers={"User-Agent": "CYNIC-Scout/1.0 (φ-judgment engine)"},
            )
            with urllib.request.urlopen(req, timeout=TIMEOUT_SEC) as resp:
                status = resp.status
                content = resp.read(4096)  # First 4KB only
                latency = (time.perf_counter() - start) * 1000
                return FetchResult(
                    url=url,
                    status=status,
                    content_len=len(content),
                    latency_ms=latency,
                )
        except urllib.error.HTTPError as e:
            latency = (time.perf_counter() - start) * 1000
            return FetchResult(url=url, status=e.code, content_len=0, latency_ms=latency)
        except urllib.error.URLError:
            latency = (time.perf_counter() - start) * 1000
            return FetchResult(url=url, status=-1, content_len=0, latency_ms=latency)
        except Exception:
            latency = (time.perf_counter() - start) * 1000
            return FetchResult(url=url, status=-1, content_len=0, latency_ms=latency)

    # ── Scoring ────────────────────────────────────────────────────────────

    def _score_results(
        self,
        results: List[FetchResult],
    ) -> Tuple[List[str], float, Dict[str, Any]]:
        """
        Score fetch results → (violations, penalty, evidence).

        Perfect: all URLs 200 + rich content → no penalty.
        Bad: timeouts, 4xx, empty pages → penalty each.
        """
        violations: List[str] = []
        total_penalty: float = 0.0

        ok_count = 0
        for r in results:
            if r.is_ok:
                ok_count += 1
                if r.content_len < MIN_CONTENT_LEN:
                    violations.append(f"empty-content:{r.url[:50]}")
                    total_penalty += EMPTY_PENALTY
            elif r.is_miss:
                violations.append(f"http-{r.status}:{r.url[:50]}")
                total_penalty += MISS_PENALTY
            elif r.is_timeout:
                violations.append(f"timeout:{r.url[:50]}")
                total_penalty += TIMEOUT_PENALTY
            elif r.is_error:
                violations.append(f"error:{r.url[:50]}")
                total_penalty += ERROR_PENALTY

        avg_latency = (
            sum(r.latency_ms for r in results) / len(results)
            if results else 0.0
        )

        evidence = {
            "urls_checked": len(results),
            "ok_count": ok_count,
            "miss_count": sum(1 for r in results if r.is_miss),
            "timeout_count": sum(1 for r in results if r.is_timeout),
            "error_count": sum(1 for r in results if r.is_error),
            "avg_latency_ms": round(avg_latency, 1),
            "total_penalty": round(total_penalty, 2),
            "results": [
                {
                    "url": r.url[:80],
                    "status": r.status,
                    "content_len": r.content_len,
                    "latency_ms": round(r.latency_ms, 1),
                }
                for r in results
            ],
        }
        return violations, total_penalty, evidence

    # ── Neutral Judgment ───────────────────────────────────────────────────

    def _neutral_judgment(
        self,
        cell: Cell,
        start: float,
        reason: str = "no-urls",
    ) -> DogJudgment:
        """Neutral when no URLs found — Scout has nothing to check."""
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(MAX_Q_SCORE * 0.5),  # Neutral → GROWL
            confidence=SCOUT_CONFIDENCE,
            reasoning=f"*head tilt* No URLs to check ({reason}) — neutral verdict",
            evidence={"reason": reason, "urls_found": 0},
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── Health ─────────────────────────────────────────────────────────────

    async def health_check(self) -> DogHealth:
        total = self._urls_checked
        hit_rate = self._hits / max(total, 1)
        status = (
            HealthStatus.HEALTHY  if total > 0 and hit_rate >= PHI_INV_2 else
            HealthStatus.DEGRADED if total > 0 else
            HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"URLs checked: {total}, "
                f"OK: {self._hits}, Miss: {self._misses}, "
                f"Timeout: {self._timeouts}, Error: {self._errors}, "
                f"Hit rate: {hit_rate:.0%}"
            ),
        )
