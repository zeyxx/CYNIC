"""
CYNIC AbstractDog â€” Base interface for all 11 Dogs (Sefirot)

Every Dog MUST implement:
  - analyze(cell) â†’ DogJudgment
  - get_capabilities() â†’ DogCapabilities
  - health_check() â†’ HealthStatus

Dogs are categorized by consciousness level:
  L3 REFLEX (non-LLM):  CYNIC-PBFT, GUARDIAN, ANALYST, JANITOR
  L2/L1 (LLM-capable):  SAGE, SCHOLAR, ORACLE, ARCHITECT, DEPLOYER, SCOUT, CARTOGRAPHER

E-Score weights and Dog priority:
  CYNIC (Keter)      â€” Ï†Â³ priority (consensus coordinator)
  SAGE (Chokmah)     â€” Ï†Â² priority (wisdom, knowledge graph)
  ANALYST (Binah)    â€” Ï†Â² priority (formal verification)
  GUARDIAN (Gevurah) â€” Ï† priority (security, anomaly)
  ORACLE (Tiferet)   â€” Ï† priority (MCTS, Thompson)
  ARCHITECT (Netzach)â€” 1.0 priority (code structure)
  CARTOGRAPHER (Daat)â€” 1.0 priority (graph, topology)
  SCHOLAR (Chesed)   â€” Ï†â»Â¹ priority (vector RAG)
  DEPLOYER (Hod)     â€” Ï†â»Â¹ priority (execution)
  SCOUT (Malkuth)    â€” Ï†â»Â² priority (web, discovery)
  JANITOR (Yesod)    â€” Ï†â»Â² priority (cleanup, linting)
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field

# Python 3.9 compatibility: StrEnum added in Python 3.11
from enum import StrEnum
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRegistry

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import (
    MAX_CONFIDENCE,
    PHI,
    PHI_2,
    PHI_3,
    PHI_INV,
    PHI_INV_2,
    phi_bound_score,
)

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# DOG REGISTRY (all 11 Dogs with their Sefirot)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class DogId(StrEnum):
    """The 11 Dogs â€” Sefirot of the Kabbalistic Tree of Life."""

    CYNIC = "CYNIC"  # Keter â€” Crown (PBFT coordinator)
    SAGE = "SAGE"  # Chokmah â€” Wisdom (LLM + RDFLib)
    ANALYST = "ANALYST"  # Binah â€” Understanding (Z3)
    GUARDIAN = "GUARDIAN"  # Gevurah â€” Strength (IsolationForest)
    ORACLE = "ORACLE"  # Tiferet â€” Beauty (MCTS + Thompson)
    ARCHITECT = "ARCHITECT"  # Netzach â€” Victory (TreeSitter)
    CARTOGRAPHER = "CARTOGRAPHER"  # Daat â€” Knowledge (NetworkX)
    SCHOLAR = "SCHOLAR"  # Chesed â€” Kindness (Qdrant RAG)
    DEPLOYER = "DEPLOYER"  # Hod â€” Splendor (Ansible/K8s)
    SCOUT = "SCOUT"  # Malkuth â€” Kingdom (Scrapy)
    JANITOR = "JANITOR"  # Yesod â€” Foundation (Ruff AST)


# Ï†-symmetric priority weights per Dog
DOG_PRIORITY: dict[str, float] = {
    DogId.CYNIC: PHI_3,  # Ï†Â³ = 4.236 â€” highest, consensus critical
    DogId.SAGE: PHI_2,  # Ï†Â² = 2.618
    DogId.ANALYST: PHI_2,  # Ï†Â² = 2.618
    DogId.GUARDIAN: PHI,  # Ï†  = 1.618
    DogId.ORACLE: PHI,  # Ï†  = 1.618
    DogId.ARCHITECT: 1.0,  # Ï†â° = 1.000
    DogId.CARTOGRAPHER: 1.0,  # Ï†â° = 1.000
    DogId.SCHOLAR: PHI_INV,  # Ï†â»Â¹ = 0.618
    DogId.DEPLOYER: PHI_INV,  # Ï†â»Â¹ = 0.618
    DogId.SCOUT: PHI_INV_2,  # Ï†â»Â² = 0.382
    DogId.JANITOR: PHI_INV_2,  # Ï†â»Â² = 0.382
}

# Non-LLM Dogs (L3 REFLEX capable)
NON_LLM_DOGS: set[str] = {
    DogId.CYNIC,
    DogId.GUARDIAN,
    DogId.ANALYST,
    DogId.ARCHITECT,
    DogId.ORACLE,
    DogId.JANITOR,
}


from pydantic import BaseModel, ConfigDict, Field, field_validator

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# DOG JUDGMENT OUTPUT
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class DogJudgment(BaseModel):
    """
    A single Dog's judgment of a Cell (Pydantic V2).

    Dogs vote with q_score + confidence.
    PBFT aggregates dog_judgments into ConsensusResult.
    
    Supports fractal growth: extra fields are allowed but validated.
    """
    model_config = ConfigDict(extra="allow", frozen=False)

    dog_id: str
    cell_id: str
    q_score: float  # [0, 100]
    confidence: float  # [0, 0.618]
    reasoning: str = ""  # Human-readable explanation
    evidence: dict[str, Any] = Field(default_factory=dict)  # Supporting data
    latency_ms: float = 0.0
    cost_usd: float = 0.0
    llm_id: str | None = None  # Which LLM was used (None for non-LLM Dogs)
    timestamp: float = Field(default_factory=time.time)
    veto: bool = False  # GUARDIAN can veto (blocks execution regardless of votes)

    @field_validator("q_score")
    @classmethod
    def bound_q_score(cls, v: float) -> float:
        return max(0.0, min(v, 100.0))

    @field_validator("confidence")
    @classmethod
    def bound_confidence(cls, v: float) -> float:
        return max(0.0, min(v, MAX_CONFIDENCE))

    @property
    def vote_weight(self) -> float:
        """Confidence-weighted vote for PBFT aggregation."""
        return self.confidence * DOG_PRIORITY.get(self.dog_id, 1.0)

    def to_dict(self) -> dict[str, Any]:
        """Legacy compatibility wrapper."""
        return self.model_dump()


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# DOG CAPABILITIES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@dataclass
class DogCapabilities:
    """
    What a Dog can do â€” used by MCTS to select Dog combinations.

    Capabilities determine which Cells a Dog can usefully analyze.
    MCTS Level 1 selects Dog subsets based on combined capability coverage.
    """

    dog_id: str
    sefirot: str  # Kabbalistic name
    consciousness_min: ConsciousnessLevel  # Minimum level to activate
    uses_llm: bool
    supported_realities: set[str]  # Which reality dimensions
    supported_analyses: set[str]  # Which analysis types
    technology: str  # Primary tech (Z3, IsolationForest, etc.)
    max_concurrent: int = 1  # How many parallel instances allowed

    @property
    def can_analyze(self, cell: Cell) -> bool:
        """Check if this Dog can analyze a given Cell."""
        return cell.reality in self.supported_realities and cell.analysis in self.supported_analyses


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# HEALTH STATUS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class HealthStatus(StrEnum):
    HEALTHY = "HEALTHY"
    DEGRADED = "DEGRADED"
    UNHEALTHY = "UNHEALTHY"
    UNKNOWN = "UNKNOWN"


@dataclass
class DogHealth:
    dog_id: str
    status: HealthStatus = HealthStatus.UNKNOWN
    latency_p50_ms: float = 0.0
    latency_p95_ms: float = 0.0
    error_rate: float = 0.0
    last_check: float = field(default_factory=time.time)
    details: str = ""

    @property
    def is_healthy(self) -> bool:
        return self.status == HealthStatus.HEALTHY


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# ABSTRACT DOG
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class AbstractDog(ABC):
    """
    Base class for all 11 CYNIC Dogs.

    Subclasses MUST implement:
      - analyze(cell) â†’ DogJudgment
      - get_capabilities() â†’ DogCapabilities
      - health_check() â†’ DogHealth

    The organism is the sum of all Dogs, coordinated by PBFT.
    """

    def __init__(self, dog_id: str, bus: Optional[EventBus] = None) -> None:
        self.dog_id = dog_id
        self._judgment_count = 0
        self._error_count = 0
        self._total_latency_ms = 0.0
        self._active = False
        from cynic.kernel.core.event_bus import CoreEvent, Event
        self.bus = bus or get_core_bus("DEFAULT")

    @abstractmethod
    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Judge a Cell. Returns DogJudgment with Ï†-bounded q_score.

        kwargs may include: budget_usd, llm_registry, context_hint
        """
        ...

    @abstractmethod
    def get_capabilities(self) -> DogCapabilities:
        """Return this Dog's capabilities (used by MCTS Level 1)."""
        ...

    @abstractmethod
    async def health_check(self) -> DogHealth:
        """Check if this Dog is operational."""
        ...

    async def start(self) -> None:
        """Initialize Dog (load models, connect to services). Called once."""
        self._active = True

    async def stop(self) -> None:
        """Graceful shutdown."""
        self._active = False

    def record_judgment(self, judgment: DogJudgment) -> None:
        """Track stats for health monitoring."""
        self._judgment_count += 1
        self._total_latency_ms += judgment.latency_ms

    def record_error(self) -> None:
        self._error_count += 1

    @property
    def avg_latency_ms(self) -> float:
        if self._judgment_count == 0:
            return 0.0
        return self._total_latency_ms / self._judgment_count

    @property
    def error_rate(self) -> float:
        total = self._judgment_count + self._error_count
        if total == 0:
            return 0.0
        return self._error_count / total

    @property
    def priority(self) -> float:
        """Ï†-weighted priority for DogScheduler."""
        return DOG_PRIORITY.get(self.dog_id, 1.0)

    def stats(self) -> dict[str, Any]:
        return {
            "dog_id": self.dog_id,
            "active": self._active,
            "judgments": self._judgment_count,
            "errors": self._error_count,
            "error_rate": round(self.error_rate, 3),
            "avg_latency_ms": round(self.avg_latency_ms, 1),
            "priority": self.priority,
        }


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# LLM DOG BASE (extends AbstractDog with LLM routing)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class LLMDog(AbstractDog):
    """
    Base class for the 7 LLM-capable Dogs.

    Provides:
      - LLM routing via LLMRegistry (best model for this Dog Ã— task_type)
      - Automatic benchmark recording after each judgment
      - Graceful degradation: if LLM unavailable â†’ GROWL verdict, low confidence
    """

    def __init__(self, dog_id: str, task_type: str = "general", bus: Optional[EventBus] = None) -> None:
        super().__init__(dog_id, bus=bus)
        self.task_type = task_type
        self._llm_registry: LLMRegistry | None = None
        self._llm_id: str | None = None

    def set_llm_registry(self, registry: LLMRegistry) -> None:
        """Inject LLMRegistry (dependency injection, no circular imports)."""
        self._llm_registry = registry

    async def get_llm(self) -> LLMAdapter | None:
        """Get best LLM for this Dog's task type."""
        if self._llm_registry is None:
            return None
        return self._llm_registry.get_best_for(self.dog_id, self.task_type)

    async def complete(
        self,
        prompt: str,
        system: str = "",
        budget_usd: float = 0.1,
    ) -> tuple[str, str, float]:
        """
        Complete a prompt using the best available LLM.

        Returns: (response_text, llm_id, cost_usd)
        Raises: RuntimeError if no LLM available.
        """
        adapter = await self.get_llm()
        if adapter is None:
            raise RuntimeError(f"No LLM available for Dog {self.dog_id}")

        from cynic.kernel.organism.brain.llm.adapter import LLMRequest

        req = LLMRequest(
            prompt=prompt,
            system=system,
            max_tokens=min(2048, int(budget_usd * 10000)),
            temperature=PHI_INV_2,  # 0.382 â€” slightly creative but grounded
        )
        resp = await adapter.complete(req)
        return resp.content, adapter.llm_id, resp.cost_usd

    def record_judgment(self, judgment: DogJudgment) -> None:
        """Track latency stats + update LLM benchmark registry."""
        super().record_judgment(judgment)
        if judgment.llm_id is not None and self._llm_registry is not None:
            self._record_benchmark(judgment)

    def _record_benchmark(self, judgment: DogJudgment) -> None:
        """
        Feed judgment outcome back into LLMRegistry routing table.

        Converts DogJudgment into a BenchmarkResult and calls
        registry.update_benchmark() â†’ EMA update â†’ better routing next time.

        Speed target: 3000ms (L1 MACRO budget â€” longer is penalized)
        Cost budget:  $0.01 per judgment (Ollama = free = 1.0 score)
        """
        from cynic.kernel.organism.brain.llm.adapter import BenchmarkResult

        # Normalize speed: 0ms â†’ 1.0, 3000ms â†’ 0.0, beyond â†’ capped at 0
        speed_score = max(0.0, 1.0 - judgment.latency_ms / _SPEED_TARGET_MS)

        # Normalize cost: free (Ollama) â†’ 1.0, over budget â†’ 0.0
        if judgment.cost_usd <= 0.0:
            cost_score = 1.0  # Local inference is free
        else:
            cost_score = max(0.0, 1.0 - judgment.cost_usd / _COST_BUDGET_USD)

        result = BenchmarkResult(
            llm_id=judgment.llm_id,
            dog_id=self.dog_id,
            task_type=self.task_type,
            quality_score=judgment.q_score,  # [0, 61.8] â€” Ï†-bounded
            speed_score=speed_score,  # [0, 1]
            cost_score=cost_score,  # [0, 1]
            error_rate=0.0,
        )
        self._llm_registry.update_benchmark(
            dog_id=self.dog_id,
            task_type=self.task_type,
            llm_id=judgment.llm_id,
            result=result,
        )


# â”€â”€ Benchmark normalisation constants â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
_SPEED_TARGET_MS: float = 3000.0  # L1 MACRO target â€” 3s budget per call
_COST_BUDGET_USD: float = 0.01  # $0.01 per judgment â€” Ollama = 0 â†’ 1.0
