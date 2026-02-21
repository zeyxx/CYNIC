"""
Tests for ScoutDog (Malkuth — Kingdom, 11th/final Sefirot).

Scout checks ground truth: URL reachability via urllib.
Two paths:
  1. Heuristic path: async URL checking via _sync_fetch (patchable)
  2. Temporal MCTS path: temporal_judgment() when LLM available

φ-alignment:
  MAX_URLS = F(4) = 3
  TIMEOUT  = F(3) = 2s
  MIN_CONTENT = F(8) = 21 bytes
"""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
from cynic.core.judgment import Cell
from cynic.core.consciousness import ConsciousnessLevel
from cynic.cognition.neurons.base import DogId, LLMDog


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_cell(content=None, reality: str = "CODE") -> Cell:
    return Cell(
        reality=reality,
        analysis="JUDGE",
        content=content or "def foo(): return 42",
        novelty=0.3,
        complexity=0.4,
        risk=0.2,
        budget_usd=0.1,
    )


def make_url_cell(urls: list[str], reality: str = "CODE") -> Cell:
    content = " ".join(urls) + "\nSome surrounding text."
    return Cell(
        reality=reality,
        analysis="PERCEIVE",
        content=content,
        novelty=0.2,
        complexity=0.3,
        risk=0.1,
        budget_usd=0.05,
    )


def _mock_registry(score: float = 45.0) -> MagicMock:
    """LLMRegistry mock returning fixed temporal score."""
    adapter = MagicMock()
    adapter.adapter_id = "ollama:llama3.2"

    async def _complete_safe(req):
        r = MagicMock()
        r.content = f"SCORE: {score}"
        return r

    adapter.complete_safe = _complete_safe
    registry = MagicMock()
    registry.get_best_for = MagicMock(return_value=adapter)
    return registry


def _make_fetch_result(status: int = 200, content_len: int = 512, latency_ms: float = 80.0):
    """Build a FetchResult for patching _sync_fetch."""
    from cynic.cognition.neurons.scout import FetchResult
    return FetchResult(
        url="https://example.com",
        status=status,
        content_len=content_len,
        latency_ms=latency_ms,
    )


# ════════════════════════════════════════════════════════════════════════════
# IDENTITY
# ════════════════════════════════════════════════════════════════════════════

class TestScoutIdentity:

    def test_scout_extends_llm_dog(self):
        from cynic.cognition.neurons.scout import ScoutDog
        assert issubclass(ScoutDog, LLMDog)

    def test_scout_dog_id(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        assert dog.dog_id == DogId.SCOUT

    def test_scout_task_type(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        assert dog.task_type == "web_discovery"

    def test_scout_uses_llm_true(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        assert dog.get_capabilities().uses_llm is True

    def test_scout_sefirot_malkuth(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        caps = dog.get_capabilities()
        assert "Malkuth" in caps.sefirot

    def test_scout_consciousness_macro(self):
        """Scout is network I/O — too slow for REFLEX."""
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        caps = dog.get_capabilities()
        assert caps.consciousness_min == ConsciousnessLevel.MACRO

    @pytest.mark.asyncio
    async def test_scout_no_veto(self):
        """Scout discovers — judgments never carry veto=True."""
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        # Even with no URLs, neutral judgment → veto=False
        j = await dog.analyze(make_cell("def foo(): pass"))
        assert j.veto is False

    def test_scout_has_llm_interface(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        assert hasattr(dog, "set_llm_registry")
        assert hasattr(dog, "get_llm")

    def test_scout_supported_realities(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        realities = dog.get_capabilities().supported_realities
        assert "CODE" in realities
        assert "SOCIAL" in realities


# ════════════════════════════════════════════════════════════════════════════
# FetchResult dataclass
# ════════════════════════════════════════════════════════════════════════════

class TestFetchResult:

    def test_is_ok_200(self):
        from cynic.cognition.neurons.scout import FetchResult
        r = FetchResult(url="u", status=200, content_len=100, latency_ms=50)
        assert r.is_ok is True
        assert r.is_miss is False
        assert r.is_timeout is False
        assert r.is_error is False

    def test_is_miss_404(self):
        from cynic.cognition.neurons.scout import FetchResult
        r = FetchResult(url="u", status=404, content_len=0, latency_ms=30)
        assert r.is_miss is True
        assert r.is_ok is False

    def test_is_miss_500(self):
        from cynic.cognition.neurons.scout import FetchResult
        r = FetchResult(url="u", status=500, content_len=0, latency_ms=30)
        assert r.is_miss is True

    def test_is_timeout(self):
        from cynic.cognition.neurons.scout import FetchResult
        r = FetchResult(url="u", status=0, content_len=0, latency_ms=2000)
        assert r.is_timeout is True
        assert r.is_ok is False
        assert r.is_miss is False

    def test_is_error(self):
        from cynic.cognition.neurons.scout import FetchResult
        r = FetchResult(url="u", status=-1, content_len=0, latency_ms=10)
        assert r.is_error is True
        assert r.is_ok is False


# ════════════════════════════════════════════════════════════════════════════
# URL Extraction
# ════════════════════════════════════════════════════════════════════════════

class TestUrlExtraction:

    def test_extract_plain_url(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = make_url_cell(["https://example.com/foo"])
        urls = dog._extract_urls(cell)
        assert "https://example.com/foo" in urls

    def test_extract_multiple_urls(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = make_url_cell([
            "https://example.com",
            "https://google.com/search?q=test",
            "http://localhost:8000/api",
        ])
        urls = dog._extract_urls(cell)
        assert len(urls) == 3

    def test_extract_deduplicates(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = make_url_cell(["https://example.com", "https://example.com"])
        urls = dog._extract_urls(cell)
        assert urls.count("https://example.com") == 1

    def test_extract_strips_trailing_punctuation(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = Cell(
            reality="CODE", analysis="JUDGE",
            content="See https://example.com/path.",  # trailing dot
            novelty=0.1, complexity=0.1, risk=0.1, budget_usd=0.01,
        )
        urls = dog._extract_urls(cell)
        assert "https://example.com/path" in urls
        assert "https://example.com/path." not in urls

    def test_extract_from_dict_content(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = Cell(
            reality="SOCIAL", analysis="PERCEIVE",
            content={"url": "https://twitter.com/test", "text": "some tweet"},
            novelty=0.1, complexity=0.1, risk=0.1, budget_usd=0.01,
        )
        urls = dog._extract_urls(cell)
        assert "https://twitter.com/test" in urls

    def test_extract_no_urls(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        cell = make_cell("def foo(): return 42")
        urls = dog._extract_urls(cell)
        assert urls == []


# ════════════════════════════════════════════════════════════════════════════
# Heuristic Path — No LLM
# ════════════════════════════════════════════════════════════════════════════

class TestHeuristicPath:

    @pytest.mark.asyncio
    async def test_no_urls_returns_neutral(self):
        """Cell without URLs → neutral judgment (no-urls-found)."""
        from cynic.cognition.neurons.scout import ScoutDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = ScoutDog()
        j = await dog.analyze(make_cell("def foo(): pass"))  # No URLs
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.SCOUT
        assert j.llm_id is None
        assert j.evidence["reason"] == "no-urls-found"
        assert j.veto is False

    @pytest.mark.asyncio
    async def test_all_ok_urls_high_score(self):
        """All URLs 200 OK → no penalty → high Q score."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=1024, latency_ms=50)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            j = await dog.analyze(make_url_cell(["https://example.com"]))
        assert j.llm_id is None
        assert j.q_score > 50.0   # No penalty → near BASE_Q
        assert j.veto is False
        assert "violations" in j.evidence
        assert len(j.evidence["violations"]) == 0

    @pytest.mark.asyncio
    async def test_404_url_penalized(self):
        """404 response → MISS_PENALTY applied → lower Q score."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult, MISS_PENALTY, BASE_Q
        dog = ScoutDog()
        miss = FetchResult(url="https://dead.example.com", status=404, content_len=0, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=miss):
            j = await dog.analyze(make_url_cell(["https://dead.example.com"]))
        # Q = max(0, BASE_Q - MISS_PENALTY)
        assert j.q_score < BASE_Q
        assert any("http-404" in v for v in j.evidence["violations"])

    @pytest.mark.asyncio
    async def test_timeout_penalized(self):
        """Timeout → TIMEOUT_PENALTY applied."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        timeout_r = FetchResult(url="https://slow.example.com", status=0, content_len=0, latency_ms=2100)
        with patch.object(ScoutDog, "_sync_fetch", return_value=timeout_r):
            j = await dog.analyze(make_url_cell(["https://slow.example.com"]))
        assert any("timeout" in v for v in j.evidence["violations"])

    @pytest.mark.asyncio
    async def test_error_penalized(self):
        """DNS/SSL error → ERROR_PENALTY applied."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        err = FetchResult(url="https://notexist.xyz", status=-1, content_len=0, latency_ms=10)
        with patch.object(ScoutDog, "_sync_fetch", return_value=err):
            j = await dog.analyze(make_url_cell(["https://notexist.xyz"]))
        assert any("error" in v for v in j.evidence["violations"])

    @pytest.mark.asyncio
    async def test_empty_content_penalized(self):
        """200 OK but content_len < MIN_CONTENT_LEN → EMPTY_PENALTY."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult, MIN_CONTENT_LEN
        dog = ScoutDog()
        empty_r = FetchResult(url="https://empty.example.com", status=200, content_len=5, latency_ms=40)
        assert empty_r.content_len < MIN_CONTENT_LEN
        with patch.object(ScoutDog, "_sync_fetch", return_value=empty_r):
            j = await dog.analyze(make_url_cell(["https://empty.example.com"]))
        assert any("empty-content" in v for v in j.evidence["violations"])

    @pytest.mark.asyncio
    async def test_max_urls_per_cell_respected(self):
        """Only MAX_URLS_PER_CELL (3) URLs checked even if cell has more."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult, MAX_URLS_PER_CELL
        dog = ScoutDog()
        good = FetchResult(url="x", status=200, content_len=500, latency_ms=30)
        call_count = 0

        def counting_fetch(url):
            nonlocal call_count
            call_count += 1
            return FetchResult(url=url, status=200, content_len=500, latency_ms=30)

        many_urls = [f"https://example{i}.com" for i in range(5)]
        with patch.object(ScoutDog, "_sync_fetch", side_effect=counting_fetch):
            await dog.analyze(make_url_cell(many_urls))
        assert call_count <= MAX_URLS_PER_CELL

    @pytest.mark.asyncio
    async def test_confidence_is_phi_inv2(self):
        """Heuristic path uses SCOUT_CONFIDENCE = PHI_INV_2 (0.382)."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult, SCOUT_CONFIDENCE
        dog = ScoutDog()
        good = FetchResult(url="https://x.com", status=200, content_len=500, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            j = await dog.analyze(make_url_cell(["https://x.com"]))
        assert abs(j.confidence - SCOUT_CONFIDENCE) < 0.001

    @pytest.mark.asyncio
    async def test_evidence_structure(self):
        """Evidence has expected keys."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=256, latency_ms=50)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            j = await dog.analyze(make_url_cell(["https://example.com"]))
        e = j.evidence
        assert "urls_checked" in e
        assert "ok_count" in e
        assert "miss_count" in e
        assert "timeout_count" in e
        assert "error_count" in e
        assert "avg_latency_ms" in e
        assert "results" in e
        assert "violations" in e

    @pytest.mark.asyncio
    async def test_counters_tracked(self):
        """ScoutDog tracks internal URL stats."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=500, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            await dog.analyze(make_url_cell(["https://example.com"]))
        assert dog._urls_checked == 1
        assert dog._hits == 1
        assert dog._misses == 0

    @pytest.mark.asyncio
    async def test_miss_counter_tracked(self):
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        miss = FetchResult(url="https://dead.example.com", status=404, content_len=0, latency_ms=20)
        with patch.object(ScoutDog, "_sync_fetch", return_value=miss):
            await dog.analyze(make_url_cell(["https://dead.example.com"]))
        assert dog._misses == 1
        assert dog._hits == 0


# ════════════════════════════════════════════════════════════════════════════
# Temporal MCTS Path (LLM)
# ════════════════════════════════════════════════════════════════════════════

class TestTemporalPath:

    @pytest.mark.asyncio
    async def test_temporal_path_activated_by_registry(self):
        """With LLM registry injected, temporal path is used (llm_id set)."""
        from cynic.cognition.neurons.scout import ScoutDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(48.0))
        j = await dog.analyze(make_cell("Check https://example.com for details"))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.SCOUT
        assert j.llm_id == "ollama:llama3.2"
        assert 0.0 < j.q_score <= MAX_Q_SCORE

    @pytest.mark.asyncio
    async def test_temporal_path_evidence_has_path(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell("Analysis content"))
        assert j.evidence.get("path") == "temporal_mcts"

    @pytest.mark.asyncio
    async def test_temporal_path_confidence_bounded(self):
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(55.0))
        j = await dog.analyze(make_cell("Content"))
        assert 0.0 < j.confidence <= PHI_INV

    @pytest.mark.asyncio
    async def test_temporal_path_no_veto(self):
        """Scout never vetoes even in temporal path."""
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(10.0))  # Very low score
        j = await dog.analyze(make_cell("Bad content"))
        assert j.veto is False

    @pytest.mark.asyncio
    async def test_temporal_skips_url_fetch(self):
        """Temporal path does NOT call _sync_fetch (uses LLM instead)."""
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(50.0))
        with patch.object(ScoutDog, "_sync_fetch") as mock_fetch:
            await dog.analyze(make_url_cell(["https://example.com"]))
            mock_fetch.assert_not_called()

    @pytest.mark.asyncio
    async def test_temporal_path_with_no_urls_still_works(self):
        """Temporal path works even with no URLs in content."""
        from cynic.cognition.neurons.scout import ScoutDog
        dog = ScoutDog()
        dog.set_llm_registry(_mock_registry(42.0))
        j = await dog.analyze(make_cell("No links here, just plain code."))
        assert j.llm_id == "ollama:llama3.2"


# ════════════════════════════════════════════════════════════════════════════
# Health Check
# ════════════════════════════════════════════════════════════════════════════

class TestScoutHealth:

    @pytest.mark.asyncio
    async def test_health_unknown_when_no_urls_checked(self):
        from cynic.cognition.neurons.scout import ScoutDog
        from cynic.cognition.neurons.base import HealthStatus
        dog = ScoutDog()
        health = await dog.health_check()
        assert health.dog_id == DogId.SCOUT
        assert health.status == HealthStatus.UNKNOWN

    @pytest.mark.asyncio
    async def test_health_healthy_when_hit_rate_above_phi_inv2(self):
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        from cynic.cognition.neurons.base import HealthStatus
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=500, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            await dog.analyze(make_url_cell(["https://example.com"]))
        health = await dog.health_check()
        # 1 hit / 1 total = 100% > PHI_INV_2 → HEALTHY
        assert health.status == HealthStatus.HEALTHY

    @pytest.mark.asyncio
    async def test_health_degraded_when_all_misses(self):
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        from cynic.cognition.neurons.base import HealthStatus
        dog = ScoutDog()
        miss = FetchResult(url="https://dead.example.com", status=404, content_len=0, latency_ms=20)
        with patch.object(ScoutDog, "_sync_fetch", return_value=miss):
            await dog.analyze(make_url_cell(["https://dead.example.com"]))
        health = await dog.health_check()
        # 0 hits / 1 total = 0% < PHI_INV_2 → DEGRADED
        assert health.status == HealthStatus.DEGRADED

    @pytest.mark.asyncio
    async def test_health_details_string(self):
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=500, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            await dog.analyze(make_url_cell(["https://example.com"]))
        health = await dog.health_check()
        assert "URLs checked" in health.details
        assert "Hit rate" in health.details


# ════════════════════════════════════════════════════════════════════════════
# φ-Alignment
# ════════════════════════════════════════════════════════════════════════════

class TestPhiAlignment:

    def test_max_urls_is_fibonacci_4(self):
        from cynic.cognition.neurons.scout import MAX_URLS_PER_CELL
        from cynic.core.phi import fibonacci
        assert MAX_URLS_PER_CELL == fibonacci(4)  # 3

    def test_timeout_is_fibonacci_3(self):
        from cynic.cognition.neurons.scout import TIMEOUT_SEC
        from cynic.core.phi import fibonacci
        assert TIMEOUT_SEC == fibonacci(3)  # 2

    def test_min_content_is_fibonacci_8(self):
        from cynic.cognition.neurons.scout import MIN_CONTENT_LEN
        from cynic.core.phi import fibonacci
        assert MIN_CONTENT_LEN == fibonacci(8)  # 21

    def test_scout_confidence_is_phi_inv2(self):
        from cynic.cognition.neurons.scout import SCOUT_CONFIDENCE
        assert abs(SCOUT_CONFIDENCE - PHI_INV_2) < 1e-6

    def test_q_score_bounded(self):
        """Q score never exceeds MAX_Q_SCORE."""
        from cynic.cognition.neurons.scout import ScoutDog, FetchResult
        import asyncio
        dog = ScoutDog()
        good = FetchResult(url="https://example.com", status=200, content_len=1024, latency_ms=30)
        with patch.object(ScoutDog, "_sync_fetch", return_value=good):
            j = asyncio.get_event_loop().run_until_complete(
                dog.analyze(make_url_cell(["https://example.com"]))
            )
        assert j.q_score <= MAX_Q_SCORE
